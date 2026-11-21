use super::AudioError;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use tokio::{sync::oneshot, time::timeout};

/// Hash of an authenticated playback credential, its protocol and its media item.
pub type PlaybackKey = [u8; 32];

#[derive(Clone, Copy)]
pub struct CueRequest {
    pub key: PlaybackKey,
    pub range: bool,
}

#[derive(Default)]
pub(super) struct Slots {
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    active: Vec<Option<PlaybackKey>>,
    waiting: HashMap<PlaybackKey, (usize, oneshot::Sender<Slot>)>,
}
pub(super) struct Slot {
    pool: Option<Arc<Slots>>,
    key: Option<PlaybackKey>,
    pub(super) count: usize,
}
impl Slots {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub(super) async fn acquire(
        self: &Arc<Self>,
        limit: usize,
        cue: Option<CueRequest>,
    ) -> Result<Slot, AudioError> {
        let pending = {
            let mut state = self.state();
            let key = cue.map(|request| request.key);
            if state.active.len() < limit {
                state.active.push(key);
                return Ok(Slot {
                    pool: Some(self.clone()),
                    key,
                    count: state.active.len(),
                });
            }
            let Some(request) = cue.filter(|r| r.range) else {
                return Err(AudioError::Busy);
            };
            if !state.active.contains(&key)
                || state
                    .waiting
                    .get(&request.key)
                    .is_some_and(|(_, tx)| !tx.is_closed())
            {
                return Err(AudioError::Busy);
            }
            let (tx, rx) = oneshot::channel();
            state.waiting.insert(request.key, (limit, tx));
            rx
        };
        // A browser cancelling its old range can issue the new one before FFmpeg exits.
        // Wait only for that playback, with at most one parked request per active key.
        timeout(Duration::from_secs(1), pending)
            .await
            .map_err(|_| AudioError::Busy)?
            .map_err(|_| AudioError::Busy)
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        let Some(pool) = self.pool.take() else { return };
        let mut state = pool.state();
        if let Some(key) = self.key
            && let Some((limit, tx)) = state.waiting.remove(&key)
            && state.active.len() <= limit
        {
            let next = Slot {
                pool: Some(pool.clone()),
                key: self.key,
                count: state.active.len(),
            };
            match tx.send(next) {
                Ok(()) => return,
                // The waiting HTTP request was cancelled or expired. Release the existing
                // slot below without running the undelivered grant's destructor twice.
                Err(mut unused) => {
                    unused.pool.take();
                }
            }
        }
        if let Some(i) = state.active.iter().position(|key| *key == self.key) {
            state.active.swap_remove(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pending<F: std::future::Future>(future: std::pin::Pin<&mut F>) -> bool {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        future.poll(&mut context).is_pending()
    }
    fn cue(key: u8, range: bool) -> Option<CueRequest> {
        Some(CueRequest {
            key: [key; 32],
            range,
        })
    }

    #[tokio::test]
    async fn a_matching_range_waits_for_exit_without_stealing_a_different_playback() {
        let pool = Arc::new(Slots::default());
        let original = pool.acquire(1, cue(1, false)).await.unwrap();
        let next = pool.acquire(1, cue(1, true));
        tokio::pin!(next);
        assert!(pending(next.as_mut()));
        assert!(matches!(
            pool.acquire(1, cue(1, true)).await,
            Err(AudioError::Busy)
        ));
        assert!(matches!(
            pool.acquire(1, cue(2, true)).await,
            Err(AudioError::Busy)
        ));
        assert!(matches!(
            pool.acquire(1, cue(1, false)).await,
            Err(AudioError::Busy)
        ));
        assert!(matches!(pool.acquire(1, None).await, Err(AudioError::Busy)));
        drop(original);
        assert!(matches!(pool.acquire(1, None).await, Err(AudioError::Busy)));
        let resumed = next.await.unwrap();
        assert_eq!(resumed.count, 1);
        drop(resumed);
        assert!(pool.acquire(1, None).await.is_ok());
    }

    #[tokio::test]
    async fn an_unreleased_request_expires_and_a_dropped_waiter_does_not_consume_capacity() {
        let pool = Arc::new(Slots::default());
        let original = pool.acquire(1, cue(1, false)).await.unwrap();
        assert!(matches!(
            pool.acquire(1, cue(1, true)).await,
            Err(AudioError::Busy)
        ));
        let mut abandoned = Box::pin(pool.acquire(1, cue(1, true)));
        assert!(pending(abandoned.as_mut()));
        drop(abandoned);
        drop(original);
        assert!(pool.acquire(1, None).await.is_ok());
    }
}
