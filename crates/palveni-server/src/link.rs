//! The optional direct Link listener. Device keys are restored before accepting peers.

use std::{net::SocketAddr, sync::Arc};

#[cfg(feature = "link")]
use anyhow::Context;
use palveni_api::LinkPairings;
use palveni_db::Db;
use palveni_link::NodeIdentity;
use tokio_util::sync::CancellationToken;

pub async fn start(
    bind: Option<SocketAddr>,
    identity: &NodeIdentity,
    db: &Db,
    shutdown: &CancellationToken,
) -> anyhow::Result<(Arc<LinkPairings>, Option<tokio::task::JoinHandle<()>>)> {
    let Some(bind) = bind else {
        return Ok((Arc::default(), None));
    };
    #[cfg(not(feature = "link"))]
    {
        let _ = (bind, identity, db, shutdown);
        anyhow::bail!("link_bind requires a server built with the link feature");
    }
    #[cfg(feature = "link")]
    {
        use palveni_link::quic::QuicTransport;
        let transport = Arc::new(
            QuicTransport::bind(identity, Some(bind), Vec::new())
                .await
                .with_context(|| format!("cannot bind Link to {bind}"))?,
        );
        let pairings = LinkPairings::load(transport.clone(), db)
            .await
            .context("cannot restore paired devices")?;
        let listener = tokio::spawn(listen(transport, shutdown.clone()));
        tracing::info!(%bind, "direct Link listener started");
        Ok((Arc::new(pairings), Some(listener)))
    }
}

#[cfg(feature = "link")]
async fn listen(transport: Arc<palveni_link::quic::QuicTransport>, shutdown: CancellationToken) {
    use palveni_link::{Transport, quic::LinkError};
    // Each accept includes a TLS handshake. An incomplete handshake must not
    // prevent another device from connecting.
    let mut handshakes = tokio::task::JoinSet::new();
    let mut connections = tokio::task::JoinSet::new();
    loop {
        while handshakes.len() < 8 {
            let listener = Arc::clone(&transport);
            handshakes.spawn(async move { listener.accept().await });
        }
        tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            Some(result) = handshakes.join_next() => match result {
                Ok(Ok((_node, connection))) => {
                    connections.spawn(async move {
                        // Reject application streams until HTTP-over-Link is mounted.
                        while let Ok((mut send, mut receive)) = connection.accept_bi().await {
                            let _ = receive.stop(2u32.into());
                            let _ = send.reset(2u32.into());
                        }
                    });
                }
                Ok(Err(LinkError::Closed)) => break,
                Ok(Err(err)) => tracing::debug!(error=%err, "Link connection declined"),
                Err(err) => tracing::warn!(error=%err, "Link handshake failed"),
            }
        }
    }
    handshakes.shutdown().await;
    transport.close().await;
    connections.shutdown().await;
}
