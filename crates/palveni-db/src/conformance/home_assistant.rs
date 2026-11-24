use palveni_core::Role;
use palveni_core::assistant::{
    ApprovalEffectItem, ApprovalRisk, ApprovalStatus, AssistantMessage, AssistantRun, Block,
    ChangeRow, MessageRole, RunState, RunStep, RunStepState,
};
use palveni_core::clock;
use palveni_core::home::{
    DevicePatch, DeviceState, MediaPlayerPatchStatus, PresetDeviceAction, SceneIcon,
};

use super::{Harness, new_library, track};
use crate::home::{HomeBridgeRecord, SyncedBridgeDevice, SyncedBridgeScene};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_home_seeding_control_rules_and_bridges() {
    let h = Harness::new().await;
    let home = h.db.home();

    let snap = home.snapshot().await.expect("initial home snapshot");
    assert_eq!(snap.rooms.len(), 3);
    assert!(snap.rooms.iter().any(|r| r.name == "客厅"));
    assert!(snap.rooms.iter().any(|r| r.name == "书房"));
    assert!(snap.rooms.iter().any(|r| r.name == "主卧"));
    assert!(snap.devices.len() >= 8);
    assert_eq!(snap.scenes.len(), 5);
    assert!(snap.rules.len() >= 3);

    // Patching a light updates brightness and turns it on.
    let updated = home
        .apply_device_patch(
            "bed-lamp",
            &DevicePatch {
                brightness: Some(55.0),
                ..Default::default()
            },
        )
        .await
        .expect("patch device")
        .expect("bed-lamp exists");
    assert_eq!(
        updated.device.state,
        DeviceState::Light {
            on: true,
            brightness: 55,
            color_temp: Some(2700),
        }
    );

    // Scene activation switches active flag.
    assert!(
        home.set_active_scene("movie")
            .await
            .expect("activate movie")
    );
    let snap = home.snapshot().await.expect("snapshot after scene");
    assert!(snap.scenes.iter().find(|s| s.id == "movie").unwrap().active);
    assert!(!snap.scenes.iter().find(|s| s.id == "home").unwrap().active);

    // Custom device + custom rule lifecycle.
    let custom_dev = home
        .create_device(
            "custom-fan-1".into(),
            "阳台",
            "新风插座",
            DeviceState::Switch {
                on: false,
                power: None,
            },
        )
        .await
        .expect("create custom device");
    assert_eq!(custom_dev.room.as_deref(), Some("阳台"));

    let custom_rule = home
        .create_rule(
            "rule-custom-1".into(),
            "video.play",
            "放映时打开新风",
            "新风插座开启",
            &[PresetDeviceAction {
                device_id: custom_dev.id.clone(),
                patch: DevicePatch {
                    on: Some(true),
                    ..Default::default()
                },
            }],
        )
        .await
        .expect("create custom rule");
    assert!(custom_rule.enabled);

    let matching = home
        .enabled_rules_for_events(&["video.play"])
        .await
        .expect("enabled rules");
    assert!(matching.iter().any(|r| r.rule.id == "rule-custom-1"));
    home.mark_rule_ran("rule-custom-1", clock::to_millis(clock::now()))
        .await
        .expect("mark ran");
    let toggled = home
        .set_rule_enabled("rule-custom-1", false)
        .await
        .expect("toggle rule")
        .expect("rule exists");
    assert!(!toggled.enabled);
    assert!(toggled.last_run_at.is_some());

    // Home Assistant bridge save + sync + cascade delete.
    let bridge_id = palveni_core::ids::ConnectionId::generate().to_string();
    home.save_bridge(HomeBridgeRecord {
        id: bridge_id.clone(),
        name: "家里 HA".into(),
        base_url: "http://192.168.1.10:8123".into(),
        encrypted_token: "enc.token".into(),
        enabled: true,
        last_sync_at: None,
        created_at: clock::to_millis(clock::now()),
    })
    .await
    .expect("save bridge");

    home.apply_bridge_sync(
        &bridge_id,
        &[SyncedBridgeDevice {
            id: "ha-light-desk".into(),
            room_name: "书房".into(),
            external_entity_id: "light.study_desk".into(),
            name: "HA 书桌灯".into(),
            online: true,
            state: DeviceState::Light {
                on: true,
                brightness: 88,
                color_temp: None,
            },
        }],
        &[SyncedBridgeScene {
            id: "ha-scene-reading".into(),
            external_entity_id: "scene.reading".into(),
            name: "阅读场景".into(),
            icon: SceneIcon::Home,
        }],
    )
    .await
    .expect("sync bridge");

    let bridged = home
        .device_record("ha-light-desk")
        .await
        .expect("lookup bridged")
        .expect("bridged exists");
    assert_eq!(
        bridged.external_entity_id.as_deref(),
        Some("light.study_desk")
    );
    assert!(home.delete_bridge(&bridge_id).await.expect("delete bridge"));
    assert!(
        home.device_record("ha-light-desk")
            .await
            .expect("cascaded device")
            .is_none()
    );

    assert!(home.delete_rule("rule-custom-1").await.unwrap());
    assert!(home.delete_device(&custom_dev.id).await.unwrap());
    let _ = MediaPlayerPatchStatus::Playing;
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_assistant_conversations_runs_approvals_memories_and_duplicates() {
    let h = Harness::new().await;
    let user =
        h.db.users()
            .create("alice", "pw", Role::Admin)
            .await
            .expect("create user");
    let assistant = h.db.assistant();

    let conv = assistant
        .active_conversation(user.id)
        .await
        .expect("active conversation");
    assert!(conv.messages.is_empty());
    assert!(conv.runs.is_empty());

    let now = clock::now();
    let user_msg = AssistantMessage {
        id: palveni_core::JobId::generate().to_string(),
        role: MessageRole::User,
        text: "检查重复文件".into(),
        created_at: now,
        run_id: None,
    };
    let run_id = palveni_core::JobId::generate().to_string();
    let reply_msg = AssistantMessage {
        id: palveni_core::JobId::generate().to_string(),
        role: MessageRole::Assistant,
        text: "发现 1 组重复文件，请确认。".into(),
        created_at: now,
        run_id: Some(run_id.clone()),
    };
    let run = AssistantRun {
        id: run_id.clone(),
        title: Some("检查重复文件".into()),
        subject: None,
        request: Some("清理 1 个重复条目".into()),
        previews: None,
        state: RunState::AwaitingApproval,
        steps: vec![RunStep {
            id: "s1".into(),
            text: "已扫描媒体库".into(),
            detail: None,
            state: RunStepState::Done,
        }],
        blocks: vec![
            Block::ChangePreview {
                id: "prev-1".into(),
                title: Some("重复项".into()),
                summary: None,
                rows: vec![ChangeRow {
                    id: "r0".into(),
                    before: "song (1).flac".into(),
                    after: "移除重复索引".into(),
                    detail: None,
                    selected: true,
                    group: None,
                    size_bytes: Some(1024),
                    thumbs: None,
                    relation: None,
                    note: None,
                }],
                groups: None,
                columns: None,
            },
            Block::Approval {
                id: "app-1".into(),
                title: "清理 {count} 项".into(),
                effects: vec![ApprovalEffectItem::Plain("移除重复索引".into())],
                risk: ApprovalRisk::Medium,
                preview_id: Some("prev-1".into()),
                confirm_label: Some("确认".into()),
                confirm_icon: None,
                status: Some(ApprovalStatus::Pending),
            },
        ],
        task_id: None,
        created_at: now,
    };

    assistant
        .save_turn(
            user.id,
            &conv.id,
            &user_msg,
            &reply_msg,
            &run,
            Some(r#"{"kind":"noop"}"#.into()),
        )
        .await
        .expect("save turn");

    // Awaiting-approval run is surfaced in unified task_runs.
    let task_runs = assistant.task_runs(user.id, 50).await.expect("task runs");
    assert_eq!(task_runs.len(), 1);
    assert_eq!(task_runs[0].run.id, run_id);

    // Approve updates state and clears pending_action_json.
    let task_id = palveni_core::JobId::generate().to_string();
    let updated = assistant
        .update_run(
            &run_id,
            RunState::Succeeded,
            &run.steps,
            &run.blocks,
            None,
            Some(task_id.clone()),
        )
        .await
        .expect("update run")
        .expect("run found");
    assert_eq!(updated.run.state, RunState::Succeeded);
    assert!(updated.pending_action_json.is_none());
    assert_eq!(updated.run.task_id.as_deref(), Some(task_id.as_str()));

    let by_task = assistant
        .find_run_by_task_or_id(user.id, &task_id, false)
        .await
        .expect("lookup by task id")
        .expect("found");
    assert_eq!(by_task.run.id, run_id);

    // Reset archives old conversation and starts fresh.
    let fresh = assistant
        .reset_conversation(user.id)
        .await
        .expect("reset conversation");
    assert_ne!(fresh.id, conv.id);
    assert!(fresh.messages.is_empty());

    // Memories CRUD.
    let mem = assistant
        .add_memory(user.id, "偏好原盘高码率版本")
        .await
        .expect("add memory");
    let listed = assistant.memories(user.id).await.expect("list memories");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].content, "偏好原盘高码率版本");
    assert!(
        assistant
            .delete_memory(user.id, &mem.id)
            .await
            .expect("delete memory")
    );
    assert!(assistant.memories(user.id).await.unwrap().is_empty());

    // Overview storage & duplicate detection.
    let lib = new_library(&h.db, "Lossless").await;
    let t1 = track(" Nocturne ", "/media/Lossless/Nocturne.flac", None, 40_000);
    let t2 = track(
        "Nocturne (1)",
        "/media/Lossless/Nocturne (1).flac",
        None,
        40_000,
    );
    h.db.items()
        .apply_scan_batch(lib.id, &[t1, t2], &[])
        .await
        .expect("scan batch");
    let storage =
        h.db.overview()
            .library_storage_bytes()
            .await
            .expect("storage bytes");
    assert_eq!(storage.len(), 1);
    assert_eq!(storage[0].bytes, 80_000);

    let dups =
        h.db.overview()
            .duplicate_groups(10)
            .await
            .expect("duplicate groups");
    assert_eq!(dups.len(), 1);
    assert_eq!(dups[0].extras.len(), 1);

    h.finish().await;
}
