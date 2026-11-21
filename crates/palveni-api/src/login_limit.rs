use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::ApiError;

const WINDOW: Duration = Duration::from_secs(60);
const MAX_FAILURES: usize = 8;
const MAX_ADDRESSES: usize = 4096;

#[derive(Default)]
struct Attempts {
    failed: VecDeque<Instant>,
    pending: usize,
}

impl Attempts {
    fn expire(&mut self, now: Instant) {
        while self
            .failed
            .front()
            .is_some_and(|at| now.duration_since(*at) >= WINDOW)
        {
            self.failed.pop_front();
        }
    }
}

pub struct LoginLimit {
    slots: Arc<Semaphore>,
    clients: Mutex<HashMap<IpAddr, Attempts>>,
}

impl LoginLimit {
    pub fn new(parallel: usize) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(parallel.clamp(1, 4))),
            clients: Mutex::default(),
        }
    }

    pub(crate) fn begin(self: &Arc<Self>, ip: IpAddr) -> Result<LoginAttempt, ApiError> {
        self.reserve(ip, Instant::now())
    }

    fn reserve(self: &Arc<Self>, ip: IpAddr, now: Instant) -> Result<LoginAttempt, ApiError> {
        let slot = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|_| ApiError::throttled(1))?;
        let mut clients = self.clients.lock().unwrap_or_else(PoisonError::into_inner);
        if clients.len() >= MAX_ADDRESSES && !clients.contains_key(&ip) {
            clients.retain(|_, entry| {
                entry.expire(now);
                entry.pending > 0 || !entry.failed.is_empty()
            });
            if clients.len() >= MAX_ADDRESSES {
                return Err(ApiError::throttled(1));
            }
        }
        let client = clients.entry(ip).or_default();
        client.expire(now);
        if client.pending + client.failed.len() >= MAX_FAILURES {
            let wait = client.failed.front().map_or(1, |oldest| {
                WINDOW
                    .saturating_sub(now.duration_since(*oldest))
                    .as_secs()
                    .saturating_add(1)
            });
            return Err(ApiError::throttled(wait.max(1)));
        }
        client.pending += 1;
        Ok(LoginAttempt {
            limit: Arc::clone(self),
            ip,
            _slot: slot,
            result: None,
        })
    }
}

pub(crate) struct LoginAttempt {
    limit: Arc<LoginLimit>,
    ip: IpAddr,
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
        let mut clients = self
            .limit
            .clients
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(client) = clients.get_mut(&self.ip) {
            client.pending -= 1;
            match self.result {
                Some(true) => client.failed.clear(),
                Some(false) => client.failed.push_back(Instant::now()),
                None => {}
            }
            if client.pending == 0 && client.failed.is_empty() {
                clients.remove(&self.ip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn failures_expire_and_success_resets_only_one_peer() {
        let limiter = Arc::new(LoginLimit::new(2));
        let peer = Ipv4Addr::LOCALHOST.into();
        for _ in 0..MAX_FAILURES - 1 {
            limiter.begin(peer).unwrap().finish(false);
        }
        limiter.begin(peer).unwrap().finish(true);
        for _ in 0..MAX_FAILURES {
            limiter.begin(peer).unwrap().finish(false);
        }
        assert_eq!(limiter.begin(peer).err().unwrap().status(), 429);
        assert!(limiter.begin(Ipv4Addr::new(127, 0, 0, 2).into()).is_ok());
        assert!(limiter.reserve(peer, Instant::now() + WINDOW).is_ok());
    }

    #[test]
    fn cancelled_work_releases_capacity() {
        let limiter = Arc::new(LoginLimit::new(1));
        let peer = Ipv4Addr::LOCALHOST.into();
        let slot = limiter.begin(peer).unwrap();
        assert!(limiter.begin(peer).is_err());
        drop(slot);
        assert!(limiter.begin(peer).is_ok());
    }
}
