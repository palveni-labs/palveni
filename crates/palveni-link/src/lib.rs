//! Palveni Link device identities, pairing policy and transport traits.
//!
//! Every server and every client device owns an Ed25519 key pair; the public
//! key is its address (`NodeId`). Pairing means both sides remember each
//! other's `NodeId`.
//!
//! This crate must not depend on any other workspace crate: it is compiled
//! into the client apps.

mod identity;
#[cfg(feature = "quic")]
pub mod quic;
mod transport;

pub use identity::{IdentityError, NodeId, NodeIdentity, SIGNATURE_LENGTH};
pub use transport::{AllowList, PairedPeers, PeerPolicy, Transport};
