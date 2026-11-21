//! Transport abstraction and policy for paired peers.

use std::collections::HashSet;
use std::sync::{PoisonError, RwLock};

use crate::identity::NodeId;

/// Connection-level authorisation: only paired peers get through.
pub trait PeerPolicy: Send + Sync {
    fn is_allowed(&self, peer: &NodeId) -> bool;
}

/// A peer-to-peer transport.
pub trait Transport: Send + Sync {
    type Connection: Send;
    type Error: std::error::Error + Send + Sync + 'static;

    fn local_node_id(&self) -> NodeId;

    /// Dials a peer by its node ID (direct first, relay as fallback).
    fn connect(
        &self,
        peer: NodeId,
    ) -> impl std::future::Future<Output = Result<Self::Connection, Self::Error>> + Send;

    /// Waits for the next inbound connection from an allowed peer.
    fn accept(
        &self,
    ) -> impl std::future::Future<Output = Result<(NodeId, Self::Connection), Self::Error>> + Send;
}

/// In-memory set of paired peers.
#[derive(Debug, Default)]
pub struct AllowList {
    peers: RwLock<HashSet<NodeId>>,
}

impl AllowList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, peer: NodeId) -> bool {
        self.peers
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(peer)
    }

    /// Revoking a device takes effect for the next connection attempt.
    pub fn remove(&self, peer: &NodeId) -> bool {
        self.peers
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(peer)
    }
}

impl PeerPolicy for AllowList {
    fn is_allowed(&self, peer: &NodeId) -> bool {
        self.peers
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(peer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::NodeIdentity;

    #[test]
    fn allow_list() {
        let list = AllowList::new();
        let peer = NodeIdentity::from_secret_bytes(&[1u8; 32]).node_id();
        assert!(!list.is_allowed(&peer));
        assert!(list.insert(peer));
        assert!(list.is_allowed(&peer));
        assert!(list.remove(&peer));
        assert!(!list.is_allowed(&peer));
    }
}
