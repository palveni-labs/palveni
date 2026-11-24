//! Fast cross-domain global search (`GET /api/v1/search`) across video works,
//! music albums & tracks, books & comics, photos, broadcast entries,
//! cross-media collections, and smart-home devices & scenes.

use std::collections::{HashMap, HashSet};

use palveni_core::UserId;
use palveni_core::search::{GlobalSearchHit, GlobalSearchKind};
use serde::Deserialize;

#[derive(Deserialize)]
struct StoredBroadcastPayload {
    title: String,
    #[serde(default)]
    group: Option<String>,
}
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait,
};

use crate::collections::{escape_like, parse_first_author};
use crate::convert::parse_id;
use crate::entities::{
    broadcast_entries, broadcast_sources, home_devices, home_rooms, home_scenes, items, libraries,
    media_collections, music_albums, photo_metadata, reading_metadata, track_metadata,
    video_metadata,
};
use crate::items::SORT_GROUP_LEAF;
use crate::{Db, DbError};

pub struct GlobalSearch<'a> {
    pub(crate) db: &'a Db,
}

impl GlobalSearch<'_> {
    pub async fn search_all(
        &self,
        user_id: UserId,
        raw_query: &str,
        limit: usize,
    ) -> Result<Vec<GlobalSearchHit>, DbError> {
        let q = raw_query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let max_total = limit.clamp(1, 60);
        let per_domain = (max_total / 2).clamp(5, 16) as u64;

        let pat = format!("%{}%", escape_like(q));
        let lower_q = q.to_lowercase();
        let lower_pat = format!("%{}%", escape_like(&lower_q));

        let live_libs: HashMap<String, String> = libraries::Entity::find()
            .filter(libraries::Column::DeletedAt.is_null())
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|l| (l.id, l.name))
            .collect();

        let mut candidates: Vec<(u8, GlobalSearchHit)> = Vec::new();

        // 1. Video works (`video_metadata`, deduplicated per `(library_id, work_key)`)
        if !live_libs.is_empty() {
            let video_rows = video_metadata::Entity::find()
                .join(JoinType::InnerJoin, video_metadata::Relation::Item.def())
                .filter(items::Column::DeletedAt.is_null())
                .filter(
                    Condition::any()
                        .add(video_metadata::Column::WorkTitle.like(pat.clone()))
                        .add(video_metadata::Column::WorkOriginalTitle.like(pat.clone()))
                        .add(video_metadata::Column::Title.like(pat.clone()))
                        .add(items::Column::SearchText.like(lower_pat.clone())),
                )
                .order_by_desc(items::Column::CreatedAt)
                .limit(per_domain * 3)
                .select_only()
                .columns([
                    video_metadata::Column::ItemId,
                    video_metadata::Column::LibraryId,
                    video_metadata::Column::WorkKey,
                    video_metadata::Column::WorkTitle,
                    video_metadata::Column::WorkYear,
                    video_metadata::Column::Season,
                ])
                .into_tuple::<(String, String, String, String, Option<i32>, Option<i32>)>()
                .all(self.db.main())
                .await?;

            let mut seen_works = HashSet::new();
            let mut count = 0u64;
            for (item_id, lib_id, work_key, work_title, year, season) in video_rows {
                let Some(lib_name) = live_libs.get(&lib_id) else {
                    continue;
                };
                if !seen_works.insert((lib_id.clone(), work_key.clone())) {
                    continue;
                }
                let kind_label = if season.is_some() { "剧集" } else { "电影" };
                let subtitle = match year {
                    Some(y) => Some(format!("{lib_name} · {kind_label} · {y}")),
                    None => Some(format!("{lib_name} · {kind_label}")),
                };
                candidates.push((
                    rank_text(&lower_q, &work_title),
                    GlobalSearchHit {
                        kind: GlobalSearchKind::VideoWork,
                        id: work_key,
                        library_id: Some(parse_id(&lib_id, "library id")?),
                        source_id: None,
                        title: work_title,
                        subtitle,
                        artwork_id: Some(format!("vp_{item_id}")),
                    },
                ));
                count += 1;
                if count >= per_domain {
                    break;
                }
            }

            // 2. Music albums + tracks
            let mut seen_albums = HashSet::new();
            let album_rows = music_albums::Entity::find()
                .filter(
                    Condition::any()
                        .add(music_albums::Column::Name.like(pat.clone()))
                        .add(music_albums::Column::Artist.like(pat.clone())),
                )
                .order_by_desc(music_albums::Column::AddedAt)
                .limit(per_domain)
                .all(self.db.main())
                .await?;

            for album in album_rows {
                if !live_libs.contains_key(&album.library_id) {
                    continue;
                }
                seen_albums.insert((
                    album.library_id.clone(),
                    album.artist_key.clone(),
                    album.album_key.clone(),
                ));
                let title = album
                    .name
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "未知专辑".into());
                let art = (!album.artwork_item_id.is_empty())
                    .then(|| format!("ma_{}", album.artwork_item_id));
                candidates.push((
                    rank_text(&lower_q, &title),
                    GlobalSearchHit {
                        kind: GlobalSearchKind::MusicAlbum,
                        id: format!("{}.{}", album.artist_key, album.album_key),
                        library_id: Some(parse_id(&album.library_id, "library id")?),
                        source_id: None,
                        title,
                        subtitle: album.artist.filter(|s| !s.trim().is_empty()),
                        artwork_id: art,
                    },
                ));
            }

            let track_rows = track_metadata::Entity::find()
                .join(JoinType::InnerJoin, track_metadata::Relation::Item.def())
                .filter(items::Column::DeletedAt.is_null())
                .filter(
                    Condition::any()
                        .add(track_metadata::Column::Title.like(pat.clone()))
                        .add(track_metadata::Column::Artist.like(pat.clone()))
                        .add(track_metadata::Column::Album.like(pat.clone()))
                        .add(items::Column::SearchText.like(lower_pat.clone())),
                )
                .order_by_desc(items::Column::CreatedAt)
                .limit(per_domain * 2)
                .select_only()
                .columns([
                    track_metadata::Column::ItemId,
                    track_metadata::Column::LibraryId,
                    track_metadata::Column::ArtistKey,
                    track_metadata::Column::AlbumKey,
                    track_metadata::Column::Title,
                    track_metadata::Column::Artist,
                    track_metadata::Column::Album,
                ])
                .into_tuple::<(
                    String,
                    String,
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                )>()
                .all(self.db.main())
                .await?;

            let mut track_count = 0u64;
            for (item_id, lib_id, artist_key, album_key, title, artist, album) in track_rows {
                if !live_libs.contains_key(&lib_id) {
                    continue;
                }
                if let Some(ref album_name) = album
                    && album_name.to_lowercase().contains(&lower_q)
                    && seen_albums.insert((lib_id.clone(), artist_key.clone(), album_key.clone()))
                {
                    candidates.push((
                        rank_text(&lower_q, album_name),
                        GlobalSearchHit {
                            kind: GlobalSearchKind::MusicAlbum,
                            id: format!("{artist_key}.{album_key}"),
                            library_id: Some(parse_id(&lib_id, "library id")?),
                            source_id: None,
                            title: album_name.clone(),
                            subtitle: artist.clone(),
                            artwork_id: Some(format!("ma_{item_id}")),
                        },
                    ));
                }
                if track_count < per_domain {
                    let sub_parts: Vec<&str> = [artist.as_deref(), album.as_deref()]
                        .into_iter()
                        .flatten()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect();
                    candidates.push((
                        rank_text(&lower_q, &title).saturating_add(1),
                        GlobalSearchHit {
                            kind: GlobalSearchKind::Track,
                            id: item_id.clone(),
                            library_id: Some(parse_id(&lib_id, "library id")?),
                            source_id: None,
                            title,
                            subtitle: (!sub_parts.is_empty()).then(|| sub_parts.join(" · ")),
                            artwork_id: Some(format!("ma_{item_id}")),
                        },
                    ));
                    track_count += 1;
                }
            }

            // 3. Books & Comics (`reading_metadata` + fallback `items`)
            let mut seen_reading = HashSet::new();
            let reading_rows = reading_metadata::Entity::find()
                .join(JoinType::InnerJoin, reading_metadata::Relation::Item.def())
                .filter(items::Column::DeletedAt.is_null())
                .filter(
                    Condition::any()
                        .add(reading_metadata::Column::Title.like(pat.clone()))
                        .add(reading_metadata::Column::SeriesName.like(pat.clone()))
                        .add(reading_metadata::Column::AuthorsJson.like(pat.clone()))
                        .add(items::Column::SearchText.like(lower_pat.clone())),
                )
                .order_by_asc(reading_metadata::Column::SortTitle)
                .limit(per_domain)
                .all(self.db.main())
                .await?;

            for row in reading_rows {
                if !live_libs.contains_key(&row.library_id) {
                    continue;
                }
                seen_reading.insert(row.item_id.clone());
                let kind = if row.kind == "comic" {
                    GlobalSearchKind::Comic
                } else {
                    GlobalSearchKind::Book
                };
                let subtitle = parse_first_author(&row.authors_json).or(row.series_name);
                candidates.push((
                    rank_text(&lower_q, &row.title),
                    GlobalSearchHit {
                        kind,
                        id: row.item_id.clone(),
                        library_id: Some(parse_id(&row.library_id, "library id")?),
                        source_id: None,
                        title: row.title,
                        subtitle,
                        artwork_id: row.has_cover.then(|| format!("rd_{}", row.item_id)),
                    },
                ));
            }

            let raw_reading = items::Entity::find()
                .filter(items::Column::DeletedAt.is_null())
                .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
                .filter(items::Column::Kind.is_in(["book", "comic"]))
                .filter(
                    Condition::any()
                        .add(items::Column::Title.like(pat.clone()))
                        .add(items::Column::SearchText.like(lower_pat.clone())),
                )
                .limit(per_domain)
                .all(self.db.main())
                .await?;

            for item in raw_reading {
                if !live_libs.contains_key(&item.library_id) || seen_reading.contains(&item.id) {
                    continue;
                }
                let kind = if item.kind == "comic" {
                    GlobalSearchKind::Comic
                } else {
                    GlobalSearchKind::Book
                };
                candidates.push((
                    rank_text(&lower_q, &item.title),
                    GlobalSearchHit {
                        kind,
                        id: item.id.clone(),
                        library_id: Some(parse_id(&item.library_id, "library id")?),
                        source_id: None,
                        title: item.title,
                        subtitle: None,
                        artwork_id: Some(format!("rd_{}", item.id)),
                    },
                ));
            }

            // 4. Photos (`photo_metadata` + `items`)
            let photo_rows = items::Entity::find()
                .join_rev(JoinType::LeftJoin, photo_metadata::Relation::Item.def())
                .filter(items::Column::DeletedAt.is_null())
                .filter(items::Column::Kind.eq("photo"))
                .filter(
                    Condition::any()
                        .add(items::Column::Title.like(pat.clone()))
                        .add(items::Column::SearchText.like(lower_pat.clone()))
                        .add(photo_metadata::Column::CameraMake.like(pat.clone()))
                        .add(photo_metadata::Column::CameraModel.like(pat.clone()))
                        .add(photo_metadata::Column::LensModel.like(pat.clone()))
                        .add(photo_metadata::Column::DayBucket.like(pat.clone())),
                )
                .order_by_desc(items::Column::CreatedAt)
                .limit(per_domain)
                .select_only()
                .columns([
                    items::Column::Id,
                    items::Column::LibraryId,
                    items::Column::Title,
                ])
                .columns([
                    photo_metadata::Column::DayBucket,
                    photo_metadata::Column::CameraMake,
                    photo_metadata::Column::CameraModel,
                ])
                .into_tuple::<(
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                )>()
                .all(self.db.main())
                .await?;

            for (id, lib_id, title, day, make, model) in photo_rows {
                if !live_libs.contains_key(&lib_id) {
                    continue;
                }
                let cam = [make.as_deref(), model.as_deref()]
                    .into_iter()
                    .flatten()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                let sub = match (day.as_deref(), cam.is_empty()) {
                    (Some(d), false) => Some(format!("{d} · {cam}")),
                    (Some(d), true) => Some(d.to_owned()),
                    (None, false) => Some(cam),
                    (None, true) => None,
                };
                candidates.push((
                    rank_text(&lower_q, &title).saturating_add(1),
                    GlobalSearchHit {
                        kind: GlobalSearchKind::Photo,
                        id: id.clone(),
                        library_id: Some(parse_id(&lib_id, "library id")?),
                        source_id: None,
                        title,
                        subtitle: sub,
                        artwork_id: Some(format!("ph_{id}")),
                    },
                ));
            }
        }

        // 5. Broadcast entries (`broadcast_entries` + `broadcast_sources`)
        let bcast_sources: HashMap<String, String> = broadcast_sources::Entity::find()
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|s| (s.id, s.name))
            .collect();
        if !bcast_sources.is_empty() {
            let bcast_rows = broadcast_entries::Entity::find()
                .filter(broadcast_entries::Column::Payload.like(pat.clone()))
                .order_by_asc(broadcast_entries::Column::Position)
                .limit(per_domain)
                .all(self.db.main())
                .await?;
            for row in bcast_rows {
                let Some(source_name) = bcast_sources.get(&row.source_id) else {
                    continue;
                };
                if let Ok(parsed) = serde_json::from_str::<StoredBroadcastPayload>(&row.payload) {
                    if !parsed.title.to_lowercase().contains(&lower_q)
                        && !parsed
                            .group
                            .as_deref()
                            .is_some_and(|g| g.to_lowercase().contains(&lower_q))
                    {
                        continue;
                    }
                    let subtitle = match parsed.group {
                        Some(ref g) if !g.is_empty() => Some(format!("{source_name} · {g}")),
                        _ => Some(source_name.clone()),
                    };
                    candidates.push((
                        rank_text(&lower_q, &parsed.title),
                        GlobalSearchHit {
                            kind: GlobalSearchKind::Broadcast,
                            id: row.entry_id,
                            library_id: None,
                            source_id: Some(row.source_id),
                            title: parsed.title,
                            subtitle,
                            artwork_id: None,
                        },
                    ));
                }
            }
        }

        // 6. Cross-media collections (`media_collections`)
        let coll_rows = media_collections::Entity::find()
            .filter(media_collections::Column::OwnerId.eq(user_id.to_string()))
            .filter(
                Condition::any()
                    .add(media_collections::Column::Title.like(pat.clone()))
                    .add(media_collections::Column::Description.like(pat.clone())),
            )
            .order_by_desc(media_collections::Column::UpdatedAt)
            .limit(per_domain)
            .all(self.db.main())
            .await?;

        for coll in coll_rows {
            candidates.push((
                rank_text(&lower_q, &coll.title),
                GlobalSearchHit {
                    kind: GlobalSearchKind::Collection,
                    id: coll.id,
                    library_id: None,
                    source_id: None,
                    title: coll.title,
                    subtitle: coll.description,
                    artwork_id: coll.cover_artwork_id,
                },
            ));
        }

        // 7. Smart-home devices & scenes (`home_devices`, `home_scenes`)
        let rooms: HashMap<String, String> = home_rooms::Entity::find()
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|r| (r.id, r.name))
            .collect();

        let dev_rows = home_devices::Entity::find()
            .filter(
                Condition::any()
                    .add(home_devices::Column::Name.like(pat.clone()))
                    .add(home_devices::Column::Kind.like(lower_pat)),
            )
            .order_by_asc(home_devices::Column::SortOrder)
            .limit(per_domain)
            .all(self.db.main())
            .await?;

        for dev in dev_rows {
            let room_name = rooms
                .get(&dev.room_id)
                .map(String::as_str)
                .unwrap_or("家居");
            candidates.push((
                rank_text(&lower_q, &dev.name),
                GlobalSearchHit {
                    kind: GlobalSearchKind::HomeDevice,
                    id: dev.id,
                    library_id: None,
                    source_id: Some(dev.room_id),
                    title: dev.name,
                    subtitle: Some(format!("{room_name} · {}", dev.kind)),
                    artwork_id: None,
                },
            ));
        }

        let scene_rows = home_scenes::Entity::find()
            .filter(home_scenes::Column::Name.like(pat))
            .order_by_asc(home_scenes::Column::SortOrder)
            .limit(per_domain)
            .all(self.db.main())
            .await?;

        for scene in scene_rows {
            candidates.push((
                rank_text(&lower_q, &scene.name),
                GlobalSearchHit {
                    kind: GlobalSearchKind::HomeScene,
                    id: scene.id,
                    library_id: None,
                    source_id: None,
                    title: scene.name,
                    subtitle: Some("家居场景".into()),
                    artwork_id: None,
                },
            ));
        }

        candidates.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.title.len().cmp(&b.1.title.len()))
                .then_with(|| a.1.title.cmp(&b.1.title))
        });

        Ok(candidates
            .into_iter()
            .take(max_total)
            .map(|(_, hit)| hit)
            .collect())
    }
}

fn rank_text(lower_query: &str, title: &str) -> u8 {
    let lower_title = title.trim().to_lowercase();
    if lower_title == lower_query {
        0
    } else if lower_title.starts_with(lower_query) {
        1
    } else if lower_title.contains(lower_query) {
        2
    } else {
        3
    }
}
