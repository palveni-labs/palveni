use palveni_core::{
    Role,
    broadcast::{BroadcastImport, BroadcastKind, ImportedEntry},
};

use super::{DbError, Harness};

#[tokio::test]
async fn conformance_broadcast_refresh_paginates_and_preserves_personal_favorites() {
    let h = Harness::new().await;
    let db = &h.db;
    let a = db
        .users()
        .create("listener", "hash", Role::User)
        .await
        .unwrap()
        .id;
    let b = db
        .users()
        .create("household", "hash", Role::User)
        .await
        .unwrap()
        .id;
    let source_id = "019d5c9d-2240-74df-bff2-807328ef96b4";
    let mut incoming = BroadcastImport {
        kind: BroadcastKind::Radio,
        name: "Home stations".into(),
        expected_version: 0,
        content: None,
        guide: None,
        feed_url: None,
        guide_url: None,
    };
    let entry = |key: &str, url: &str| ImportedEntry {
        key: key.into(),
        title: key.into(),
        group: Some("Local".into()),
        streams: vec![url.into()],
        published_at: None,
        programmes: vec![],
    };
    let original = db
        .save_broadcast(
            source_id,
            &incoming,
            vec![
                entry("one", "https://media.example/one"),
                entry("two", "https://media.example/two"),
            ],
        )
        .await
        .unwrap();
    let page = db
        .broadcast_entries(a, source_id, original.version, 0, 1)
        .await
        .unwrap();
    assert_eq!(page.next_offset, Some(1));
    let key = page.entries[0].id.clone();
    db.broadcast_favorite(a, source_id, &key, true)
        .await
        .unwrap();
    db.broadcast_favorite(a, source_id, &key, true)
        .await
        .unwrap();
    incoming.expected_version = original.version;
    let fresh = db
        .save_broadcast(
            source_id,
            &incoming,
            vec![
                entry("two", "https://media.example/two"),
                entry("one", "https://media.example/updated"),
            ],
        )
        .await
        .unwrap();
    assert!(matches!(
        db.broadcast_entries(a, source_id, original.version, 0, 1)
            .await,
        Err(DbError::Conflict(_))
    ));
    assert!(matches!(
        db.save_broadcast(
            source_id,
            &incoming,
            vec![entry("stale", "https://media.example/stale")]
        )
        .await,
        Err(DbError::Conflict(_))
    ));
    let updated = db
        .broadcast_entries(a, source_id, fresh.version, 1, 1)
        .await
        .unwrap();
    assert!(updated.next_offset.is_none());
    assert_eq!(updated.entries[0].id, key);
    assert_eq!(
        updated.entries[0].streams,
        ["https://media.example/updated"]
    );
    assert!(updated.entries[0].favorite);
    assert!(
        !db.broadcast_entries(b, source_id, fresh.version, 1, 1)
            .await
            .unwrap()
            .entries[0]
            .favorite
    );
    assert!(
        db.broadcast_sources(Some(BroadcastKind::Podcast))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.broadcast_sources(Some(BroadcastKind::Radio))
            .await
            .unwrap()
            .len(),
        1
    );
    incoming.expected_version = fresh.version;
    let reduced = db
        .save_broadcast(
            source_id,
            &incoming,
            vec![entry("two", "https://media.example/two")],
        )
        .await
        .unwrap();
    incoming.expected_version = reduced.version;
    let restored = db
        .save_broadcast(
            source_id,
            &incoming,
            vec![entry("one", "https://media.example/one")],
        )
        .await
        .unwrap();
    assert!(
        !db.broadcast_entries(a, source_id, restored.version, 0, 1)
            .await
            .unwrap()
            .entries[0]
            .favorite
    );
    db.delete_broadcast(source_id, restored.version)
        .await
        .unwrap();
    assert!(matches!(
        db.broadcast_entries(a, source_id, fresh.version, 0, 200)
            .await,
        Err(DbError::NotFound(_))
    ));
    h.finish().await;
}
