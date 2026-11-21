use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasher, RandomState};
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use palveni_db::users::username_key;
use tokio::sync::futures::OwnedNotified;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};

use crate::ApiError;

const WINDOW: Duration = Duration::from_secs(60);
const MAX_FAILURES: usize = 8;
const MAX_PEER_FAILURES: usize = 64;
const MAX_ADDRESSES: usize = 4096;
const OVERFLOW_GROUPS: u64 = 64;

#[derive(Default)]
struct Attempts {
    failed: VecDeque<(Instant, u64)>,
    pending: Vec<u64>,
    changed: Arc<Notify>,
}

impl Attempts {
    fn expire(&mut self, now: Instant) {
        while self
            .failed
            .front()
            .is_some_and(|(at, _)| now.duration_since(*at) >= WINDOW)
        {
            self.failed.pop_front();
        }
    }

    fn is_empty(&self) -> bool {
        self.failed.is_empty() && self.pending.is_empty()
    }

    /// Returns the signal to wait for when outstanding checks hold the remaining attempts.
    fn reserve(
        &mut self,
        account: u64,
        now: Instant,
    ) -> Result<Option<(OwnedNotified, Instant)>, ApiError> {
        self.expire(now);
        let mut failures = self.failed.iter().filter(|(_, name)| *name == account);
        let count = failures.clone().count();
        let oldest = if count >= MAX_FAILURES {
            failures.next()
        } else if self.failed.len() >= MAX_PEER_FAILURES {
            self.failed.front()
        } else {
            None
        };
        if let Some((at, _)) = oldest {
            let remaining = WINDOW.saturating_sub(now.duration_since(*at));
            let seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0);
            return Err(ApiError::throttled(seconds));
        }
        if self.pending.len() + self.failed.len() >= MAX_PEER_FAILURES
            || count + self.pending.iter().filter(|name| **name == account).count() >= MAX_FAILURES
        {
            let expires = self.failed.front().map_or(now, |(at, _)| *at) + WINDOW;
            return Ok(Some((self.changed.clone().notified_owned(), expires)));
        }
        self.pending.push(account);
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PeerKey {
    Address(IpAddr),
    Overflow(u64),
}

struct Peers {
    clients: HashMap<PeerKey, Attempts>,
    last_sweep: Instant,
}

impl Peers {
    fn get(&mut self, ip: IpAddr, group: u64, now: Instant) -> (PeerKey, &mut Attempts) {
        let full = self.clients.len() >= MAX_ADDRESSES;
        let interval = if full { Duration::from_secs(1) } else { WINDOW };
        if now.duration_since(self.last_sweep) >= interval {
            self.clients.retain(|_, entry| {
                entry.expire(now);
                !entry.is_empty()
            });
            self.last_sweep = now;
        }
        let address = PeerKey::Address(ip);
        let overflow = PeerKey::Overflow(group);
        // Keep admission and completion in the same group until its active window ends.
        let key = if self.clients.contains_key(&address) {
            address
        } else if self.clients.len() >= MAX_ADDRESSES || self.clients.contains_key(&overflow) {
            overflow
        } else {
            address
        };
        (key, self.clients.entry(key).or_default())
    }
}

pub struct LoginLimit {
    slots: Arc<Semaphore>,
    peers: Mutex<Peers>,
    names: RandomState,
}

impl LoginLimit {
    pub fn new(parallel: usize) -> Self {
        Self::with_slots(parallel.clamp(1, 4))
    }

    pub fn for_music_clients() -> Self {
        Self::with_slots(64)
    }

    fn with_slots(parallel: usize) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(parallel)),
            peers: Mutex::new(Peers {
                clients: HashMap::new(),
                last_sweep: Instant::now(),
            }),
            names: RandomState::new(),
        }
    }

    pub(crate) async fn begin(
        self: &Arc<Self>,
        ip: IpAddr,
        username: &str,
    ) -> Result<LoginAttempt, ApiError> {
        // Waiting requests also hold a slot so the admission queue stays bounded.
        let slot = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|_| ApiError::throttled(1))?;
        let account = self.names.hash_one((ip, username_key(username)));
        let group = self.names.hash_one(ip) % OVERFLOW_GROUPS;
        let peer = loop {
            let (peer, wait) = {
                let mut peers = self.peers.lock().unwrap_or_else(PoisonError::into_inner);
                let now = Instant::now();
                let (peer, attempts) = peers.get(ip, group, now);
                (peer, attempts.reserve(account, now)?)
            };
            match wait {
                Some((changed, expires)) => {
                    let _ = tokio::time::timeout_at(expires.into(), changed).await;
                }
                None => break peer,
            }
        };
        Ok(LoginAttempt {
            limit: Arc::clone(self),
            peer,
            account,
            _slot: slot,
            result: None,
        })
    }
}

pub(crate) struct LoginAttempt {
    limit: Arc<LoginLimit>,
    peer: PeerKey,
    account: u64,
    _slot: OwnedSemaphorePermit,
    result: Option<bool>,
}

impl LoginAttempt {
    pub(crate) fn finish(mut self, authenticated: bool) {
        self.result = Some(authenticated);
    }
}

impl Drop for LoginAttempt {
    fn drop(&mut self) {
        let mut peers = self
            .limit
            .peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(client) = peers.clients.get_mut(&self.peer) {
            if let Some(index) = client.pending.iter().position(|name| *name == self.account) {
                client.pending.swap_remove(index);
            }
            match self.result {
                Some(true) => client.failed.retain(|(_, name)| *name != self.account),
                Some(false) => client.failed.push_back((Instant::now(), self.account)),
                None => {}
            }
            client.changed.notify_waiters();
            if client.is_empty() {
                peers.clients.remove(&self.peer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;
    use axum::response::IntoResponse;
    use futures_util::poll;
    use std::net::Ipv4Addr;

    const PEER: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
    const OTHER_PEER: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

    #[tokio::test]
    async fn successful_concurrent_reads_wait_for_verification() {
        let limit = Arc::new(LoginLimit::for_music_clients());
        let mut reading = Vec::new();
        for _ in 0..MAX_FAILURES {
            reading.push(limit.begin(PEER, "listener").await.unwrap());
        }
        let mut next = Box::pin(limit.begin(PEER, "listener"));
        assert!(poll!(&mut next).is_pending());
        limit.begin(PEER, "neighbor").await.unwrap().finish(true);
        reading.pop().unwrap().finish(true);
        next.await.unwrap().finish(true);
        for read in reading {
            read.finish(true);
        }
        assert!(limit.peers.lock().unwrap().clients.is_empty());
    }

    #[tokio::test]
    async fn failures_and_resets_are_scoped_to_peer_and_login_name() {
        let limit = Arc::new(LoginLimit::new(2));
        for _ in 1..MAX_FAILURES {
            limit.begin(PEER, "Listener").await.unwrap().finish(false);
            limit.begin(PEER, "neighbor").await.unwrap().finish(false);
        }
        limit.begin(PEER, " neighbor ").await.unwrap().finish(true);
        let last = limit.begin(PEER, "LISTENER").await.unwrap();
        let mut next = Box::pin(limit.begin(PEER, " listener "));
        assert!(poll!(&mut next).is_pending());
        last.finish(false);
        let response = next.await.err().unwrap().into_response();
        assert_eq!(response.status(), 429);
        let seconds: u64 = response.headers()[header::RETRY_AFTER]
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!((1..=WINDOW.as_secs()).contains(&seconds));
        for _ in 0..MAX_FAILURES {
            limit.begin(PEER, "neighbor").await.unwrap().finish(false);
        }
        limit
            .begin(OTHER_PEER, "listener")
            .await
            .unwrap()
            .finish(true);
        assert!(limit.begin(PEER, "Listener").await.is_err());
    }

    #[tokio::test]
    async fn cancelled_checks_and_waiters_release_capacity_without_a_failure() {
        let limit = Arc::new(LoginLimit::with_slots(MAX_FAILURES + 1));
        let mut reading = Vec::new();
        for _ in 0..MAX_FAILURES {
            reading.push(limit.begin(PEER, "listener").await.unwrap());
        }
        let mut waiting = Box::pin(limit.begin(PEER, "listener"));
        assert!(poll!(&mut waiting).is_pending());
        assert_eq!(
            limit
                .begin(OTHER_PEER, "neighbor")
                .await
                .err()
                .unwrap()
                .status(),
            429
        );
        drop(waiting);
        let mut retry = Box::pin(limit.begin(PEER, "listener"));
        assert!(poll!(&mut retry).is_pending());
        drop(reading.pop());
        drop(retry.await.unwrap());
        drop(reading);
        assert!(limit.peers.lock().unwrap().clients.is_empty());
        assert_eq!(limit.slots.available_permits(), MAX_FAILURES + 1);
    }

    #[tokio::test]
    async fn source_budget_bounds_saved_failures_across_names() {
        let limit = Arc::new(LoginLimit::for_music_clients());
        for account in 0..MAX_PEER_FAILURES - 1 {
            limit
                .begin(PEER, &account.to_string())
                .await
                .unwrap()
                .finish(false);
        }
        let last = limit.begin(PEER, "listener").await.unwrap();
        let mut next = Box::pin(limit.begin(PEER, "neighbor"));
        assert!(poll!(&mut next).is_pending());
        last.finish(false);
        assert_eq!(next.await.err().unwrap().status(), 429);
        limit
            .begin(OTHER_PEER, "neighbor")
            .await
            .unwrap()
            .finish(true);
        assert_eq!(
            limit.peers.lock().unwrap().clients[&PeerKey::Address(PEER)]
                .failed
                .len(),
            MAX_PEER_FAILURES
        );
    }

    #[test]
    fn window_expires_each_failure_at_its_own_deadline() {
        let now = Instant::now();
        let mut client = Attempts::default();
        client.failed.push_back((now, 1));
        client.failed.extend(std::iter::repeat_n(
            (now + Duration::from_secs(1), 1),
            MAX_FAILURES - 1,
        ));
        let just_before = now + WINDOW - Duration::from_nanos(1);
        let response = client
            .reserve(1, just_before)
            .err()
            .unwrap()
            .into_response();
        assert_eq!(response.status(), 429);
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
        assert!(client.reserve(1, now + WINDOW).unwrap().is_none());
        let (_, expires) = client.reserve(1, now + WINDOW).unwrap().unwrap();
        assert_eq!(expires, now + WINDOW + Duration::from_secs(1));
        assert!(client.reserve(1, expires).unwrap().is_none());
    }

    #[test]
    fn full_peer_table_admits_new_peers_and_preserves_existing_windows() {
        let now = Instant::now();
        let mut peers = Peers {
            clients: HashMap::new(),
            last_sweep: now,
        };
        let (_, active) = peers.get(PEER, 0, now);
        assert!(active.reserve(1, now).unwrap().is_none());
        for i in 1..MAX_ADDRESSES {
            let ip = Ipv4Addr::from(u32::try_from(i).unwrap()).into();
            peers.get(ip, 0, now).1.failed.push_back((now, 1));
        }
        let (key, attempts) = peers.get(OTHER_PEER, 1, now);
        assert_eq!(key, PeerKey::Overflow(1));
        attempts.failed.push_back((now, 2));
        assert_eq!(peers.clients.len(), MAX_ADDRESSES + 1);
        peers
            .clients
            .remove(&PeerKey::Address(Ipv4Addr::from(1).into()));
        peers
            .clients
            .remove(&PeerKey::Address(Ipv4Addr::from(2).into()));
        let (key, attempts) = peers.get(OTHER_PEER, 1, now);
        assert_eq!(key, PeerKey::Overflow(1));
        assert_eq!(attempts.failed.len(), 1);
        assert_eq!(
            peers.get(OTHER_PEER, 1, now + WINDOW).0,
            PeerKey::Address(OTHER_PEER)
        );
        assert_eq!(peers.clients.len(), 2);
        assert_eq!(peers.clients[&PeerKey::Address(PEER)].pending, [1]);
    }

    #[tokio::test]
    async fn successful_logins_clear_only_their_own_failures_in_an_overflow_group() {
        let limit = Arc::new(LoginLimit::new(2));
        let now = Instant::now();
        let group = limit.names.hash_one(PEER) % OVERFLOW_GROUPS;
        let neighbor = (1..)
            .map(|i: u32| IpAddr::from(Ipv4Addr::from(i)))
            .find(|ip| limit.names.hash_one(ip) % OVERFLOW_GROUPS == group && *ip != PEER)
            .unwrap();
        limit
            .peers
            .lock()
            .unwrap()
            .clients
            .entry(PeerKey::Overflow(group))
            .or_default()
            .failed
            .push_back((now, 0));
        for _ in 0..MAX_FAILURES {
            limit.begin(PEER, "listener").await.unwrap().finish(false);
        }
        limit
            .begin(neighbor, "listener")
            .await
            .unwrap()
            .finish(true);
        assert_eq!(
            limit.begin(PEER, "listener").await.err().unwrap().status(),
            429
        );
    }
}
