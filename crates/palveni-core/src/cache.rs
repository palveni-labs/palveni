//! Byte-oriented cache abstraction.
//!
//! The in-memory cache supports expiry and explicit invalidation.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::error::Result;

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Stores `value`; `ttl == None` keeps it until evicted or deleted.
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()>;

    /// Returns whether a live entry was removed.
    async fn delete(&self, key: &str) -> Result<bool>;
}

struct Entry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
    /// Insertion order, used to evict the oldest entry when full.
    seq: u64,
}

impl Entry {
    fn is_live(&self, now: Instant) -> bool {
        match self.expires_at {
            Some(at) => at > now,
            None => true,
        }
    }
}

struct Inner {
    entries: HashMap<String, Entry>,
    next_seq: u64,
}

/// Process-local cache bounded by entry count.
pub struct MemoryCache {
    inner: Mutex<Inner>,
    max_entries: usize,
}

impl MemoryCache {
    pub fn new(max_entries: usize) -> Self {
        MemoryCache {
            inner: Mutex::new(Inner {
                entries: HashMap::new(),
                next_seq: 0,
            }),
            max_entries: max_entries.max(1),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // The map stays consistent even if a holder panicked mid-operation.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn len(&self) -> usize {
        let now = Instant::now();
        self.lock()
            .entries
            .values()
            .filter(|e| e.is_live(now))
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let now = Instant::now();
        let mut inner = self.lock();
        let live = inner.entries.get(key).map(|e| e.is_live(now));
        match live {
            Some(true) => Ok(inner.entries.get(key).map(|e| e.value.clone())),
            Some(false) => {
                inner.entries.remove(key);
                Ok(None)
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let now = Instant::now();
        let mut inner = self.lock();
        if !inner.entries.contains_key(key) && inner.entries.len() >= self.max_entries {
            inner.entries.retain(|_, e| e.is_live(now));
            if inner.entries.len() >= self.max_entries {
                let oldest = inner
                    .entries
                    .iter()
                    .min_by_key(|(_, e)| e.seq)
                    .map(|(k, _)| k.clone());
                if let Some(oldest) = oldest {
                    inner.entries.remove(&oldest);
                }
            }
        }
        let seq = inner.next_seq;
        inner.next_seq = inner.next_seq.wrapping_add(1);
        let expires_at = ttl.and_then(|ttl| now.checked_add(ttl));
        inner.entries.insert(
            key.to_owned(),
            Entry {
                value,
                expires_at,
                seq,
            },
        );
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let now = Instant::now();
        let removed = self.lock().entries.remove(key);
        Ok(removed.is_some_and(|e| e.is_live(now)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_get_delete() {
        let cache = MemoryCache::new(10);
        cache.set("a", b"one".to_vec(), None).await.unwrap();
        assert_eq!(cache.get("a").await.unwrap(), Some(b"one".to_vec()));
        assert!(cache.delete("a").await.unwrap());
        assert!(!cache.delete("a").await.unwrap());
        assert_eq!(cache.get("a").await.unwrap(), None);
    }

    #[tokio::test]
    async fn expired_entries_disappear() {
        let cache = MemoryCache::new(10);
        cache
            .set("short", b"x".to_vec(), Some(Duration::from_millis(1)))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(cache.get("short").await.unwrap(), None);
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn evicts_oldest_when_full() {
        let cache = MemoryCache::new(2);
        cache.set("a", vec![1], None).await.unwrap();
        cache.set("b", vec![2], None).await.unwrap();
        cache.set("c", vec![3], None).await.unwrap();
        assert_eq!(cache.get("a").await.unwrap(), None);
        assert_eq!(cache.get("b").await.unwrap(), Some(vec![2]));
        assert_eq!(cache.get("c").await.unwrap(), Some(vec![3]));
        assert_eq!(cache.len(), 2);
    }
}
