use super::Harness;
use crate::connections::RemoteConnection;
use palveni_core::ids::ConnectionId;

#[tokio::test]
async fn read_only_network_sources_preserve_credentials_until_removed() {
    let h = Harness::new().await;
    let id = ConnectionId::generate().to_string();
    let record = RemoteConnection {
        id: id.clone(),
        name: "家庭媒体".into(),
        kind: "smb".into(),
        address: "smb://example.test/music".into(),
        credentials: "opaque-ciphertext".into(),
    };
    h.db.add_media_connection(record)
        .await
        .expect("create connection");
    let records = h.db.media_connections().await.expect("list connections");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, id);
    let stored =
        h.db.media_connection(&id)
            .await
            .expect("lookup connection")
            .expect("persisted source");
    assert_eq!(stored.name, "家庭媒体");
    assert_eq!(stored.credentials, "opaque-ciphertext");
    assert!(
        h.db.remove_media_connection(&id)
            .await
            .expect("delete connection")
    );
    assert!(
        h.db.media_connection(&id)
            .await
            .expect("query removed source")
            .is_none()
    );
    h.finish().await;
}
