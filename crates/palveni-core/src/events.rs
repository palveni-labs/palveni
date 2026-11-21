//! Numbered, bounded state-event history shared by live and reconnecting clients.

use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::sync::{broadcast, watch};

use crate::PerfTier;
use crate::ids::LibraryId;
use crate::model::{Job, Library};

/// Events delivered to clients over `GET /api/v1/events`.
#[derive(Clone, Debug)]
pub enum ServerEvent {
    /// A job changed state or progress. Admin-only.
    JobUpdated(Job),
    /// A library was created or changed (including item count after a scan).
    LibraryUpdated(Library),
    SettingsUpdated {
        server_name: Option<String>,
        perf_tier: Option<PerfTier>,
    },
    /// A library was deleted.
    LibraryDeleted { id: LibraryId },
}

impl ServerEvent {
    /// SSE `event:` name from the contract.
    pub const fn name(&self) -> &'static str {
        match self {
            ServerEvent::JobUpdated(_) => "job.updated",
            ServerEvent::SettingsUpdated { .. } => "settings.updated",
            ServerEvent::LibraryUpdated(_) => "library.updated",
            ServerEvent::LibraryDeleted { .. } => "library.deleted",
        }
    }

    pub const fn admin_only(&self) -> bool {
        matches!(self, Self::JobUpdated(_) | Self::SettingsUpdated { .. })
    }

    /// SSE `data:` payload (single-line JSON).
    pub fn data_json(&self) -> serde_json::Result<String> {
        match self {
            ServerEvent::JobUpdated(job) => serde_json::to_string(job),
            ServerEvent::SettingsUpdated {
                server_name,
                perf_tier,
            } => {
                serde_json::to_string(&json!({ "serverName": server_name, "perfTier": perf_tier }))
            }
            ServerEvent::LibraryUpdated(library) => serde_json::to_string(library),
            ServerEvent::LibraryDeleted { id } => serde_json::to_string(&json!({ "id": id })),
        }
    }
}

#[derive(Debug)]
pub struct NumberedEvent {
    pub revision: i64,
    pub event: ServerEvent,
}

struct History {
    // A cursor at this boundary can replay everything after it.
    floor: i64,
    entries: VecDeque<Arc<NumberedEvent>>,
    capacity: usize,
}

impl History {
    fn latest(&self) -> i64 {
        self.entries
            .back()
            .map_or(self.floor, |event| event.revision)
    }

    fn append(&mut self, revision: i64, event: ServerEvent) {
        if self.entries.len() == self.capacity
            && let Some(old) = self.entries.pop_front()
        {
            self.floor = old.revision;
        }
        self.entries
            .push_back(Arc::new(NumberedEvent { revision, event }));
    }

    fn after(&self, cursor: Option<i64>) -> Replay {
        let latest = self.latest();
        let cursor = cursor.unwrap_or(latest);
        if cursor < self.floor || cursor > latest {
            return Replay {
                latest,
                reset: true,
                entries: VecDeque::new(),
            };
        }
        let mut entries = VecDeque::new();
        for event in self
            .entries
            .iter()
            .rev()
            .take_while(|event| event.revision > cursor)
        {
            entries.push_front(Arc::clone(event));
        }
        Replay {
            latest,
            reset: false,
            entries,
        }
    }
}

pub struct Replay {
    pub latest: i64,
    pub reset: bool,
    pub entries: VecDeque<Arc<NumberedEvent>>,
}

pub struct Subscription(watch::Receiver<History>);

impl Subscription {
    /// Takes the backlog and marks exactly this snapshot as observed. A publication
    /// racing with the snapshot is either included or wakes the next changed().
    pub fn after(&mut self, cursor: Option<i64>) -> Replay {
        self.0.borrow_and_update().after(cursor)
    }

    pub async fn changed(&mut self) -> bool {
        self.0.changed().await.is_ok()
    }
}

#[derive(Clone)]
pub struct EventBus {
    pending: broadcast::Sender<ServerEvent>,
    history: watch::Receiver<History>,
}

impl EventBus {
    /// The initial boundary and all subsequent IDs come from the durable sync counter.
    /// One writer orders allocations; progress callbacks can publish synchronously.
    pub fn new<F, Fut, E>(capacity: usize, initial: i64, allocate: F) -> Self
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = Result<i64, E>> + Send,
        E: fmt::Display + Send,
    {
        let capacity = capacity.max(1);
        let (pending, mut input) = broadcast::channel(capacity);
        let (journal, history) = watch::channel(History {
            floor: initial,
            entries: VecDeque::with_capacity(capacity),
            capacity,
        });
        tokio::spawn(async move {
            loop {
                let event = match input.recv().await {
                    Ok(event) => Some(event),
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(skipped, "event writer lagged; clients must refresh");
                        None
                    }
                };
                // Keep the pending event on transient DB failures. A full input ring
                // generates a fresh reset boundary before we resume retained events.
                let revision = loop {
                    match allocate().await {
                        Ok(revision) => break revision,
                        Err(err) => tracing::warn!(error = %err, "cannot allocate event revision"),
                    }
                    tokio::select! {
                        () = journal.closed() => return,
                        () = tokio::time::sleep(Duration::from_secs(1)) => {}
                    }
                };
                journal.send_modify(|window| match event {
                    Some(event) => window.append(revision, event),
                    None => {
                        window.floor = revision;
                        window.entries.clear();
                    }
                });
            }
        });
        Self { pending, history }
    }

    pub fn publish(&self, event: ServerEvent) {
        let _ = self.pending.send(event);
    }

    pub fn subscribe(&self) -> Subscription {
        Subscription(self.history.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library_deleted() -> ServerEvent {
        ServerEvent::LibraryDeleted {
            id: LibraryId::generate(),
        }
    }

    #[test]
    fn retained_window_handles_gaps_eviction_and_restart() {
        let mut window = History {
            floor: 41,
            entries: VecDeque::new(),
            capacity: 2,
        };
        assert!(window.after(Some(40)).reset);
        window.append(44, library_deleted());
        window.append(50, library_deleted());
        window.append(57, library_deleted());
        assert!(window.after(Some(43)).reset);
        assert!(window.after(Some(58)).reset);
        let ids: Vec<_> = window
            .after(Some(44))
            .entries
            .iter()
            .map(|event| event.revision)
            .collect();
        assert_eq!(ids, [50, 57]);
        assert_eq!(window.after(Some(51)).entries.front().unwrap().revision, 57);
        assert!(window.after(None).entries.is_empty());
        assert!(!window.after(Some(57)).reset);
    }

    #[tokio::test]
    async fn replay_and_live_handoff_keep_events_without_subscribers() {
        use std::sync::atomic::{AtomicI64, Ordering};
        let next = AtomicI64::new(10);
        let bus = EventBus::new(4, 9, move || {
            std::future::ready(Ok::<_, std::convert::Infallible>(
                next.fetch_add(1, Ordering::Relaxed),
            ))
        });
        bus.publish(library_deleted());
        let mut stream = bus.subscribe();
        while stream.after(Some(9)).entries.is_empty() {
            assert!(stream.changed().await);
        }
        let first = stream.after(Some(9));
        bus.publish(library_deleted());
        assert!(stream.changed().await);
        let next = stream.after(Some(first.latest));
        assert_eq!(next.entries.len(), 1);
        assert_eq!(next.entries[0].revision, 11);
    }

    #[tokio::test]
    async fn overloaded_publisher_marks_gap_then_retains_remaining_events() {
        use std::sync::atomic::{AtomicI64, Ordering};
        let next = AtomicI64::new(11);
        let bus = EventBus::new(2, 10, move || {
            std::future::ready(Ok::<_, std::convert::Infallible>(
                next.fetch_add(1, Ordering::Relaxed),
            ))
        });
        let mut subscriber = bus.subscribe();
        for _ in 0..3 {
            bus.publish(library_deleted());
        }
        while subscriber.after(None).latest < 13 {
            assert!(subscriber.changed().await);
        }
        assert!(subscriber.after(Some(10)).reset);
        let recovered = subscriber.after(Some(11));
        assert!(!recovered.reset);
        assert_eq!(recovered.entries.len(), 2);
    }

    #[test]
    fn settings_are_admin_only_and_use_the_settings_payload() {
        let event = ServerEvent::SettingsUpdated {
            server_name: None,
            perf_tier: Some(PerfTier::Lite),
        };
        assert!(event.admin_only());
        assert_eq!(event.name(), "settings.updated");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&event.data_json().unwrap()).unwrap(),
            json!({"serverName":null,"perfTier":"lite"})
        );
    }
}
