//! Smart home domain types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub id: String,
    pub name: String,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClimateMode {
    Cool,
    Heat,
    Auto,
    Dry,
    Fan,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    Temperature,
    Humidity,
    Contact,
    Motion,
    Pm25,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SensorValue {
    Bool(bool),
    Number(f64),
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorReading {
    pub kind: SensorKind,
    pub value: SensorValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaPlayerStatus {
    Playing,
    Paused,
    Idle,
    Off,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaPlayerPatchStatus {
    Playing,
    Paused,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceState {
    Light {
        on: bool,
        brightness: u8,
        #[serde(rename = "colorTemp", default, skip_serializing_if = "Option::is_none")]
        color_temp: Option<u16>,
    },
    Switch {
        on: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        power: Option<f64>,
    },
    Climate {
        on: bool,
        mode: ClimateMode,
        current: f64,
        target: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        humidity: Option<f64>,
    },
    Sensor {
        readings: Vec<SensorReading>,
    },
    #[serde(rename = "media_player")]
    MediaPlayer {
        status: MediaPlayerStatus,
        #[schema(required = true)]
        title: Option<String>,
        #[schema(required = true)]
        subtitle: Option<String>,
        #[schema(required = true)]
        app: Option<String>,
        volume: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        progress: Option<f64>,
        #[serde(rename = "artworkId", default, skip_serializing_if = "Option::is_none")]
        artwork_id: Option<String>,
    },
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn clamp_percent(value: f64) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, 100.0) as u8
}

impl DeviceState {
    pub const fn kind_str(&self) -> &'static str {
        match self {
            DeviceState::Light { .. } => "light",
            DeviceState::Switch { .. } => "switch",
            DeviceState::Climate { .. } => "climate",
            DeviceState::Sensor { .. } => "sensor",
            DeviceState::MediaPlayer { .. } => "media_player",
        }
    }

    pub fn apply_patch(&mut self, patch: &DevicePatch) {
        match self {
            DeviceState::Light { on, brightness, .. } => {
                if let Some(next_on) = patch.on {
                    *on = next_on;
                }
                if let Some(next_b) = patch.brightness {
                    let clamped = clamp_percent(next_b);
                    *brightness = clamped;
                    if patch.on.is_none() && clamped > 0 {
                        *on = true;
                    }
                }
            }
            DeviceState::Switch { on, .. } => {
                if let Some(next_on) = patch.on {
                    *on = next_on;
                }
            }
            DeviceState::Climate { on, target, .. } => {
                if let Some(next_on) = patch.on {
                    *on = next_on;
                }
                if let Some(next_target) = patch.target
                    && next_target.is_finite()
                {
                    *target = (next_target.clamp(16.0, 30.0) * 2.0).round() / 2.0;
                }
            }
            DeviceState::Sensor { .. } => {}
            DeviceState::MediaPlayer { status, volume, .. } => {
                if let Some(next_vol) = patch.volume {
                    *volume = clamp_percent(next_vol);
                }
                if let Some(next_status) = patch.status {
                    *status = match next_status {
                        MediaPlayerPatchStatus::Playing => MediaPlayerStatus::Playing,
                        MediaPlayerPatchStatus::Paused => MediaPlayerStatus::Paused,
                    };
                } else if let Some(next_on) = patch.on {
                    *status = if next_on {
                        MediaPlayerStatus::Idle
                    } else {
                        MediaPlayerStatus::Off
                    };
                }
            }
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    pub online: bool,
    pub state: DeviceState,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeDevice {
    pub id: String,
    pub room_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    pub online: bool,
    pub state: DeviceState,
}

impl From<HomeDevice> for DeviceInfo {
    fn from(device: HomeDevice) -> Self {
        Self {
            id: device.id,
            name: device.name,
            room: device.room,
            online: device.online,
            state: device.state,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DevicePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<MediaPlayerPatchStatus>,
}

impl DevicePatch {
    pub fn is_empty(&self) -> bool {
        self.on.is_none()
            && self.brightness.is_none()
            && self.target.is_none()
            && self.volume.is_none()
            && self.status.is_none()
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneIcon {
    Movie,
    Home,
    Night,
    Morning,
    Away,
}

impl SceneIcon {
    pub const fn as_str(self) -> &'static str {
        match self {
            SceneIcon::Movie => "movie",
            SceneIcon::Home => "home",
            SceneIcon::Night => "night",
            SceneIcon::Morning => "morning",
            SceneIcon::Away => "away",
        }
    }

    pub fn from_name(raw: &str) -> Self {
        match raw {
            "movie" => SceneIcon::Movie,
            "night" => SceneIcon::Night,
            "morning" => SceneIcon::Morning,
            "away" => SceneIcon::Away,
            _ => SceneIcon::Home,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub icon: SceneIcon,
    pub active: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRule {
    pub id: String,
    pub trigger: String,
    pub action: String,
    pub enabled: bool,
    #[schema(required = true)]
    pub last_run_at: Option<DateTime<Utc>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSnapshot {
    pub rooms: Vec<Room>,
    pub devices: Vec<HomeDevice>,
    pub scenes: Vec<Scene>,
    pub rules: Vec<AutomationRule>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDeviceAction {
    pub device_id: String,
    pub patch: DevicePatch,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeBridgeSummary {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
    pub token_configured: bool,
    #[schema(required = true)]
    pub last_sync_at: Option<DateTime<Utc>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeBridgeList {
    pub bridges: Vec<HomeBridgeSummary>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveHomeBridgeRequest {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

const fn default_true() -> bool {
    true
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeBridgeSyncResult {
    pub bridge: HomeBridgeSummary,
    pub synced_devices: usize,
    pub synced_scenes: usize,
    pub snapshot: HomeSnapshot,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeDeviceKind {
    Light,
    Switch,
    Climate,
    Sensor,
    MediaPlayer,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateHomeDeviceRequest {
    pub room_name: String,
    pub name: String,
    pub kind: HomeDeviceKind,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetHomeRuleRequest {
    pub enabled: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateHomeRuleRequest {
    #[serde(default)]
    pub event_kind: Option<String>,
    pub trigger: String,
    pub action: String,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub patch: Option<DevicePatch>,
}
