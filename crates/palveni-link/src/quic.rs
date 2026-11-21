//! Authenticated transport for peers enrolled by the pairing service.
use crate::{NodeId, NodeIdentity, PairedPeers, Transport};
use iroh::{
    Endpoint, EndpointAddr, RelayMode, RelayUrl,
    endpoint::{Connection, WeakConnectionHandle, presets},
};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Mutex, PoisonError},
};

pub const ALPN: &[u8] = b"palveni/http/1";

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("the device is not paired")]
    NotPaired,
    #[error("at least one local bind address or relay is required")]
    NoTransport,
    #[error("the endpoint has closed")]
    Closed,
    #[error("transport error: {0}")]
    Transport(String),
}

fn transport(error: impl std::fmt::Display) -> LinkError {
    LinkError::Transport(error.to_string())
}

#[derive(Default)]
struct Peer {
    address: Option<EndpointAddr>,
    active: Vec<WeakConnectionHandle>,
}

pub struct QuicTransport {
    endpoint: Endpoint,
    local: NodeId,
    peers: Mutex<BTreeMap<NodeId, Peer>>,
}

impl QuicTransport {
    /// Only configured interfaces and relays are used. Peer addresses come from pairing.
    pub async fn bind(
        identity: &NodeIdentity,
        direct_bind: Option<SocketAddr>,
        relays: Vec<RelayUrl>,
    ) -> Result<Self, LinkError> {
        if direct_bind.is_none() && relays.is_empty() {
            return Err(LinkError::NoTransport);
        }
        let relay_mode = if relays.is_empty() {
            RelayMode::Disabled
        } else {
            RelayMode::custom(relays)
        };
        let builder = Endpoint::builder(presets::Minimal)
            .secret_key(identity.iroh_secret())
            .alpns(vec![ALPN.to_vec()])
            .clear_ip_transports()
            .relay_mode(relay_mode);
        let builder = match direct_bind {
            Some(address) => builder.bind_addr(address).map_err(transport)?,
            None => builder,
        };
        let endpoint = builder.bind().await.map_err(transport)?;
        Ok(Self {
            endpoint,
            local: identity.node_id(),
            peers: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn address(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    /// Apply an entry from the authenticated pairing store.
    pub fn register(&self, address: EndpointAddr) -> Result<NodeId, LinkError> {
        let node = NodeId::from_bytes(*address.id.as_bytes()).map_err(transport)?;
        let mut peers = self.peers.lock().unwrap_or_else(PoisonError::into_inner);
        peers.entry(node).or_default().address = Some(address);
        Ok(node)
    }

    /// Registers a paired server reached at a known UDP socket.
    pub fn register_direct(&self, node: NodeId, socket: SocketAddr) -> Result<NodeId, LinkError> {
        let id = iroh::EndpointId::from_bytes(node.as_bytes()).map_err(transport)?;
        self.register(EndpointAddr::new(id).with_ip_addr(socket))
    }

    pub fn revoke(&self, node: NodeId) {
        let mut peers = self.peers.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(peer) = peers.remove(&node) {
            for connection in peer.active.into_iter().filter_map(|h| h.upgrade()) {
                connection.close(1u32.into(), b"device revoked");
            }
        }
    }

    fn admit(&self, connection: Connection) -> Result<(NodeId, Connection), LinkError> {
        let node = NodeId::from_bytes(*connection.remote_id().as_bytes()).map_err(transport)?;
        let mut peers = self.peers.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(peer) = peers.get_mut(&node) else {
            connection.close(1u32.into(), b"device not paired");
            return Err(LinkError::NotPaired);
        };
        peer.active
            .retain(|h| h.upgrade().is_some_and(|c| c.close_reason().is_none()));
        peer.active.push(connection.weak_handle());
        Ok((node, connection))
    }

    pub async fn close(&self) {
        self.endpoint.close().await;
    }
}

impl PairedPeers for QuicTransport {
    fn allow_incoming(&self, node: NodeId) {
        self.peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(node)
            .or_default();
    }

    fn disconnect(&self, node: NodeId) {
        self.revoke(node);
    }

    fn connected(&self, node: NodeId) -> bool {
        self.peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&node)
            .is_some_and(|peer| {
                peer.active
                    .iter()
                    .any(|h| h.upgrade().is_some_and(|c| c.close_reason().is_none()))
            })
    }
}

impl Transport for QuicTransport {
    type Connection = Connection;
    type Error = LinkError;
    fn local_node_id(&self) -> NodeId {
        self.local
    }

    async fn connect(&self, peer: NodeId) -> Result<Connection, LinkError> {
        let address = self
            .peers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&peer)
            .and_then(|p| p.address.clone())
            .ok_or(LinkError::NotPaired)?;
        let connection = self
            .endpoint
            .connect(address, ALPN)
            .await
            .map_err(transport)?;
        self.admit(connection).map(|(_, connection)| connection)
    }

    async fn accept(&self) -> Result<(NodeId, Connection), LinkError> {
        let incoming = self.endpoint.accept().await.ok_or(LinkError::Closed)?;
        let connection = incoming.await.map_err(transport)?;
        self.admit(connection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn local_endpoint() -> QuicTransport {
        QuicTransport::bind(
            &NodeIdentity::generate().unwrap(),
            Some(([127, 0, 0, 1], 0).into()),
            Vec::new(),
        )
        .await
        .unwrap()
    }

    fn direct_address(transport: &QuicTransport) -> EndpointAddr {
        let socket = transport.endpoint.bound_sockets()[0];
        assert!(socket.ip().is_loopback());
        EndpointAddr::new(transport.endpoint.id()).with_ip_addr(socket)
    }

    #[tokio::test]
    async fn paired_peers_exchange_data_and_revocation_ends_the_connection() {
        tokio::time::timeout(Duration::from_secs(15), async {
            let server = local_endpoint().await;
            let client = local_endpoint().await;
            let client_id = server.register(direct_address(&client)).unwrap();
            let server_id = client.register(direct_address(&server)).unwrap();
            let (outgoing, incoming) = tokio::join!(client.connect(server_id), server.accept());
            let outgoing = outgoing.unwrap();
            let (sender, incoming) = incoming.unwrap();
            assert_eq!(sender, client_id);
            let (mut request, mut response) = outgoing.open_bi().await.unwrap();
            request.write_all(b"request").await.unwrap();
            request.finish().unwrap();
            let (mut reply, mut received) = incoming.accept_bi().await.unwrap();
            assert_eq!(received.read_to_end(16).await.unwrap(), b"request");
            reply.write_all(b"response").await.unwrap();
            reply.finish().unwrap();
            assert_eq!(response.read_to_end(16).await.unwrap(), b"response");

            server.revoke(client_id);
            outgoing.closed().await;
            assert!(incoming.close_reason().is_some());
            assert!(matches!(
                server.connect(client_id).await,
                Err(LinkError::NotPaired)
            ));
            let (_, next_incoming) = tokio::join!(client.connect(server_id), server.accept());
            assert!(matches!(next_incoming, Err(LinkError::NotPaired)));
            tokio::join!(client.close(), server.close());
        })
        .await
        .expect("isolated QUIC exchange timed out");
    }

    #[cfg(feature = "relay-tests")]
    #[tokio::test]
    async fn paired_peers_use_the_private_relay_with_an_unreachable_direct_address() {
        use iroh_relay::server::{RelayConfig, Server, ServerConfig};
        tokio::time::timeout(Duration::from_secs(25), async {
            let mut config = ServerConfig::default();
            config.relay = Some(RelayConfig::new(([127, 0, 0, 1], 0)));
            let relay = Server::spawn(config).await.unwrap();
            let relay_url: RelayUrl = format!("http://{}", relay.http_addr().unwrap())
                .parse()
                .unwrap();
            let server = QuicTransport::bind(
                &NodeIdentity::generate().unwrap(),
                None,
                vec![relay_url.clone()],
            )
            .await
            .unwrap();
            let client = QuicTransport::bind(
                &NodeIdentity::generate().unwrap(),
                Some(([127, 0, 0, 1], 0).into()),
                vec![relay_url.clone()],
            )
            .await
            .unwrap();
            tokio::join!(client.endpoint.online(), server.endpoint.online());
            server
                .register(EndpointAddr::new(client.endpoint.id()).with_relay_url(relay_url.clone()))
                .unwrap();
            // This reachable UDP socket never answers a QUIC handshake.
            let dead_end = std::net::UdpSocket::bind(("127.0.0.1", 0)).unwrap();
            let server_id = client
                .register(
                    EndpointAddr::new(server.endpoint.id())
                        .with_relay_url(relay_url)
                        .with_ip_addr(dead_end.local_addr().unwrap()),
                )
                .unwrap();
            let (outgoing, incoming) = tokio::join!(client.connect(server_id), server.accept());
            let outgoing = outgoing.unwrap();
            let (_, incoming) = incoming.unwrap();
            let mut stream = outgoing.open_uni().await.unwrap();
            stream.write_all(b"relay").await.unwrap();
            stream.finish().unwrap();
            let mut received = incoming.accept_uni().await.unwrap();
            assert_eq!(received.read_to_end(16).await.unwrap(), b"relay");
            assert!(outgoing.paths().iter().any(|path| path.is_relay()));
            assert!(outgoing.paths().iter().all(|path| !path.is_ip()));
            tokio::join!(client.close(), server.close());
            relay.shutdown().await.unwrap();
        })
        .await
        .expect("local relay exchange timed out");
    }
}
