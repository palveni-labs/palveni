use palveni_core::Role;
use palveni_core::downloads::{
    CreateSubscriptionRequest, DownloaderKind, OrganizeMode, OrganizeOperationRecord,
    SubscriptionMediaKind,
};

use super::{Harness, new_library, track};
use crate::DbError;
use crate::downloads::SaveDownloaderClient;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_downloads_subscriptions_and_organize_batches() {
    let h = Harness::new().await;
    let admin =
        h.db.users()
            .create("admin", "pw-hash", Role::Admin)
            .await
            .expect("create admin");
    let member =
        h.db.users()
            .create("alice", "pw-hash", Role::User)
            .await
            .expect("create member");

    let repo = h.db.downloads();

    // 1. Downloader client CRUD + credential retention
    let client = repo
        .save_client(SaveDownloaderClient {
            id: None,
            name: "NAS qBittorrent",
            kind: DownloaderKind::Qbittorrent,
            base_url: "http://192.168.1.20:8080",
            credential_cipher: Some("nonce.ciphertext".into()),
            keep_existing_credential: true,
            save_path: Some("/downloads/media".into()),
            enabled: true,
        })
        .await
        .expect("save downloader client");
    assert_eq!(client.name, "NAS qBittorrent");
    assert_eq!(client.kind, DownloaderKind::Qbittorrent);
    assert!(client.enabled);

    let updated_client = repo
        .save_client(SaveDownloaderClient {
            id: Some(&client.id),
            name: "NAS qBittorrent Pro",
            kind: DownloaderKind::Qbittorrent,
            base_url: "http://192.168.1.20:8080",
            credential_cipher: None,
            keep_existing_credential: true,
            save_path: Some("/downloads/movies".into()),
            enabled: false,
        })
        .await
        .expect("update downloader client");
    assert_eq!(updated_client.name, "NAS qBittorrent Pro");
    assert!(!updated_client.enabled);
    assert_eq!(
        updated_client.credential_cipher.as_deref(),
        Some("nonce.ciphertext")
    );
    assert_eq!(repo.list_clients().await.expect("list clients").len(), 1);

    // 2. Media subscriptions + owner scoping + cascade on user delete
    let sub = repo
        .create_subscription(
            member.id,
            &CreateSubscriptionRequest {
                title: "三体".into(),
                media_kind: SubscriptionMediaKind::Series,
                quality_rule: Some("2160p".into()),
                feed_url: Some("https://example.com/rss.xml".into()),
                target_library_id: None,
                enabled: true,
            },
        )
        .await
        .expect("create subscription");
    assert_eq!(sub.title, "三体");
    assert!(sub.last_checked_at.is_none());

    let checked = repo
        .mark_subscription_checked(&sub.id, 1_700_000_123_000)
        .await
        .expect("mark checked")
        .expect("subscription exists");
    assert!(checked.last_checked_at.is_some());

    assert_eq!(
        repo.list_subscriptions(Some(member.id))
            .await
            .expect("member subs")
            .len(),
        1
    );
    assert_eq!(
        repo.list_subscriptions(Some(admin.id))
            .await
            .expect("admin own subs")
            .len(),
        0
    );

    // Non-owner non-admin cannot delete someone else's subscription
    let other =
        h.db.users()
            .create("bob", "pw-hash", Role::User)
            .await
            .expect("create bob");
    assert!(
        !repo
            .delete_subscription(&sub.id, other.id, false)
            .await
            .expect("bob delete attempt")
    );
    assert!(
        repo.delete_subscription(&sub.id, member.id, false)
            .await
            .expect("owner delete")
    );

    // 3. Organize batch atomic path update + 1-click undo
    let library = new_library(&h.db, "OrganizeMusic").await;
    let item_row = track(
        "晴天 (2004)",
        "/media/OrganizeMusic/raw-qingtian.flac",
        None,
        24_000_000,
    );
    let item_id = item_row.id;
    h.db.items()
        .apply_scan_batch(library.id, &[item_row], &[])
        .await
        .expect("seed item");

    let metas = repo
        .library_organize_items(library.id, 50)
        .await
        .expect("organize items");
    assert_eq!(metas.len(), 1);
    assert_eq!(metas[0].year, Some(2004));

    let op = OrganizeOperationRecord {
        item_id,
        from_path: "/media/OrganizeMusic/raw-qingtian.flac".into(),
        to_path: "/media/OrganizeMusic/周杰伦 - 晴天 (2004).flac".into(),
        created_dirs: Vec::new(),
    };
    let batch = repo
        .record_organize_batch(
            admin.id,
            library.id,
            OrganizeMode::Rename,
            "{artist} - {title}",
            std::slice::from_ref(&op),
        )
        .await
        .expect("record organize batch");
    assert_eq!(batch.operation_count, 1);
    assert!(batch.undone_at.is_none());

    let after_apply =
        h.db.items()
            .get(item_id)
            .await
            .expect("get item")
            .expect("item exists");
    assert_eq!(
        after_apply.path.as_deref(),
        Some("/media/OrganizeMusic/周杰伦 - 晴天 (2004).flac")
    );

    let undone = repo
        .mark_organize_batch_undone(&batch.id)
        .await
        .expect("undo batch")
        .expect("batch exists");
    assert!(undone.undone_at.is_some());

    // Undoing twice returns Conflict
    assert!(matches!(
        repo.mark_organize_batch_undone(&batch.id).await,
        Err(DbError::Conflict(_))
    ));

    let after_undo =
        h.db.items()
            .get(item_id)
            .await
            .expect("get item after undo")
            .expect("item exists");
    assert_eq!(
        after_undo.path.as_deref(),
        Some("/media/OrganizeMusic/raw-qingtian.flac")
    );

    assert!(repo.delete_client(&client.id).await.expect("delete client"));
    h.finish().await;
}
