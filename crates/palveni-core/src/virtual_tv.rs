//! Domain models and deterministic 24x7 clock-synced EPG scheduler for Virtual Live TV (`module: virtualTv`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::clock;
use crate::ids::{ItemId, LibraryId};

/// Default runtime (45 minutes) assigned to unprobed video items so every indexed video schedules cleanly.
pub const DEFAULT_VIRTUAL_RUNTIME_MS: i64 = 45 * 60 * 1_000;
/// Upcoming EPG horizon (12 hours).
pub const VIRTUAL_EPG_HORIZON_MS: i64 = 12 * 3_600 * 1_000;
/// Safety cap on emitted upcoming programmes when test clips are very short.
const MAX_UPCOMING_PROGRAMMES: usize = 500;

fn default_true() -> bool {
    true
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualChannel {
    pub id: String,
    pub number: i32,
    pub name: String,
    pub library_id: LibraryId,
    pub genre_filter: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualChannelList {
    pub channels: Vec<VirtualChannel>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVirtualChannelRequest {
    pub number: i32,
    pub name: String,
    pub library_id: LibraryId,
    pub genre_filter: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualProgramme {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub work_title: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub offset_ms: i64,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualChannelGuide {
    pub channel: VirtualChannel,
    pub total_items: usize,
    pub cycle_duration_ms: i64,
    pub now_playing: Option<VirtualProgramme>,
    pub programmes: Vec<VirtualProgramme>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualTvGuide {
    pub generated_at: DateTime<Utc>,
    pub channels: Vec<VirtualChannelGuide>,
    pub playlist_url: String,
    pub epg_url: String,
}

/// Minimal item metadata input to the deterministic 24x7 loop scheduler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulableVideoItem {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub work_title: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub runtime_ms: Option<i64>,
}

impl SchedulableVideoItem {
    pub fn effective_duration_ms(&self) -> i64 {
        self.runtime_ms
            .filter(|&ms| ms >= 1_000)
            .unwrap_or(DEFAULT_VIRTUAL_RUNTIME_MS)
    }
}

/// Deterministically computes `(total_cycle_ms, now_playing, upcoming_12h_programmes)` at `now`.
pub fn compute_channel_schedule(
    items: &[SchedulableVideoItem],
    now: DateTime<Utc>,
) -> (i64, Option<VirtualProgramme>, Vec<VirtualProgramme>) {
    if items.is_empty() {
        return (0, None, Vec::new());
    }

    let durations: Vec<i64> = items
        .iter()
        .map(SchedulableVideoItem::effective_duration_ms)
        .collect();
    let total_cycle_ms: i64 = durations.iter().copied().fold(0_i64, i64::saturating_add);
    if total_cycle_ms <= 0 {
        return (0, None, Vec::new());
    }

    let epoch_ms = clock::to_millis(now);
    let cycle_pos = epoch_ms.rem_euclid(total_cycle_ms);

    let mut acc = 0_i64;
    let mut current_idx = 0_usize;
    let mut offset_ms = 0_i64;
    for (idx, &dur) in durations.iter().enumerate() {
        if cycle_pos < acc + dur {
            current_idx = idx;
            offset_ms = cycle_pos - acc;
            break;
        }
        acc += dur;
    }

    let cur_item = &items[current_idx];
    let cur_dur = durations[current_idx];
    let cur_start_ms = epoch_ms - offset_ms;
    let cur_end_ms = cur_start_ms + cur_dur;

    let now_playing = Some(VirtualProgramme {
        item_id: cur_item.item_id,
        library_id: cur_item.library_id,
        title: cur_item.title.clone(),
        work_title: cur_item.work_title.clone(),
        season: cur_item.season,
        episode: cur_item.episode,
        offset_ms,
        duration_ms: cur_dur,
        started_at: clock::from_millis(cur_start_ms).unwrap_or(now),
        ends_at: clock::from_millis(cur_end_ms).unwrap_or(now),
    });

    let horizon_end_ms = epoch_ms.saturating_add(VIRTUAL_EPG_HORIZON_MS);
    let mut programmes = Vec::new();
    let mut next_start_ms = cur_end_ms;
    let mut step = 1_usize;
    while next_start_ms < horizon_end_ms && programmes.len() < MAX_UPCOMING_PROGRAMMES {
        let idx = (current_idx + step) % items.len();
        let item = &items[idx];
        let dur = durations[idx];
        let next_end_ms = next_start_ms.saturating_add(dur);
        programmes.push(VirtualProgramme {
            item_id: item.item_id,
            library_id: item.library_id,
            title: item.title.clone(),
            work_title: item.work_title.clone(),
            season: item.season,
            episode: item.episode,
            offset_ms: 0,
            duration_ms: dur,
            started_at: clock::from_millis(next_start_ms).unwrap_or(now),
            ends_at: clock::from_millis(next_end_ms).unwrap_or(now),
        });
        next_start_ms = next_end_ms;
        step += 1;
    }

    (total_cycle_ms, now_playing, programmes)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn deterministic_scheduler_computes_exact_offset_and_12h_upcoming_horizon() {
        let lib_id = LibraryId::generate();
        let item_a = ItemId::generate();
        let item_b = ItemId::generate();
        let items = vec![
            SchedulableVideoItem {
                item_id: item_a,
                library_id: lib_id,
                title: "Pilot".into(),
                work_title: "Sci-Fi Odyssey".into(),
                season: Some(1),
                episode: Some(1),
                runtime_ms: Some(1_800_000), // 30m
            },
            SchedulableVideoItem {
                item_id: item_b,
                library_id: lib_id,
                title: "Unprobed Feature".into(),
                work_title: "Mystery Film".into(),
                season: None,
                episode: None,
                runtime_ms: None, // defaults to 45m (2,700,000 ms)
            },
        ];

        // Total cycle = 4,500,000 ms (75 min).
        // Pick epoch_ms = 4_500_000 * 100 + 2_000_000 -> 200,000 ms into item_b!
        let target_ms: i64 = 4_500_000 * 100 + 2_000_000;
        let now = Utc.timestamp_millis_opt(target_ms).single().unwrap();
        let (cycle_ms, now_playing, upcoming) = compute_channel_schedule(&items, now);
        assert_eq!(cycle_ms, 4_500_000);

        let current = now_playing.expect("now playing");
        assert_eq!(current.item_id, item_b);
        assert_eq!(current.duration_ms, DEFAULT_VIRTUAL_RUNTIME_MS);
        assert_eq!(current.offset_ms, 200_000);
        assert_eq!(current.started_at.timestamp_millis(), target_ms - 200_000);
        assert_eq!(
            current.ends_at.timestamp_millis(),
            target_ms - 200_000 + 2_700_000
        );

        assert!(!upcoming.is_empty());
        assert_eq!(upcoming[0].item_id, item_a);
        assert_eq!(upcoming[0].started_at, current.ends_at);
        assert!(
            upcoming.last().unwrap().ends_at.timestamp_millis()
                >= target_ms + VIRTUAL_EPG_HORIZON_MS
        );
    }
}
