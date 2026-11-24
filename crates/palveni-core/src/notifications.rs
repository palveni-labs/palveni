//! Domain models and deterministic helpers for Notification Center, Scheduled Routines,
//! and Inbound Media Automation Webhooks (`modules: notifications, routines`).

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::ids::{LibraryId, UserId};

fn default_true() -> bool {
    true
}

/// Supported external notification delivery providers.
#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannelKind {
    Webhook,
    Telegram,
    Bark,
    Ntfy,
    Wecom,
    Feishu,
    Discord,
}

impl NotificationChannelKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Webhook => "webhook",
            Self::Telegram => "telegram",
            Self::Bark => "bark",
            Self::Ntfy => "ntfy",
            Self::Wecom => "wecom",
            Self::Feishu => "feishu",
            Self::Discord => "discord",
        }
    }
}

impl fmt::Display for NotificationChannelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NotificationChannelKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "webhook" => Ok(Self::Webhook),
            "telegram" => Ok(Self::Telegram),
            "bark" => Ok(Self::Bark),
            "ntfy" => Ok(Self::Ntfy),
            "wecom" => Ok(Self::Wecom),
            "feishu" => Ok(Self::Feishu),
            "discord" => Ok(Self::Discord),
            other => Err(Error::invalid(format!(
                "unknown notification channel kind `{other}`"
            ))),
        }
    }
}

/// Secret channel configuration encrypted at rest via `AES-256-GCM`.
#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotificationChannelConfig {
    /// Endpoint URL (Webhook / Bark / ntfy / WeCom / Feishu / Discord, or optional Telegram API base).
    #[serde(default)]
    pub url: Option<String>,
    /// Provider secret token (Telegram bot token, ntfy bearer token, Bark device key, or Webhook secret).
    #[serde(default)]
    pub token: Option<String>,
    /// Provider target (Telegram `chat_id`, ntfy topic, etc.).
    #[serde(default)]
    pub target: Option<String>,
}

impl NotificationChannelConfig {
    /// Produces a non-secret display summary (host or target ID) for UI listing.
    pub fn redacted_hint(&self) -> Option<String> {
        if let Some(target) = self
            .target
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return Some(target.to_owned());
        }
        if let Some(url) = self.url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
            let host = after_scheme.split('/').next().unwrap_or(after_scheme);
            if !host.is_empty() {
                return Some(host.to_owned());
            }
        }
        None
    }

    pub fn has_secret(&self) -> bool {
        self.token.as_deref().is_some_and(|s| !s.trim().is_empty())
            || self.url.as_deref().is_some_and(|s| !s.trim().is_empty())
    }
}

/// Configured push notification channel (secrets redacted).
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChannel {
    pub id: String,
    pub owner_id: UserId,
    pub name: String,
    pub kind: NotificationChannelKind,
    pub target_hint: Option<String>,
    pub has_secret: bool,
    pub events: Vec<String>,
    pub quiet_hours: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChannelList {
    pub channels: Vec<NotificationChannel>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateNotificationChannelRequest {
    pub name: String,
    pub kind: NotificationChannelKind,
    pub config: NotificationChannelConfig,
    #[serde(default)]
    pub events: Vec<String>,
    pub quiet_hours: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateNotificationChannelRequest {
    pub name: Option<String>,
    pub config: Option<NotificationChannelConfig>,
    pub events: Option<Vec<String>>,
    /// Empty string clears quiet hours; `"HH:MM-HH:MM"` updates it.
    pub quiet_hours: Option<String>,
    pub enabled: Option<bool>,
}

/// Delivery outcome for an in-app or external notification record.
#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryStatus {
    InApp,
    Sent,
    Failed,
    SkippedQuiet,
}

impl NotificationDeliveryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Sent => "sent",
            Self::Failed => "failed",
            Self::SkippedQuiet => "skipped_quiet",
        }
    }
}

impl FromStr for NotificationDeliveryStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "in_app" => Ok(Self::InApp),
            "sent" => Ok(Self::Sent),
            "failed" => Ok(Self::Failed),
            "skipped_quiet" => Ok(Self::SkippedQuiet),
            other => Err(Error::invalid(format!(
                "unknown notification delivery status `{other}`"
            ))),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDelivery {
    pub id: String,
    pub owner_id: Option<UserId>,
    pub channel_id: Option<String>,
    pub event_type: String,
    pub title: String,
    pub body: String,
    pub status: NotificationDeliveryStatus,
    pub error: Option<String>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationList {
    pub notifications: Vec<NotificationDelivery>,
    pub unread_count: u64,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadAllNotificationsResponse {
    pub marked_read: u64,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestChannelResponse {
    pub delivery: NotificationDelivery,
}

/// Supported inbound media automation webhook sources.
#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboundWebhookSourceKind {
    Sonarr,
    Radarr,
    Moviepilot,
    Qbittorrent,
    Custom,
}

impl InboundWebhookSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sonarr => "sonarr",
            Self::Radarr => "radarr",
            Self::Moviepilot => "moviepilot",
            Self::Qbittorrent => "qbittorrent",
            Self::Custom => "custom",
        }
    }
}

impl FromStr for InboundWebhookSourceKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "sonarr" => Ok(Self::Sonarr),
            "radarr" => Ok(Self::Radarr),
            "moviepilot" => Ok(Self::Moviepilot),
            "qbittorrent" => Ok(Self::Qbittorrent),
            "custom" => Ok(Self::Custom),
            other => Err(Error::invalid(format!(
                "unknown inbound webhook source `{other}`"
            ))),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundWebhook {
    pub id: String,
    pub name: String,
    pub source_kind: InboundWebhookSourceKind,
    pub library_id: Option<LibraryId>,
    pub enabled: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundWebhookList {
    pub webhooks: Vec<InboundWebhook>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInboundWebhookRequest {
    pub name: String,
    pub source_kind: InboundWebhookSourceKind,
    pub library_id: Option<LibraryId>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedInboundWebhook {
    pub webhook: InboundWebhook,
    pub token: String,
    pub trigger_path: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundWebhookTriggerResponse {
    pub accepted: bool,
    pub source_kind: InboundWebhookSourceKind,
    pub summary: String,
    pub queued_library_ids: Vec<LibraryId>,
}

/// Open schema describing common fields sent by Sonarr, Radarr, MoviePilot, qBittorrent, or custom callers.
#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundWebhookPayload {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

/// Schedule trigger type for automated routines.
#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineScheduleKind {
    Hourly,
    Daily,
    Weekly,
    OnEvent,
}

impl RoutineScheduleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::OnEvent => "on_event",
        }
    }
}

impl FromStr for RoutineScheduleKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "hourly" => Ok(Self::Hourly),
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "on_event" => Ok(Self::OnEvent),
            other => Err(Error::invalid(format!(
                "unknown routine schedule kind `{other}`"
            ))),
        }
    }
}

/// Action executed when a scheduled routine runs.
#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineActionKind {
    ScanAll,
    ScanLibrary,
    HomeScene,
    AssistantPrompt,
}

impl RoutineActionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScanAll => "scan_all",
            Self::ScanLibrary => "scan_library",
            Self::HomeScene => "home_scene",
            Self::AssistantPrompt => "assistant_prompt",
        }
    }
}

impl FromStr for RoutineActionKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "scan_all" => Ok(Self::ScanAll),
            "scan_library" => Ok(Self::ScanLibrary),
            "home_scene" => Ok(Self::HomeScene),
            "assistant_prompt" => Ok(Self::AssistantPrompt),
            other => Err(Error::invalid(format!(
                "unknown routine action kind `{other}`"
            ))),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRoutine {
    pub id: String,
    pub owner_id: UserId,
    pub name: String,
    pub schedule_kind: RoutineScheduleKind,
    pub schedule_expr: String,
    pub action_kind: RoutineActionKind,
    pub action_payload: String,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRoutineList {
    pub routines: Vec<ScheduledRoutine>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateRoutineRequest {
    pub name: String,
    pub schedule_kind: RoutineScheduleKind,
    pub schedule_expr: String,
    pub action_kind: RoutineActionKind,
    #[serde(default)]
    pub action_payload: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateRoutineRequest {
    pub name: Option<String>,
    pub schedule_kind: Option<RoutineScheduleKind>,
    pub schedule_expr: Option<String>,
    pub action_kind: Option<RoutineActionKind>,
    pub action_payload: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRoutineResponse {
    pub routine: ScheduledRoutine,
    pub detail: String,
}

fn parse_hhmm(raw: &str) -> Option<(u32, u32)> {
    let (h_str, m_str) = raw.trim().split_once(':')?;
    if h_str.len() > 2 || m_str.len() != 2 {
        return None;
    }
    let hour: u32 = h_str.parse().ok()?;
    let minute: u32 = m_str.parse().ok()?;
    if hour < 24 && minute < 60 {
        Some((hour, minute))
    } else {
        None
    }
}

/// Validates and canonicalizes a `HH:MM-HH:MM` quiet-hours window.
pub fn normalize_quiet_hours(raw: &str) -> Option<String> {
    let (start, end) = raw.trim().split_once('-')?;
    let (sh, sm) = parse_hhmm(start)?;
    let (eh, em) = parse_hhmm(end)?;
    if sh == eh && sm == em {
        return None;
    }
    Some(format!("{sh:02}:{sm:02}-{eh:02}:{em:02}"))
}

/// Checks whether `now` (evaluated in `Asia/Shanghai`, UTC+8) falls within `quiet_hours`.
pub fn is_in_quiet_hours(quiet_hours: &str, now: DateTime<Utc>) -> bool {
    let Some((start, end)) = quiet_hours.trim().split_once('-') else {
        return false;
    };
    let (Some((sh, sm)), Some((eh, em))) = (parse_hhmm(start), parse_hhmm(end)) else {
        return false;
    };
    let start_mins = sh * 60 + sm;
    let end_mins = eh * 60 + em;
    if start_mins == end_mins {
        return false;
    }
    let shanghai = now + Duration::hours(8);
    let cur_mins = shanghai.hour() * 60 + shanghai.minute();
    if start_mins < end_mins {
        cur_mins >= start_mins && cur_mins < end_mins
    } else {
        // Overnight range, e.g. 23:00-07:00
        cur_mins >= start_mins || cur_mins < end_mins
    }
}

fn parse_weekday(raw: &str) -> Option<Weekday> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "1" | "MON" | "MONDAY" => Some(Weekday::Mon),
        "2" | "TUE" | "TUESDAY" => Some(Weekday::Tue),
        "3" | "WED" | "WEDNESDAY" => Some(Weekday::Wed),
        "4" | "THU" | "THURSDAY" => Some(Weekday::Thu),
        "5" | "FRI" | "FRIDAY" => Some(Weekday::Fri),
        "6" | "SAT" | "SATURDAY" => Some(Weekday::Sat),
        "7" | "0" | "SUN" | "SUNDAY" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Validates and normalizes `schedule_expr` for the given `schedule_kind`.
pub fn normalize_schedule_expr(kind: RoutineScheduleKind, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    match kind {
        RoutineScheduleKind::Hourly => {
            if trimmed.is_empty() {
                return Some("00".to_owned());
            }
            let minute_str = trimmed.strip_prefix(':').unwrap_or(trimmed);
            let minute: u32 = minute_str.parse().ok()?;
            (minute < 60).then(|| format!("{minute:02}"))
        }
        RoutineScheduleKind::Daily => {
            let (h, m) = parse_hhmm(trimmed)?;
            Some(format!("{h:02}:{m:02}"))
        }
        RoutineScheduleKind::Weekly => {
            let (day_part, time_part) = trimmed.split_once(char::is_whitespace)?;
            let day = parse_weekday(day_part)?;
            let (h, m) = parse_hhmm(time_part.trim())?;
            let day_code = match day {
                Weekday::Mon => "MON",
                Weekday::Tue => "TUE",
                Weekday::Wed => "WED",
                Weekday::Thu => "THU",
                Weekday::Fri => "FRI",
                Weekday::Sat => "SAT",
                Weekday::Sun => "SUN",
            };
            Some(format!("{day_code} {h:02}:{m:02}"))
        }
        RoutineScheduleKind::OnEvent => {
            if trimmed.is_empty()
                || trimmed.len() > 64
                || trimmed.chars().any(|c| c.is_control() || c.is_whitespace())
            {
                return None;
            }
            Some(trimmed.to_owned())
        }
    }
}

/// Returns `true` if a periodic routine (`hourly` / `daily` / `weekly`) is due at `now`
/// (evaluated in `Asia/Shanghai` UTC+8) and has not yet run during this scheduled slot.
pub fn is_periodic_routine_due(
    kind: RoutineScheduleKind,
    expr: &str,
    last_run_ms: Option<i64>,
    now: DateTime<Utc>,
) -> bool {
    let shanghai = now + Duration::hours(8);
    // Check both the current minute and the preceding minute so a 60s ticker with slight jitter
    // never misses a scheduled minute while `last_run_ms >= slot_start_ms` prevents double runs.
    for lookback_mins in [0_i64, 1_i64] {
        let candidate_local = shanghai - Duration::minutes(lookback_mins);
        let matches = match kind {
            RoutineScheduleKind::Hourly => {
                let target_min: u32 = expr
                    .trim()
                    .strip_prefix(':')
                    .unwrap_or(expr.trim())
                    .parse()
                    .unwrap_or(0);
                candidate_local.minute() == target_min
            }
            RoutineScheduleKind::Daily => parse_hhmm(expr)
                .is_some_and(|(h, m)| candidate_local.hour() == h && candidate_local.minute() == m),
            RoutineScheduleKind::Weekly => {
                if let Some((day_part, time_part)) = expr.trim().split_once(char::is_whitespace)
                    && let (Some(day), Some((h, m))) =
                        (parse_weekday(day_part), parse_hhmm(time_part))
                {
                    candidate_local.weekday() == day
                        && candidate_local.hour() == h
                        && candidate_local.minute() == m
                } else {
                    false
                }
            }
            RoutineScheduleKind::OnEvent => false,
        };
        if matches {
            let candidate_utc = candidate_local - Duration::hours(8);
            let slot_start_ms = candidate_utc.timestamp_millis()
                - i64::from(candidate_utc.second()) * 1_000
                - i64::from(candidate_utc.timestamp_subsec_millis());
            if last_run_ms.is_none_or(|last| last < slot_start_ms) {
                return true;
            }
        }
    }
    false
}

/// Checks whether an event subscription pattern (`"*"` or exact/prefix match) matches `event_type`.
pub fn event_matches(pattern: &str, event_type: &str) -> bool {
    let p = pattern.trim();
    if p == "*" || p.eq_ignore_ascii_case("all") {
        return true;
    }
    if let Some(prefix) = p.strip_suffix(".*") {
        return event_type
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('.'));
    }
    p.eq_ignore_ascii_case(event_type)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn quiet_hours_handles_same_day_and_overnight_shanghai_windows() {
        assert_eq!(
            normalize_quiet_hours("23:00-07:00").as_deref(),
            Some("23:00-07:00")
        );
        assert!(normalize_quiet_hours("08:00-08:00").is_none());
        assert!(normalize_quiet_hours("25:00-07:00").is_none());

        // 15:30 UTC == 23:30 Asia/Shanghai -> inside 23:00-07:00
        let night = Utc.with_ymd_and_hms(2026, 11, 23, 15, 30, 0).unwrap();
        assert!(is_in_quiet_hours("23:00-07:00", night));

        // 22:30 UTC == 06:30+1 Asia/Shanghai -> inside 23:00-07:00
        let early_morning = Utc.with_ymd_and_hms(2026, 11, 23, 22, 30, 0).unwrap();
        assert!(is_in_quiet_hours("23:00-07:00", early_morning));

        // 04:00 UTC == 12:00 Asia/Shanghai -> outside 23:00-07:00, inside 09:00-18:00
        let noon = Utc.with_ymd_and_hms(2026, 11, 23, 4, 0, 0).unwrap();
        assert!(!is_in_quiet_hours("23:00-07:00", noon));
        assert!(is_in_quiet_hours("09:00-18:00", noon));
    }

    #[test]
    fn routine_schedule_validation_and_due_evaluation() {
        assert_eq!(
            normalize_schedule_expr(RoutineScheduleKind::Hourly, "15").as_deref(),
            Some("15")
        );
        assert_eq!(
            normalize_schedule_expr(RoutineScheduleKind::Daily, "3:05").as_deref(),
            Some("03:05")
        );
        assert_eq!(
            normalize_schedule_expr(RoutineScheduleKind::Weekly, "mon 04:30").as_deref(),
            Some("MON 04:30")
        );
        assert_eq!(
            normalize_schedule_expr(RoutineScheduleKind::OnEvent, "webhook.triggered").as_deref(),
            Some("webhook.triggered")
        );

        // 2026-11-23 is a Monday. 19:05 UTC on Sunday 2026-11-22 == Monday 2026-11-23 03:05 Asia/Shanghai!
        let mon_0305 = Utc.with_ymd_and_hms(2026, 11, 22, 19, 5, 20).unwrap();
        assert!(is_periodic_routine_due(
            RoutineScheduleKind::Daily,
            "03:05",
            None,
            mon_0305
        ));
        assert!(is_periodic_routine_due(
            RoutineScheduleKind::Weekly,
            "MON 03:05",
            None,
            mon_0305
        ));
        // Once marked ran at 19:05:20, must NOT trigger again at 19:05:50 or 19:06:10!
        let ran_ms = mon_0305.timestamp_millis();
        let mon_0306 = Utc.with_ymd_and_hms(2026, 11, 22, 19, 6, 10).unwrap();
        assert!(!is_periodic_routine_due(
            RoutineScheduleKind::Daily,
            "03:05",
            Some(ran_ms),
            mon_0306
        ));
    }
}
