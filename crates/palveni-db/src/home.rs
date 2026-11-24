//! Smart home rooms, devices, scenes, media-playback rules, and Home Assistant bridges.

use std::collections::HashMap;

use palveni_core::home::{
    AutomationRule, ClimateMode, DevicePatch, DeviceState, HomeBridgeSummary, HomeDevice,
    HomeSnapshot, MediaPlayerPatchStatus, MediaPlayerStatus, PresetDeviceAction, Room, Scene,
    SceneIcon, SensorKind, SensorReading, SensorValue,
};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};

use crate::convert::{now_ms, opt_ts, truncate_chars};
use crate::entities::{
    home_bridges as bridges, home_devices as devices, home_rooms as rooms, home_rules as rules,
    home_scenes as scenes, settings,
};
use crate::{Db, DbError};

const SEEDED_SETTING_KEY: &str = "home.seeded";

pub struct Home<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct HomeBridgeRecord {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub encrypted_token: String,
    pub enabled: bool,
    pub last_sync_at: Option<i64>,
    pub created_at: i64,
}

impl HomeBridgeRecord {
    pub fn to_summary(&self) -> Result<HomeBridgeSummary, DbError> {
        Ok(HomeBridgeSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            enabled: self.enabled,
            token_configured: !self.encrypted_token.is_empty(),
            last_sync_at: opt_ts(self.last_sync_at)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct HomeDeviceRecord {
    pub device: HomeDevice,
    pub bridge_id: Option<String>,
    pub external_entity_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HomeSceneRecord {
    pub scene: Scene,
    pub bridge_id: Option<String>,
    pub external_entity_id: Option<String>,
    pub actions: Vec<PresetDeviceAction>,
}

#[derive(Clone, Debug)]
pub struct HomeRuleRecord {
    pub rule: AutomationRule,
    pub event_kind: String,
    pub actions: Vec<PresetDeviceAction>,
}

#[derive(Clone, Debug)]
pub struct SyncedBridgeDevice {
    pub id: String,
    pub room_name: String,
    pub external_entity_id: String,
    pub name: String,
    pub online: bool,
    pub state: DeviceState,
}

#[derive(Clone, Debug)]
pub struct SyncedBridgeScene {
    pub id: String,
    pub external_entity_id: String,
    pub name: String,
    pub icon: SceneIcon,
}

fn encode_json<T: serde::Serialize>(val: &T) -> Result<String, DbError> {
    serde_json::to_string(val).map_err(|e| DbError::Internal(format!("json encode failed: {e}")))
}

impl Home<'_> {
    /// Seeds initial rooms (`客厅`, `书房`, `主卧`), controllable local devices,
    /// scenes (`movie`, `home`, `night`, `morning`, `away`), and `"影音联动"` rules
    /// on first query when `home_rooms` is empty.
    pub async fn ensure_seeded(&self) -> Result<(), DbError> {
        let count = rooms::Entity::find().count(self.db.main()).await?;
        if count > 0 {
            return Ok(());
        }
        let already = settings::Entity::find_by_id(SEEDED_SETTING_KEY.to_owned())
            .one(self.db.main())
            .await?;
        if already.is_some() {
            return Ok(());
        }

        let now = now_ms();
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;
        if rooms::Entity::find().count(&txn).await? > 0 {
            txn.rollback().await?;
            return Ok(());
        }

        let seed_rooms = [
            ("living", "客厅", 0),
            ("study", "书房", 1),
            ("bedroom", "主卧", 2),
        ];
        rooms::Entity::insert_many(
            seed_rooms
                .iter()
                .map(|(id, name, order)| rooms::ActiveModel {
                    id: Set((*id).to_owned()),
                    name: Set((*name).to_owned()),
                    sort_order: Set(*order),
                    created_at: Set(now),
                }),
        )
        .exec_without_returning(&txn)
        .await?;

        let seed_devices: Vec<(&str, &str, &str, bool, DeviceState, i32)> = vec![
            (
                "living-main",
                "living",
                "客厅主灯",
                true,
                DeviceState::Light {
                    on: true,
                    brightness: 70,
                    color_temp: Some(4000),
                },
                0,
            ),
            (
                "living-strip",
                "living",
                "氛围灯带",
                true,
                DeviceState::Light {
                    on: true,
                    brightness: 35,
                    color_temp: Some(2700),
                },
                1,
            ),
            (
                "living-tv",
                "living",
                "客厅电视",
                true,
                DeviceState::MediaPlayer {
                    status: MediaPlayerStatus::Idle,
                    title: Some("Palveni 家庭影院".into()),
                    subtitle: Some("待机中".into()),
                    app: Some("Viozi".into()),
                    volume: 36,
                    progress: None,
                    artwork_id: None,
                },
                2,
            ),
            (
                "living-ac",
                "living",
                "客厅空调",
                true,
                DeviceState::Climate {
                    on: true,
                    mode: ClimateMode::Cool,
                    current: 25.5,
                    target: 24.0,
                    humidity: Some(52.0),
                },
                3,
            ),
            (
                "living-sensor",
                "living",
                "温湿度计",
                true,
                DeviceState::Sensor {
                    readings: vec![
                        SensorReading {
                            kind: SensorKind::Temperature,
                            value: SensorValue::Number(25.5),
                            unit: Some("°C".into()),
                        },
                        SensorReading {
                            kind: SensorKind::Humidity,
                            value: SensorValue::Number(52.0),
                            unit: Some("%".into()),
                        },
                    ],
                },
                4,
            ),
            (
                "study-lamp",
                "study",
                "书房台灯",
                true,
                DeviceState::Light {
                    on: true,
                    brightness: 80,
                    color_temp: Some(4500),
                },
                0,
            ),
            (
                "study-plug",
                "study",
                "桌面音箱插座",
                true,
                DeviceState::Switch {
                    on: true,
                    power: Some(38.0),
                },
                1,
            ),
            (
                "study-air",
                "study",
                "空气质量",
                true,
                DeviceState::Sensor {
                    readings: vec![
                        SensorReading {
                            kind: SensorKind::Pm25,
                            value: SensorValue::Number(11.0),
                            unit: Some("µg/m³".into()),
                        },
                        SensorReading {
                            kind: SensorKind::Temperature,
                            value: SensorValue::Number(24.2),
                            unit: Some("°C".into()),
                        },
                    ],
                },
                2,
            ),
            (
                "bed-lamp",
                "bedroom",
                "床头灯",
                true,
                DeviceState::Light {
                    on: false,
                    brightness: 35,
                    color_temp: Some(2700),
                },
                0,
            ),
            (
                "bed-humidifier",
                "bedroom",
                "静音加湿器",
                true,
                DeviceState::Switch {
                    on: false,
                    power: None,
                },
                1,
            ),
            (
                "bed-window",
                "bedroom",
                "窗户传感器",
                true,
                DeviceState::Sensor {
                    readings: vec![SensorReading {
                        kind: SensorKind::Contact,
                        value: SensorValue::Bool(false),
                        unit: None,
                    }],
                },
                2,
            ),
        ];

        let mut device_models = Vec::with_capacity(seed_devices.len());
        for (id, room_id, name, online, state, order) in &seed_devices {
            device_models.push(devices::ActiveModel {
                id: Set((*id).to_owned()),
                room_id: Set((*room_id).to_owned()),
                bridge_id: Set(None),
                external_entity_id: Set(None),
                name: Set((*name).to_owned()),
                kind: Set(state.kind_str().to_owned()),
                online: Set(*online),
                state_json: Set(encode_json(state)?),
                sort_order: Set(*order),
                updated_at: Set(now),
            });
        }
        devices::Entity::insert_many(device_models)
            .exec_without_returning(&txn)
            .await?;

        let patch_on = |on: bool| DevicePatch {
            on: Some(on),
            ..Default::default()
        };
        let patch_light = |on: bool, brightness: f64| DevicePatch {
            on: Some(on),
            brightness: Some(brightness),
            ..Default::default()
        };

        struct SeedScene {
            id: &'static str,
            name: &'static str,
            icon: SceneIcon,
            active: bool,
            actions: Vec<PresetDeviceAction>,
            order: i32,
        }

        let seed_scenes = vec![
            SeedScene {
                id: "movie",
                name: "观影模式",
                icon: SceneIcon::Movie,
                active: false,
                actions: vec![
                    PresetDeviceAction {
                        device_id: "living-main".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "living-strip".into(),
                        patch: patch_light(true, 15.0),
                    },
                    PresetDeviceAction {
                        device_id: "living-tv".into(),
                        patch: DevicePatch {
                            status: Some(MediaPlayerPatchStatus::Playing),
                            ..Default::default()
                        },
                    },
                ],
                order: 0,
            },
            SeedScene {
                id: "home",
                name: "回家",
                icon: SceneIcon::Home,
                active: true,
                actions: vec![
                    PresetDeviceAction {
                        device_id: "living-main".into(),
                        patch: patch_light(true, 75.0),
                    },
                    PresetDeviceAction {
                        device_id: "living-strip".into(),
                        patch: patch_light(true, 40.0),
                    },
                    PresetDeviceAction {
                        device_id: "living-ac".into(),
                        patch: patch_on(true),
                    },
                ],
                order: 1,
            },
            SeedScene {
                id: "night",
                name: "晚安",
                icon: SceneIcon::Night,
                active: false,
                actions: vec![
                    PresetDeviceAction {
                        device_id: "living-main".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "living-strip".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "study-lamp".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "bed-lamp".into(),
                        patch: patch_on(false),
                    },
                ],
                order: 2,
            },
            SeedScene {
                id: "morning",
                name: "早安",
                icon: SceneIcon::Morning,
                active: false,
                actions: vec![
                    PresetDeviceAction {
                        device_id: "living-main".into(),
                        patch: patch_light(true, 80.0),
                    },
                    PresetDeviceAction {
                        device_id: "bed-lamp".into(),
                        patch: patch_light(true, 50.0),
                    },
                ],
                order: 3,
            },
            SeedScene {
                id: "away",
                name: "离家",
                icon: SceneIcon::Away,
                active: false,
                actions: vec![
                    PresetDeviceAction {
                        device_id: "living-main".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "living-strip".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "study-lamp".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "living-ac".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "bed-humidifier".into(),
                        patch: patch_on(false),
                    },
                ],
                order: 4,
            },
        ];

        let mut scene_models = Vec::with_capacity(seed_scenes.len());
        for sc in &seed_scenes {
            scene_models.push(scenes::ActiveModel {
                id: Set(sc.id.to_owned()),
                bridge_id: Set(None),
                external_entity_id: Set(None),
                name: Set(sc.name.to_owned()),
                icon: Set(sc.icon.as_str().to_owned()),
                active: Set(sc.active),
                actions_json: Set(encode_json(&sc.actions)?),
                sort_order: Set(sc.order),
            });
        }
        scenes::Entity::insert_many(scene_models)
            .exec_without_returning(&txn)
            .await?;

        struct SeedRule {
            id: &'static str,
            event_kind: &'static str,
            trigger: &'static str,
            action: &'static str,
            actions: Vec<PresetDeviceAction>,
            enabled: bool,
            order: i32,
        }

        let seed_rules = vec![
            SeedRule {
                id: "rule-movie-dim",
                event_kind: "video.play",
                trigger: "开始播放电影时调暗客厅灯光",
                action: "客厅主灯关闭，氛围灯带调至 15%",
                actions: vec![
                    PresetDeviceAction {
                        device_id: "living-main".into(),
                        patch: patch_on(false),
                    },
                    PresetDeviceAction {
                        device_id: "living-strip".into(),
                        patch: patch_light(true, 15.0),
                    },
                    PresetDeviceAction {
                        device_id: "living-tv".into(),
                        patch: DevicePatch {
                            status: Some(MediaPlayerPatchStatus::Playing),
                            ..Default::default()
                        },
                    },
                ],
                enabled: true,
                order: 0,
            },
            SeedRule {
                id: "rule-pause-glow",
                event_kind: "video.pause",
                trigger: "暂停播放时微亮客厅灯光",
                action: "氛围灯带恢复至 45%",
                actions: vec![PresetDeviceAction {
                    device_id: "living-strip".into(),
                    patch: patch_light(true, 45.0),
                }],
                enabled: true,
                order: 1,
            },
            SeedRule {
                id: "rule-night-volume",
                event_kind: "night.play",
                trigger: "夜间 23:30 后自动调低音量",
                action: "客厅电视音量限制在 22%",
                actions: vec![PresetDeviceAction {
                    device_id: "living-tv".into(),
                    patch: DevicePatch {
                        volume: Some(22.0),
                        ..Default::default()
                    },
                }],
                enabled: true,
                order: 2,
            },
            SeedRule {
                id: "rule-music-study",
                event_kind: "music.play",
                trigger: "播放音乐时联动书房氛围",
                action: "桌面音箱插座开启，书房台灯调至 60%",
                actions: vec![
                    PresetDeviceAction {
                        device_id: "study-plug".into(),
                        patch: patch_on(true),
                    },
                    PresetDeviceAction {
                        device_id: "study-lamp".into(),
                        patch: patch_light(true, 60.0),
                    },
                ],
                enabled: true,
                order: 3,
            },
        ];

        let mut rule_models = Vec::with_capacity(seed_rules.len());
        for r in &seed_rules {
            rule_models.push(rules::ActiveModel {
                id: Set(r.id.to_owned()),
                event_kind: Set(r.event_kind.to_owned()),
                trigger_text: Set(r.trigger.to_owned()),
                action_text: Set(r.action.to_owned()),
                actions_json: Set(encode_json(&r.actions)?),
                enabled: Set(r.enabled),
                last_run_at: Set(None),
                sort_order: Set(r.order),
            });
        }
        rules::Entity::insert_many(rule_models)
            .exec_without_returning(&txn)
            .await?;

        settings::Entity::insert(settings::ActiveModel {
            setting_key: Set(SEEDED_SETTING_KEY.to_owned()),
            setting_value: Set("1".to_owned()),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::column(settings::Column::SettingKey)
                .update_columns([settings::Column::SettingValue, settings::Column::UpdatedAt])
                .to_owned(),
        )
        .exec_without_returning(&txn)
        .await?;

        txn.commit().await?;
        Ok(())
    }

    pub async fn snapshot(&self) -> Result<HomeSnapshot, DbError> {
        self.ensure_seeded().await?;

        let room_rows = rooms::Entity::find()
            .order_by_asc(rooms::Column::SortOrder)
            .order_by_asc(rooms::Column::Id)
            .all(self.db.main())
            .await?;
        let room_names: HashMap<String, String> = room_rows
            .iter()
            .map(|r| (r.id.clone(), r.name.clone()))
            .collect();
        let rooms_out: Vec<Room> = room_rows
            .into_iter()
            .map(|r| Room {
                id: r.id,
                name: r.name,
            })
            .collect();

        let device_rows = devices::Entity::find()
            .order_by_asc(devices::Column::SortOrder)
            .order_by_asc(devices::Column::Id)
            .all(self.db.main())
            .await?;
        let mut devices_out = Vec::with_capacity(device_rows.len());
        for row in device_rows {
            let state: DeviceState = serde_json::from_str(&row.state_json).map_err(|e| {
                DbError::Corrupt(format!("corrupt home device `{}` state: {e}", row.id))
            })?;
            let room = room_names.get(&row.room_id).cloned();
            devices_out.push(HomeDevice {
                id: row.id,
                room_id: row.room_id,
                name: row.name,
                room,
                online: row.online,
                state,
            });
        }

        let scene_rows = scenes::Entity::find()
            .order_by_asc(scenes::Column::SortOrder)
            .order_by_asc(scenes::Column::Id)
            .all(self.db.main())
            .await?;
        let scenes_out: Vec<Scene> = scene_rows
            .into_iter()
            .map(|row| Scene {
                id: row.id,
                name: row.name,
                icon: SceneIcon::from_name(&row.icon),
                active: row.active,
            })
            .collect();

        let rule_rows = rules::Entity::find()
            .order_by_asc(rules::Column::SortOrder)
            .order_by_asc(rules::Column::Id)
            .all(self.db.main())
            .await?;
        let mut rules_out = Vec::with_capacity(rule_rows.len());
        for row in rule_rows {
            rules_out.push(AutomationRule {
                id: row.id,
                trigger: row.trigger_text,
                action: row.action_text,
                enabled: row.enabled,
                last_run_at: opt_ts(row.last_run_at)?,
            });
        }

        Ok(HomeSnapshot {
            rooms: rooms_out,
            devices: devices_out,
            scenes: scenes_out,
            rules: rules_out,
        })
    }

    pub async fn device_record(&self, id: &str) -> Result<Option<HomeDeviceRecord>, DbError> {
        self.ensure_seeded().await?;
        let Some(row) = devices::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let room_name = rooms::Entity::find_by_id(row.room_id.clone())
            .one(self.db.main())
            .await?
            .map(|r| r.name);
        let state: DeviceState = serde_json::from_str(&row.state_json)
            .map_err(|e| DbError::Corrupt(format!("corrupt home device `{}`: {e}", row.id)))?;
        Ok(Some(HomeDeviceRecord {
            device: HomeDevice {
                id: row.id,
                room_id: row.room_id,
                name: row.name,
                room: room_name,
                online: row.online,
                state,
            },
            bridge_id: row.bridge_id,
            external_entity_id: row.external_entity_id,
        }))
    }

    pub async fn apply_device_patch(
        &self,
        id: &str,
        patch: &DevicePatch,
    ) -> Result<Option<HomeDeviceRecord>, DbError> {
        let Some(mut record) = self.device_record(id).await? else {
            return Ok(None);
        };
        if !record.device.online {
            return Ok(Some(record));
        }
        record.device.state.apply_patch(patch);
        let state_json = encode_json(&record.device.state)?;
        devices::Entity::update_many()
            .col_expr(devices::Column::StateJson, Expr::val(state_json))
            .col_expr(devices::Column::UpdatedAt, Expr::val(now_ms()))
            .filter(devices::Column::Id.eq(id.to_owned()))
            .exec(self.db.main())
            .await?;
        Ok(Some(record))
    }

    /// Updates a light device's ambient brightness and color temperature (Kelvin).
    /// Returns `Ok(None)` when the device does not exist, is offline, or is not a light.
    pub async fn apply_ambient_light(
        &self,
        id: &str,
        brightness: u8,
        color_temp: u16,
    ) -> Result<Option<HomeDeviceRecord>, DbError> {
        let Some(mut record) = self.device_record(id).await? else {
            return Ok(None);
        };
        if !record.device.online {
            return Ok(None);
        }
        match &mut record.device.state {
            DeviceState::Light {
                on,
                brightness: current_brightness,
                color_temp: current_temp,
            } => {
                *on = true;
                *current_brightness = brightness.clamp(1, 100);
                *current_temp = Some(color_temp.clamp(2700, 6500));
            }
            _ => return Ok(None),
        }
        let state_json = encode_json(&record.device.state)?;
        devices::Entity::update_many()
            .col_expr(devices::Column::StateJson, Expr::val(state_json))
            .col_expr(devices::Column::UpdatedAt, Expr::val(now_ms()))
            .filter(devices::Column::Id.eq(id.to_owned()))
            .exec(self.db.main())
            .await?;
        Ok(Some(record))
    }

    pub async fn create_device(
        &self,
        id: String,
        room_name: &str,
        name: &str,
        state: DeviceState,
    ) -> Result<HomeDevice, DbError> {
        self.ensure_seeded().await?;
        let trimmed_room = truncate_chars(room_name.trim(), 100);
        let trimmed_name = truncate_chars(name.trim(), 128);
        let now = now_ms();

        let existing_rooms = rooms::Entity::find()
            .order_by_asc(rooms::Column::SortOrder)
            .all(self.db.main())
            .await?;
        let room_id = if let Some(found) = existing_rooms
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(&trimmed_room) || r.name == trimmed_room)
        {
            found.id.clone()
        } else {
            let new_room_id = format!("room-{}", &crate::convert::sha256_hex(&trimmed_room)[..12]);
            let next_order = existing_rooms
                .last()
                .map_or(0, |r| r.sort_order.saturating_add(1));
            rooms::Entity::insert(rooms::ActiveModel {
                id: Set(new_room_id.clone()),
                name: Set(trimmed_room.clone()),
                sort_order: Set(next_order),
                created_at: Set(now),
            })
            .on_conflict(
                OnConflict::column(rooms::Column::Id)
                    .update_column(rooms::Column::Name)
                    .to_owned(),
            )
            .exec_without_returning(self.db.main())
            .await?;
            new_room_id
        };

        let dev_count = devices::Entity::find()
            .filter(devices::Column::RoomId.eq(room_id.clone()))
            .count(self.db.main())
            .await?;
        let order = i32::try_from(dev_count).unwrap_or(100);
        devices::Entity::insert(devices::ActiveModel {
            id: Set(id.clone()),
            room_id: Set(room_id.clone()),
            bridge_id: Set(None),
            external_entity_id: Set(None),
            name: Set(trimmed_name.clone()),
            kind: Set(state.kind_str().to_owned()),
            online: Set(true),
            state_json: Set(encode_json(&state)?),
            sort_order: Set(order),
            updated_at: Set(now),
        })
        .exec_without_returning(self.db.main())
        .await?;

        Ok(HomeDevice {
            id,
            room_id,
            name: trimmed_name,
            room: Some(trimmed_room),
            online: true,
            state,
        })
    }

    pub async fn delete_device(&self, id: &str) -> Result<bool, DbError> {
        let res = devices::Entity::delete_by_id(id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn scene_record(&self, id: &str) -> Result<Option<HomeSceneRecord>, DbError> {
        self.ensure_seeded().await?;
        let Some(row) = scenes::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let actions: Vec<PresetDeviceAction> =
            serde_json::from_str(&row.actions_json).unwrap_or_default();
        Ok(Some(HomeSceneRecord {
            scene: Scene {
                id: row.id,
                name: row.name,
                icon: SceneIcon::from_name(&row.icon),
                active: row.active,
            },
            bridge_id: row.bridge_id,
            external_entity_id: row.external_entity_id,
            actions,
        }))
    }

    pub async fn set_active_scene(&self, id: &str) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;
        let exists = scenes::Entity::find_by_id(id.to_owned()).one(&txn).await?;
        if exists.is_none() {
            txn.rollback().await?;
            return Ok(false);
        }
        scenes::Entity::update_many()
            .col_expr(scenes::Column::Active, Expr::val(false))
            .exec(&txn)
            .await?;
        scenes::Entity::update_many()
            .col_expr(scenes::Column::Active, Expr::val(true))
            .filter(scenes::Column::Id.eq(id.to_owned()))
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(true)
    }

    pub async fn set_rule_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<Option<AutomationRule>, DbError> {
        self.ensure_seeded().await?;
        let Some(row) = rules::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        rules::Entity::update_many()
            .col_expr(rules::Column::Enabled, Expr::val(enabled))
            .filter(rules::Column::Id.eq(id.to_owned()))
            .exec(self.db.main())
            .await?;
        Ok(Some(AutomationRule {
            id: row.id,
            trigger: row.trigger_text,
            action: row.action_text,
            enabled,
            last_run_at: opt_ts(row.last_run_at)?,
        }))
    }

    pub async fn create_rule(
        &self,
        id: String,
        event_kind: &str,
        trigger: &str,
        action: &str,
        actions: &[PresetDeviceAction],
    ) -> Result<AutomationRule, DbError> {
        self.ensure_seeded().await?;
        let count = rules::Entity::find().count(self.db.main()).await?;
        let order = i32::try_from(count).unwrap_or(100);
        let trigger_clean = truncate_chars(trigger.trim(), 255);
        let action_clean = truncate_chars(action.trim(), 255);
        let event_clean = truncate_chars(event_kind.trim(), 64);

        rules::Entity::insert(rules::ActiveModel {
            id: Set(id.clone()),
            event_kind: Set(event_clean),
            trigger_text: Set(trigger_clean.clone()),
            action_text: Set(action_clean.clone()),
            actions_json: Set(encode_json(&actions)?),
            enabled: Set(true),
            last_run_at: Set(None),
            sort_order: Set(order),
        })
        .exec_without_returning(self.db.main())
        .await?;

        Ok(AutomationRule {
            id,
            trigger: trigger_clean,
            action: action_clean,
            enabled: true,
            last_run_at: None,
        })
    }

    pub async fn delete_rule(&self, id: &str) -> Result<bool, DbError> {
        let res = rules::Entity::delete_by_id(id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn enabled_rules_for_events(
        &self,
        event_kinds: &[&str],
    ) -> Result<Vec<HomeRuleRecord>, DbError> {
        self.ensure_seeded().await?;
        let kinds: Vec<String> = event_kinds.iter().map(|s| (*s).to_owned()).collect();
        let rows = rules::Entity::find()
            .filter(rules::Column::Enabled.eq(true))
            .filter(rules::Column::EventKind.is_in(kinds))
            .order_by_asc(rules::Column::SortOrder)
            .all(self.db.main())
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let actions: Vec<PresetDeviceAction> =
                serde_json::from_str(&row.actions_json).unwrap_or_default();
            out.push(HomeRuleRecord {
                rule: AutomationRule {
                    id: row.id,
                    trigger: row.trigger_text,
                    action: row.action_text,
                    enabled: row.enabled,
                    last_run_at: opt_ts(row.last_run_at)?,
                },
                event_kind: row.event_kind,
                actions,
            });
        }
        Ok(out)
    }

    pub async fn mark_rule_ran(&self, id: &str, at_ms: i64) -> Result<(), DbError> {
        rules::Entity::update_many()
            .col_expr(rules::Column::LastRunAt, Expr::val(Some(at_ms)))
            .filter(rules::Column::Id.eq(id.to_owned()))
            .exec(self.db.main())
            .await?;
        Ok(())
    }

    pub async fn bridges(&self) -> Result<Vec<HomeBridgeRecord>, DbError> {
        let rows = bridges::Entity::find()
            .order_by_asc(bridges::Column::CreatedAt)
            .order_by_asc(bridges::Column::Id)
            .all(self.db.main())
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| HomeBridgeRecord {
                id: r.id,
                name: r.name,
                base_url: r.base_url,
                encrypted_token: r.encrypted_token,
                enabled: r.enabled,
                last_sync_at: r.last_sync_at,
                created_at: r.created_at,
            })
            .collect())
    }

    pub async fn bridge(&self, id: &str) -> Result<Option<HomeBridgeRecord>, DbError> {
        Ok(bridges::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
            .map(|r| HomeBridgeRecord {
                id: r.id,
                name: r.name,
                base_url: r.base_url,
                encrypted_token: r.encrypted_token,
                enabled: r.enabled,
                last_sync_at: r.last_sync_at,
                created_at: r.created_at,
            }))
    }

    pub async fn save_bridge(&self, record: HomeBridgeRecord) -> Result<(), DbError> {
        bridges::Entity::insert(bridges::ActiveModel {
            id: Set(record.id),
            name: Set(truncate_chars(&record.name, 100)),
            base_url: Set(record.base_url),
            encrypted_token: Set(record.encrypted_token),
            enabled: Set(record.enabled),
            last_sync_at: Set(record.last_sync_at),
            created_at: Set(record.created_at),
        })
        .on_conflict(
            OnConflict::column(bridges::Column::Id)
                .update_columns([
                    bridges::Column::Name,
                    bridges::Column::BaseUrl,
                    bridges::Column::EncryptedToken,
                    bridges::Column::Enabled,
                    bridges::Column::LastSyncAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(self.db.main())
        .await?;
        Ok(())
    }

    pub async fn delete_bridge(&self, id: &str) -> Result<bool, DbError> {
        let res = bridges::Entity::delete_by_id(id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn apply_bridge_sync(
        &self,
        bridge_id: &str,
        synced_devices: &[SyncedBridgeDevice],
        synced_scenes: &[SyncedBridgeScene],
    ) -> Result<(), DbError> {
        self.ensure_seeded().await?;
        let now = now_ms();
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let mut existing_rooms: HashMap<String, String> = rooms::Entity::find()
            .all(&txn)
            .await?
            .into_iter()
            .map(|r| (r.name, r.id))
            .collect();
        let mut next_room_order = i32::try_from(existing_rooms.len()).unwrap_or(10);

        for (index, dev) in synced_devices.iter().enumerate() {
            let room_label = truncate_chars(dev.room_name.trim(), 100);
            let room_id = if let Some(id) = existing_rooms.get(&room_label) {
                id.clone()
            } else {
                let new_id = format!("room-{}", &crate::convert::sha256_hex(&room_label)[..12]);
                rooms::Entity::insert(rooms::ActiveModel {
                    id: Set(new_id.clone()),
                    name: Set(room_label.clone()),
                    sort_order: Set(next_room_order),
                    created_at: Set(now),
                })
                .on_conflict(
                    OnConflict::column(rooms::Column::Id)
                        .update_column(rooms::Column::Name)
                        .to_owned(),
                )
                .exec_without_returning(&txn)
                .await?;
                next_room_order = next_room_order.saturating_add(1);
                existing_rooms.insert(room_label, new_id.clone());
                new_id
            };

            devices::Entity::insert(devices::ActiveModel {
                id: Set(dev.id.clone()),
                room_id: Set(room_id),
                bridge_id: Set(Some(bridge_id.to_owned())),
                external_entity_id: Set(Some(dev.external_entity_id.clone())),
                name: Set(truncate_chars(&dev.name, 128)),
                kind: Set(dev.state.kind_str().to_owned()),
                online: Set(dev.online),
                state_json: Set(encode_json(&dev.state)?),
                sort_order: Set(i32::try_from(index).unwrap_or(100)),
                updated_at: Set(now),
            })
            .on_conflict(
                OnConflict::column(devices::Column::Id)
                    .update_columns([
                        devices::Column::RoomId,
                        devices::Column::BridgeId,
                        devices::Column::ExternalEntityId,
                        devices::Column::Name,
                        devices::Column::Kind,
                        devices::Column::Online,
                        devices::Column::StateJson,
                        devices::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&txn)
            .await?;
        }

        for (index, sc) in synced_scenes.iter().enumerate() {
            scenes::Entity::insert(scenes::ActiveModel {
                id: Set(sc.id.clone()),
                bridge_id: Set(Some(bridge_id.to_owned())),
                external_entity_id: Set(Some(sc.external_entity_id.clone())),
                name: Set(truncate_chars(&sc.name, 100)),
                icon: Set(sc.icon.as_str().to_owned()),
                active: Set(false),
                actions_json: Set("[]".to_owned()),
                sort_order: Set(100_i32.saturating_add(i32::try_from(index).unwrap_or(0))),
            })
            .on_conflict(
                OnConflict::column(scenes::Column::Id)
                    .update_columns([
                        scenes::Column::BridgeId,
                        scenes::Column::ExternalEntityId,
                        scenes::Column::Name,
                        scenes::Column::Icon,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&txn)
            .await?;
        }

        bridges::Entity::update_many()
            .col_expr(bridges::Column::LastSyncAt, Expr::val(Some(now)))
            .filter(bridges::Column::Id.eq(bridge_id.to_owned()))
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }
}
