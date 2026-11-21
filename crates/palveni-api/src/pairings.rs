//! Live transport policy and single-use proof of possession for device pairing.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use axum::http::StatusCode;
use palveni_core::{DeviceId, SessionId};
use palveni_db::{Db, DbError};
use palveni_link::{NodeId, PairedPeers};
use ring::rand::{SecureRandom, SystemRandom};

use crate::ApiError;

const LIFETIME: Duration = Duration::from_secs(300);
const MAX_PENDING: usize = 1024;

struct Challenge {
    text: String,
    expires: Instant,
}

#[derive(Default)]
pub struct LinkPairings {
    transport: Option<Arc<dyn PairedPeers>>,
    pending: Mutex<HashMap<SessionId, Challenge>>,
    // Serialize each durable write with publication to the active listener.
    pub(crate) changes: tokio::sync::Mutex<()>,
}

impl LinkPairings {
    pub async fn load(transport: Arc<dyn PairedPeers>, db: &Db) -> Result<Self, DbError> {
        for device in db.sessions().paired_devices(None).await? {
            let node = device.public_key.parse().map_err(|_| {
                DbError::Corrupt(format!("invalid Link key on device {}", device.id))
            })?;
            transport.allow_incoming(node);
        }
        Ok(Self {
            transport: Some(transport),
            ..Default::default()
        })
    }

    pub fn enabled(&self) -> bool {
        self.transport.is_some()
    }

    pub(crate) fn peers(&self) -> Result<&dyn PairedPeers, ApiError> {
        self.transport
            .as_deref()
            .ok_or_else(|| ApiError::not_found("Link is disabled"))
    }

    pub(crate) fn challenge(
        &self,
        server: NodeId,
        device: DeviceId,
        session: SessionId,
    ) -> Result<String, ApiError> {
        let mut random = [0u8; 32];
        SystemRandom::new()
            .fill(&mut random)
            .map_err(ApiError::internal)?;
        let text = format!(
            "palveni:link-pairing:v1\n{server}\n{device}\n{}",
            hex::encode(random)
        );
        let now = Instant::now();
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        pending.retain(|_, entry| entry.expires > now);
        if pending.len() >= MAX_PENDING && !pending.contains_key(&session) {
            return Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "busy",
                "Too many pending Link pairings",
            ));
        }
        pending.insert(
            session,
            Challenge {
                text: text.clone(),
                expires: now + LIFETIME,
            },
        );
        Ok(text)
    }

    pub(crate) fn verify(
        &self,
        session: SessionId,
        key: NodeId,
        signature: &[u8; 64],
    ) -> Result<(), ApiError> {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        if !pending.get(&session).is_some_and(|entry| {
            entry.expires > Instant::now() && key.verify(entry.text.as_bytes(), signature)
        }) {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "forbidden",
                "Invalid or expired Link pairing proof",
            ));
        }
        pending.remove(&session);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use palveni_link::NodeIdentity;

    #[test]
    fn challenge_is_scoped_replaced_expiring_and_single_use() {
        let service = LinkPairings::default();
        let client = NodeIdentity::generate().unwrap();
        let server = NodeIdentity::generate().unwrap().node_id();
        let session = SessionId::generate();
        let device = DeviceId::generate();
        let stale = service.challenge(server, device, session).unwrap();
        let current = service.challenge(server, device, session).unwrap();
        let proof = client.sign(current.as_bytes());
        assert!(
            service
                .verify(session, client.node_id(), &client.sign(stale.as_bytes()))
                .is_err()
        );
        assert!(
            service
                .verify(SessionId::generate(), client.node_id(), &proof)
                .is_err()
        );
        service.verify(session, client.node_id(), &proof).unwrap();
        assert!(service.verify(session, client.node_id(), &proof).is_err());
        let expiring = service.challenge(server, device, session).unwrap();
        service
            .pending
            .lock()
            .unwrap()
            .get_mut(&session)
            .unwrap()
            .expires = Instant::now();
        assert!(
            service
                .verify(session, client.node_id(), &client.sign(expiring.as_bytes()))
                .is_err()
        );
    }
}
