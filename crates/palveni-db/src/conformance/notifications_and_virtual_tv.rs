use chrono::{TimeZone, Utc};
use palveni_core::notifications::{
    InboundWebhookSourceKind, NotificationChannelKind, NotificationDeliveryStatus,
    RoutineActionKind, RoutineScheduleKind,
};
use palveni_core::virtual_tv::DEFAULT_VIRTUAL_RUNTIME_MS;
use palveni_core::{ItemKind, LibraryKind, Role, clock};

use super::{Harness, ItemUpsert, NewLibrary, track};
use crate::notifications::{
    NewNotificationDelivery, NewRoutine, NotificationChannelPatch, NotificationChannelRecord,
    RoutinePatch,
};
use crate::video::VideoMetadata;

fn video_meta(
    item_id: palveni_core::ItemId,
    group: &str,
    work_title: &str,
    title: &str,
    runtime_ms: Option<i64>,
    genres: Vec<String>,
) -> VideoMetadata {
    VideoMetadata {
        item_id,
        stamp: "vtv-test".into(),
        work_group: group.into(),
        work_title: work_title.into(),
        year: Some(2025),
        work_overview: None,
        work_poster: None,
        work_original_title: None,
        work_tagline: None,
        work_rating: None,
        work_official_rating: None,
        work_premiered: None,
        work_studios: Vec::new(),
        work_backdrop: None,
        season: None,
        season_poster: None,
        episode: None,
        title: title.into(),
        overview: None,
        episode_poster: None,
        runtime_ms,
        rating: None,
        premiered: None,
        thumb: None,
        genres,
        people: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_notifications_routines_webhooks_and_virtual_tv() {
    let h = Harness::new().await;
    let admin =
        h.db.users()
            .create_first_admin("admin", "hash-a")
            .await
            .unwrap()
            .expect("admin");
    let member =
        h.db.users()
            .create("member", "hash-m", Role::User)
            .await
            .unwrap();

    let notifs = h.db.notifications();

    // 1. Notification channels CRUD + owner isolation
    let now_ms = clock::to_millis(clock::now());
    let ch = notifs
        .create_channel(NotificationChannelRecord {
            id: palveni_core::JobId::generate().to_string(),
            owner_id: admin.id,
            name: "客厅飞书机器人".into(),
            kind: NotificationChannelKind::Feishu,
            config_cipher: "nonce.ciphertext".into(),
            events: vec!["webhook.triggered".into(), "job.failed".into()],
            quiet_hours: Some("23:00-07:00".into()),
            enabled: true,
            created_at: now_ms,
            updated_at: now_ms,
        })
        .await
        .expect("create channel");
    assert_eq!(notifs.list_channels(admin.id, true).await.unwrap().len(), 1);
    assert!(
        notifs
            .list_channels(member.id, false)
            .await
            .unwrap()
            .is_empty()
    );

    let patched = notifs
        .update_channel(
            admin.id,
            true,
            &ch.id,
            NotificationChannelPatch {
                name: Some("全屋飞书告警".into()),
                quiet_hours: Some(None),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .expect("patched channel");
    assert_eq!(patched.name, "全屋飞书告警");
    assert!(patched.quiet_hours.is_none());

    // 2. Notification deliveries + read-all + admin broadcast visibility
    notifs
        .record_delivery(NewNotificationDelivery {
            owner_id: None, // system/admin broadcast
            channel_id: None,
            event_type: "webhook.triggered".into(),
            title: "Sonarr 新剧入库".into(),
            body: "已触发影视库增量扫描".into(),
            status: NotificationDeliveryStatus::InApp,
            error: None,
        })
        .await
        .unwrap();
    notifs
        .record_delivery(NewNotificationDelivery {
            owner_id: Some(member.id),
            channel_id: None,
            event_type: "routine.completed".into(),
            title: "例行任务完成".into(),
            body: "已完成早安场景切换".into(),
            status: NotificationDeliveryStatus::InApp,
            error: None,
        })
        .await
        .unwrap();

    let admin_inbox = notifs.list_deliveries(admin.id, true, 50).await.unwrap();
    assert_eq!(admin_inbox.unread_count, 1);
    assert_eq!(admin_inbox.notifications.len(), 1);

    let member_inbox = notifs.list_deliveries(member.id, false, 50).await.unwrap();
    assert_eq!(member_inbox.unread_count, 1);
    assert_eq!(member_inbox.notifications[0].title, "例行任务完成");

    assert_eq!(notifs.mark_all_read(admin.id, true).await.unwrap(), 1);
    assert_eq!(
        notifs
            .list_deliveries(admin.id, true, 50)
            .await
            .unwrap()
            .unread_count,
        0
    );

    // 3. Inbound webhooks SHA-256 token lookup
    let raw_token = "pwh_secret_test_token_123456";
    let wh = notifs
        .create_inbound_webhook(
            "Sonarr 主力实例",
            InboundWebhookSourceKind::Sonarr,
            raw_token,
            None,
            true,
        )
        .await
        .expect("create inbound webhook");
    assert!(
        notifs
            .find_inbound_webhook_by_token("wrong_token")
            .await
            .unwrap()
            .is_none()
    );
    let found = notifs
        .find_inbound_webhook_by_token(raw_token)
        .await
        .unwrap()
        .expect("found webhook");
    assert_eq!(found.id, wh.id);
    notifs
        .touch_inbound_webhook(&wh.id, now_ms)
        .await
        .expect("touch webhook");
    assert!(
        notifs
            .list_inbound_webhooks()
            .await
            .unwrap()
            .into_iter()
            .find(|w| w.id == wh.id)
            .unwrap()
            .last_triggered_at
            .is_some()
    );
    assert!(notifs.delete_inbound_webhook(&wh.id).await.unwrap());

    // 4. Scheduled routines lifecycle
    let routine = notifs
        .create_routine(NewRoutine {
            owner_id: admin.id,
            name: "每日凌晨全库巡检",
            schedule_kind: RoutineScheduleKind::Daily,
            schedule_expr: "03:00",
            action_kind: RoutineActionKind::ScanAll,
            action_payload: "",
            enabled: true,
        })
        .await
        .expect("create routine");
    assert_eq!(notifs.list_enabled_routines().await.unwrap().len(), 1);
    let updated_routine = notifs
        .update_routine(
            admin.id,
            true,
            &routine.id,
            RoutinePatch {
                schedule_expr: Some("04:00".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .expect("updated routine");
    assert_eq!(updated_routine.schedule_expr, "04:00");
    let ran = notifs
        .mark_routine_ran(&routine.id, now_ms, "succeeded")
        .await
        .unwrap()
        .expect("marked ran");
    assert_eq!(ran.last_status.as_deref(), Some("succeeded"));
    assert!(
        notifs
            .delete_routine(admin.id, true, &routine.id)
            .await
            .unwrap()
    );
    assert!(notifs.delete_channel(admin.id, true, &ch.id).await.unwrap());

    // 5. Virtual Live TV channels + genre filtering + unprobed fallback scheduler
    let lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "家庭影院".into(),
                kind: LibraryKind::Video,
                paths: vec!["/media/cinema".into()],
            })
            .await
            .unwrap();

    let item_scifi = ItemUpsert {
        kind: ItemKind::Video,
        ..track("arrival", "/media/cinema/arrival.mkv", None, 1024)
    };
    let item_drama = ItemUpsert {
        kind: ItemKind::Video,
        ..track("autumn", "/media/cinema/autumn.mkv", None, 1024)
    };
    h.db.items()
        .apply_scan_batch(lib.id, &[item_scifi.clone(), item_drama.clone()], &[])
        .await
        .unwrap();
    h.db.video()
        .save_batch(
            lib.id,
            vec![
                video_meta(
                    item_scifi.id,
                    "arrival",
                    "Arrival",
                    "Arrival (4K)",
                    Some(3_600_000), // 60 min probed
                    vec!["Sci-Fi".into()],
                ),
                video_meta(
                    item_drama.id,
                    "autumn",
                    "Autumn Sonata",
                    "Autumn Sonata",
                    None, // unprobed -> 2,700,000 ms (45 min)
                    vec!["Drama".into()],
                ),
            ],
        )
        .await
        .unwrap();

    let vtv = h.db.virtual_tv();
    let ch_all = vtv
        .create_channel(101, "综合影院台", lib.id, None, true)
        .await
        .expect("create channel 101");
    let ch_scifi = vtv
        .create_channel(102, "科幻轮播台", lib.id, Some("sci-fi"), true)
        .await
        .expect("create channel 102");
    assert!(
        vtv.create_channel(101, "冲突频道", lib.id, None, true)
            .await
            .is_err()
    );

    // Total cycle for ch_all = 3,600,000 + 2,700,000 = 6,300,000 ms.
    // At epoch = 6,300,000 * 50 + 4,000,000 -> 400,000 ms into Autumn Sonata!
    let probe_ms = 6_300_000_i64 * 50 + 4_000_000;
    let probe_time = Utc.timestamp_millis_opt(probe_ms).single().unwrap();
    let guide = vtv
        .guide_at(
            probe_time,
            "/api/v1/virtual-channels/playlist.m3u".into(),
            "/api/v1/virtual-channels/epg.xml".into(),
        )
        .await
        .expect("guide_at");
    assert_eq!(guide.channels.len(), 2);

    let g_all = &guide.channels[0];
    assert_eq!(g_all.channel.id, ch_all.id);
    assert_eq!(g_all.total_items, 2);
    assert_eq!(g_all.cycle_duration_ms, 6_300_000);
    let now_all = g_all.now_playing.as_ref().expect("ch_all now_playing");
    assert_eq!(now_all.item_id, item_drama.id);
    assert_eq!(now_all.duration_ms, DEFAULT_VIRTUAL_RUNTIME_MS);
    assert_eq!(now_all.offset_ms, 400_000);
    assert!(!g_all.programmes.is_empty());

    let g_scifi = &guide.channels[1];
    assert_eq!(g_scifi.channel.id, ch_scifi.id);
    assert_eq!(g_scifi.total_items, 1);
    assert_eq!(g_scifi.cycle_duration_ms, 3_600_000);
    assert_eq!(g_scifi.now_playing.as_ref().unwrap().item_id, item_scifi.id);

    assert!(vtv.delete_channel(&ch_scifi.id).await.unwrap());
    h.finish().await;
}
