//! Authenticated, resumable server-sent events.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use futures_util::stream::{self, Stream};
use palveni_core::events::{NumberedEvent, Replay, Subscription};
use tokio::time::{Instant, Interval, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::state::AppState;

const PING_INTERVAL: Duration = Duration::from_secs(20);

pub async fn stream(
    current: CurrentUser,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let after = cursor(&headers)?;
    let events = event_stream(
        state.bus.subscribe(),
        after,
        current.is_admin(),
        state.shutdown.clone(),
    );
    Ok(([("x-accel-buffering", "no")], Sse::new(events)))
}

fn cursor(headers: &HeaderMap) -> Result<Option<i64>, ApiError> {
    let invalid = || ApiError::validation("Last-Event-ID must be a nonnegative decimal revision");
    let mut ids = headers.get_all("last-event-id").iter();
    let Some(header) = ids.next() else {
        return Ok(None);
    };
    let s = header.to_str().map_err(|_| invalid())?;
    if ids.next().is_some() || s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    Ok(Some(s.parse().map_err(|_| invalid())?))
}

struct StreamState {
    subscription: Subscription,
    batch: Replay,
    initial_ping: bool,
    ping: Interval,
    is_admin: bool,
    shutdown: CancellationToken,
}

fn event_stream(
    mut subscription: Subscription,
    after: Option<i64>,
    is_admin: bool,
    shutdown: CancellationToken,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    let mut ping = tokio::time::interval_at(Instant::now() + PING_INTERVAL, PING_INTERVAL);
    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let initial = StreamState {
        batch: subscription.after(after),
        subscription,
        initial_ping: true,
        ping,
        is_admin,
        shutdown,
    };
    stream::unfold(initial, |mut st| async move {
        loop {
            if st.shutdown.is_cancelled() {
                return None;
            }
            if std::mem::take(&mut st.initial_ping) {
                return Some((Ok(Event::default().event("ping").data("{}")), st));
            }
            if std::mem::take(&mut st.batch.reset) {
                let reset = Event::default()
                    .event("stream.reset")
                    .id(st.batch.latest.to_string())
                    .data("{}");
                return Some((Ok(reset), st));
            }
            while let Some(event) = st.batch.entries.pop_front() {
                if let Some(sse) = to_sse(&event, st.is_admin) {
                    return Some((Ok(sse), st));
                }
            }
            tokio::select! {
                _ = st.shutdown.cancelled() => return None,
                _ = st.ping.tick() => return Some((Ok(Event::default().event("ping").data("{}")), st)),
                changed = st.subscription.changed() => {
                    if !changed { return None; }
                    st.batch = st.subscription.after(Some(st.batch.latest));
                }
            }
        }
    })
}

fn to_sse(numbered: &NumberedEvent, is_admin: bool) -> Option<Event> {
    let event = &numbered.event;
    if event.admin_only() && !is_admin {
        return None;
    }
    match event.data_json() {
        Ok(data) => Some(
            Event::default()
                .event(event.name())
                .id(numbered.revision.to_string())
                .data(data),
        ),
        Err(err) => {
            tracing::warn!(event = event.name(), error = %err, "cannot encode event");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use palveni_core::ServerEvent;

    #[test]
    fn cursor_is_one_nonnegative_decimal() {
        let mut headers = HeaderMap::new();
        assert_eq!(cursor(&headers).unwrap(), None);
        for invalid in [
            "",
            "-1",
            "+1",
            "0.25",
            " 9",
            "3,4",
            "999999999999999999999999",
        ] {
            headers.insert("last-event-id", HeaderValue::from_str(invalid).unwrap());
            assert!(cursor(&headers).is_err());
        }
        headers.insert(
            "last-event-id",
            HeaderValue::from_static("9223372036854775807"),
        );
        assert_eq!(cursor(&headers).unwrap(), Some(i64::MAX));
        headers.append("last-event-id", HeaderValue::from_static("1"));
        assert!(cursor(&headers).is_err());
    }

    #[test]
    fn nonadmins_do_not_receive_private_events() {
        let event = NumberedEvent {
            revision: 12,
            event: ServerEvent::SettingsUpdated {
                server_name: None,
                perf_tier: None,
            },
        };
        assert!(to_sse(&event, false).is_none());
        assert!(to_sse(&event, true).is_some());
    }
}
