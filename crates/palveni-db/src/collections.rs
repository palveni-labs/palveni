//! Cross-media collections (`media_collections` + `media_collection_items`) and
//! automatic cross-media work discovery by normalized title stem.

use std::collections::{BTreeSet, HashMap, HashSet};

use palveni_core::collections::{
    MediaCollectionDetail, MediaCollectionEntry, MediaCollectionEntryKind, MediaCollectionSummary,
    RelatedWork,
};
use palveni_core::{CollectionEntryId, CollectionId, ItemId, LibraryId, UserId};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, TransactionTrait,
};

use crate::convert::{now_ms, parse_id, parse_opt_id, truncate_chars, ts};
use crate::entities::{
    items, libraries,
    media_collection_items::{self as ci, Column as CICol},
    media_collections::{self as mc, Column as MCCol},
    music_albums, reading_metadata, track_metadata, video_metadata,
};
use crate::items::SORT_GROUP_LEAF;
use crate::{Db, DbError};

pub struct Collections<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug, Default)]
pub struct CollectionPatch<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub cover_artwork_id: Option<&'a str>,
    pub item_order: Option<&'a [CollectionEntryId]>,
}

#[derive(Clone, Debug)]
pub struct AddCollectionItemInput<'a> {
    pub entry_kind: MediaCollectionEntryKind,
    pub target_id: &'a str,
    pub library_id: Option<LibraryId>,
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub artwork_id: Option<&'a str>,
}

#[derive(Clone, Debug)]
struct ResolvedCatalogMeta {
    library_id: Option<LibraryId>,
    title: String,
    subtitle: Option<String>,
    artwork_id: Option<String>,
}

impl Collections<'_> {
    pub async fn list(&self, owner_id: UserId) -> Result<Vec<MediaCollectionSummary>, DbError> {
        let rows = mc::Entity::find()
            .filter(MCCol::OwnerId.eq(owner_id.to_string()))
            .order_by_desc(MCCol::UpdatedAt)
            .order_by_desc(MCCol::Id)
            .all(self.db.main())
            .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
        let all_items = ci::Entity::find()
            .filter(CICol::CollectionId.is_in(ids))
            .order_by_asc(CICol::Position)
            .order_by_asc(CICol::Id)
            .all(self.db.main())
            .await?;

        let mut grouped: HashMap<String, Vec<ci::Model>> = HashMap::new();
        for item in all_items {
            grouped
                .entry(item.collection_id.clone())
                .or_default()
                .push(item);
        }

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let items = grouped.remove(&row.id).unwrap_or_default();
            out.push(build_summary(row, &items)?);
        }
        Ok(out)
    }

    pub async fn create(
        &self,
        owner_id: UserId,
        title: &str,
        description: Option<&str>,
        cover_artwork_id: Option<&str>,
    ) -> Result<MediaCollectionSummary, DbError> {
        let clean_title = validate_title(title)?;
        let clean_desc = clean_opt_text(description, 2000);
        let clean_cover = clean_opt_text(cover_artwork_id, 80);
        let now = now_ms();
        let id = CollectionId::generate();

        let model = mc::Model {
            id: id.to_string(),
            owner_id: owner_id.to_string(),
            title: clean_title,
            description: clean_desc.clone(),
            cover_artwork_id: clean_cover.clone(),
            created_at: now,
            updated_at: now,
        };

        mc::Entity::insert(mc::ActiveModel {
            id: ActiveValue::Set(model.id.clone()),
            owner_id: ActiveValue::Set(model.owner_id.clone()),
            title: ActiveValue::Set(model.title.clone()),
            description: ActiveValue::Set(model.description.clone()),
            cover_artwork_id: ActiveValue::Set(model.cover_artwork_id.clone()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        })
        .exec_without_returning(self.db.main())
        .await?;

        build_summary(model, &[])
    }

    pub async fn get_detail(
        &self,
        viewer_id: UserId,
        id: CollectionId,
        is_admin: bool,
    ) -> Result<Option<MediaCollectionDetail>, DbError> {
        let Some(row) = mc::Entity::find_by_id(id.to_string())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        if !is_admin && row.owner_id != viewer_id.to_string() {
            return Ok(None);
        }

        let item_rows = ci::Entity::find()
            .filter(CICol::CollectionId.eq(row.id.clone()))
            .order_by_asc(CICol::Position)
            .order_by_asc(CICol::Id)
            .all(self.db.main())
            .await?;

        let mut seed_titles: Vec<String> = vec![row.title.clone()];
        for item in item_rows.iter().take(6) {
            if !seed_titles.iter().any(|t| t == &item.title) {
                seed_titles.push(item.title.clone());
            }
        }

        let summary = build_summary(row, &item_rows)?;
        let mut entries = Vec::with_capacity(item_rows.len());
        let mut existing_keys = HashSet::new();
        for item in item_rows {
            let entry = to_entry(item)?;
            existing_keys.insert((entry.entry_kind, entry.target_id.clone()));
            entries.push(entry);
        }

        let mut seen_related = HashSet::new();
        let mut auto_related = Vec::new();
        for seed in &seed_titles {
            let candidates = self.related_across_media(seed, None, 12).await?;
            for work in candidates {
                let key = (work.entry_kind, work.target_id.clone());
                if existing_keys.contains(&key)
                    || !seen_related.insert((
                        work.entry_kind,
                        work.library_id,
                        work.target_id.clone(),
                    ))
                {
                    continue;
                }
                auto_related.push(work);
                if auto_related.len() >= 16 {
                    break;
                }
            }
            if auto_related.len() >= 16 {
                break;
            }
        }

        Ok(Some(MediaCollectionDetail {
            collection: summary,
            items: entries,
            auto_related,
        }))
    }

    pub async fn update(
        &self,
        actor_id: UserId,
        id: CollectionId,
        is_admin: bool,
        patch: CollectionPatch<'_>,
    ) -> Result<MediaCollectionSummary, DbError> {
        let Some(existing) = mc::Entity::find_by_id(id.to_string())
            .one(self.db.main())
            .await?
        else {
            return Err(DbError::NotFound("collection"));
        };
        if !is_admin && existing.owner_id != actor_id.to_string() {
            return Err(DbError::NotFound("collection"));
        }

        let next_title = match patch.title {
            Some(raw) => validate_title(raw)?,
            None => existing.title.clone(),
        };
        let next_desc = match patch.description {
            Some(raw) => clean_opt_text(Some(raw), 2000),
            None => existing.description.clone(),
        };
        let next_cover = match patch.cover_artwork_id {
            Some(raw) => clean_opt_text(Some(raw), 80),
            None => existing.cover_artwork_id.clone(),
        };
        let now = now_ms();

        let txn = self.db.main().begin().await?;
        mc::Entity::update_many()
            .col_expr(MCCol::Title, Expr::val(next_title.clone()))
            .col_expr(MCCol::Description, Expr::val(next_desc.clone()))
            .col_expr(MCCol::CoverArtworkId, Expr::val(next_cover.clone()))
            .col_expr(MCCol::UpdatedAt, Expr::val(now))
            .filter(MCCol::Id.eq(id.to_string()))
            .exec(&txn)
            .await?;

        if let Some(order) = patch.item_order {
            let current_items = ci::Entity::find()
                .filter(CICol::CollectionId.eq(id.to_string()))
                .order_by_asc(CICol::Position)
                .order_by_asc(CICol::Id)
                .all(&txn)
                .await?;
            let valid_ids: HashSet<String> = current_items.iter().map(|i| i.id.clone()).collect();
            let mut new_sequence: Vec<String> = Vec::with_capacity(current_items.len());
            let mut seen = HashSet::new();
            for eid in order {
                let s = eid.to_string();
                if valid_ids.contains(&s) && seen.insert(s.clone()) {
                    new_sequence.push(s);
                }
            }
            for item in &current_items {
                if seen.insert(item.id.clone()) {
                    new_sequence.push(item.id.clone());
                }
            }
            for (idx, entry_id) in new_sequence.into_iter().enumerate() {
                let pos = i32::try_from(idx).unwrap_or(i32::MAX);
                ci::Entity::update_many()
                    .col_expr(CICol::Position, Expr::val(pos))
                    .filter(CICol::CollectionId.eq(id.to_string()))
                    .filter(CICol::Id.eq(entry_id))
                    .exec(&txn)
                    .await?;
            }
        }

        let item_rows = ci::Entity::find()
            .filter(CICol::CollectionId.eq(id.to_string()))
            .order_by_asc(CICol::Position)
            .order_by_asc(CICol::Id)
            .all(&txn)
            .await?;
        txn.commit().await?;

        build_summary(
            mc::Model {
                id: existing.id,
                owner_id: existing.owner_id,
                title: next_title,
                description: next_desc,
                cover_artwork_id: next_cover,
                created_at: existing.created_at,
                updated_at: now,
            },
            &item_rows,
        )
    }

    pub async fn delete(
        &self,
        actor_id: UserId,
        id: CollectionId,
        is_admin: bool,
    ) -> Result<bool, DbError> {
        let mut cond = Condition::all().add(MCCol::Id.eq(id.to_string()));
        if !is_admin {
            cond = cond.add(MCCol::OwnerId.eq(actor_id.to_string()));
        }
        let res = mc::Entity::delete_many()
            .filter(cond)
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn add_item(
        &self,
        actor_id: UserId,
        collection_id: CollectionId,
        is_admin: bool,
        input: AddCollectionItemInput<'_>,
    ) -> Result<MediaCollectionEntry, DbError> {
        let Some(coll) = mc::Entity::find_by_id(collection_id.to_string())
            .one(self.db.main())
            .await?
        else {
            return Err(DbError::NotFound("collection"));
        };
        if !is_admin && coll.owner_id != actor_id.to_string() {
            return Err(DbError::NotFound("collection"));
        }

        let target_id = input.target_id.trim();
        if target_id.is_empty()
            || target_id.chars().count() > 128
            || target_id.chars().any(char::is_control)
        {
            return Err(DbError::Invalid("Invalid collection item targetId"));
        }

        // Return existing entry if already present in this collection.
        if let Some(existing) = ci::Entity::find()
            .filter(CICol::CollectionId.eq(collection_id.to_string()))
            .filter(CICol::EntryKind.eq(input.entry_kind.as_str()))
            .filter(CICol::TargetId.eq(target_id))
            .one(self.db.main())
            .await?
        {
            return to_entry(existing);
        }

        let resolved = self
            .resolve_catalog_entry(input.entry_kind, target_id, input.library_id)
            .await?;

        let final_title = input
            .title
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| truncate_chars(s, 255))
            .or_else(|| resolved.as_ref().map(|r| truncate_chars(&r.title, 255)))
            .ok_or_else(|| DbError::NotFound("target media item"))?;

        let final_subtitle = clean_opt_text(input.subtitle, 255).or_else(|| {
            resolved
                .as_ref()
                .and_then(|r| r.subtitle.as_deref())
                .map(|s| truncate_chars(s, 255))
        });
        let final_lib = input
            .library_id
            .or_else(|| resolved.as_ref().and_then(|r| r.library_id));
        let final_art =
            clean_opt_text(input.artwork_id, 80).or_else(|| resolved.and_then(|r| r.artwork_id));

        let last_pos = ci::Entity::find()
            .filter(CICol::CollectionId.eq(collection_id.to_string()))
            .order_by_desc(CICol::Position)
            .select_only()
            .column(CICol::Position)
            .into_tuple::<i32>()
            .one(self.db.main())
            .await?
            .map_or(0, |p| p.saturating_add(1));

        let now = now_ms();
        let entry_id = CollectionEntryId::generate();
        let model = ci::Model {
            id: entry_id.to_string(),
            collection_id: collection_id.to_string(),
            entry_kind: input.entry_kind.as_str().to_owned(),
            target_id: target_id.to_owned(),
            library_id: final_lib.map(|l| l.to_string()),
            title: final_title,
            subtitle: final_subtitle,
            artwork_id: final_art,
            position: last_pos,
            added_at: now,
        };

        let txn = self.db.main().begin().await?;
        ci::Entity::insert(ci::ActiveModel {
            id: ActiveValue::Set(model.id.clone()),
            collection_id: ActiveValue::Set(model.collection_id.clone()),
            entry_kind: ActiveValue::Set(model.entry_kind.clone()),
            target_id: ActiveValue::Set(model.target_id.clone()),
            library_id: ActiveValue::Set(model.library_id.clone()),
            title: ActiveValue::Set(model.title.clone()),
            subtitle: ActiveValue::Set(model.subtitle.clone()),
            artwork_id: ActiveValue::Set(model.artwork_id.clone()),
            position: ActiveValue::Set(model.position),
            added_at: ActiveValue::Set(model.added_at),
        })
        .exec_without_returning(&txn)
        .await?;

        mc::Entity::update_many()
            .col_expr(MCCol::UpdatedAt, Expr::val(now))
            .filter(MCCol::Id.eq(collection_id.to_string()))
            .exec(&txn)
            .await?;
        txn.commit().await?;

        to_entry(model)
    }

    pub async fn remove_item(
        &self,
        actor_id: UserId,
        collection_id: CollectionId,
        entry_id: CollectionEntryId,
        is_admin: bool,
    ) -> Result<bool, DbError> {
        let Some(coll) = mc::Entity::find_by_id(collection_id.to_string())
            .one(self.db.main())
            .await?
        else {
            return Err(DbError::NotFound("collection"));
        };
        if !is_admin && coll.owner_id != actor_id.to_string() {
            return Err(DbError::NotFound("collection"));
        }

        let res = ci::Entity::delete_many()
            .filter(CICol::CollectionId.eq(collection_id.to_string()))
            .filter(CICol::Id.eq(entry_id.to_string()))
            .exec(self.db.main())
            .await?;
        if res.rows_affected > 0 {
            mc::Entity::update_many()
                .col_expr(MCCol::UpdatedAt, Expr::val(now_ms()))
                .filter(MCCol::Id.eq(collection_id.to_string()))
                .exec(self.db.main())
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Finds related works across `video_metadata`, `music_albums`/`track_metadata`,
    /// and `reading_metadata`/`items` matching the normalized title stem of `title`.
    pub async fn related_across_media(
        &self,
        title: &str,
        exclude_kind: Option<MediaCollectionEntryKind>,
        limit: usize,
    ) -> Result<Vec<RelatedWork>, DbError> {
        let stem = normalize_work_stem(title);
        if !is_searchable_stem(&stem) {
            return Ok(Vec::new());
        }

        let live_libs: HashSet<String> = libraries::Entity::find()
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column(libraries::Column::Id)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?
            .into_iter()
            .collect();
        if live_libs.is_empty() {
            return Ok(Vec::new());
        }

        let like_pat = format!("%{}%", escape_like(&stem_probe_token(&stem)));
        let mut scored: Vec<(u8, RelatedWork)> = Vec::new();
        let max_out = limit.clamp(1, 50);

        // 1. Video works (`video_metadata`)
        if exclude_kind != Some(MediaCollectionEntryKind::Video) {
            let rows = video_metadata::Entity::find()
                .join(JoinType::InnerJoin, video_metadata::Relation::Item.def())
                .filter(items::Column::DeletedAt.is_null())
                .filter(
                    Condition::any()
                        .add(video_metadata::Column::WorkTitle.like(like_pat.clone()))
                        .add(video_metadata::Column::WorkOriginalTitle.like(like_pat.clone()))
                        .add(items::Column::SearchText.like(like_pat.to_lowercase())),
                )
                .order_by_desc(items::Column::CreatedAt)
                .limit(120)
                .select_only()
                .columns([
                    video_metadata::Column::ItemId,
                    video_metadata::Column::LibraryId,
                    video_metadata::Column::WorkKey,
                    video_metadata::Column::WorkTitle,
                    video_metadata::Column::WorkOriginalTitle,
                    video_metadata::Column::WorkYear,
                ])
                .into_tuple::<(String, String, String, String, Option<String>, Option<i32>)>()
                .all(self.db.main())
                .await?;

            let mut seen_works = HashSet::new();
            for (item_id, lib_id, work_key, work_title, orig_title, year) in rows {
                if !live_libs.contains(&lib_id)
                    || !seen_works.insert((lib_id.clone(), work_key.clone()))
                {
                    continue;
                }
                let rank = stem_match_rank(&stem, &work_title).or_else(|| {
                    orig_title
                        .as_deref()
                        .and_then(|o| stem_match_rank(&stem, o))
                });
                let Some(score) = rank else { continue };
                scored.push((
                    score,
                    RelatedWork {
                        entry_kind: MediaCollectionEntryKind::Video,
                        target_id: work_key,
                        library_id: parse_id(&lib_id, "library id")?,
                        title: work_title,
                        subtitle: year.map(|y| y.to_string()),
                        artwork_id: Some(format!("vp_{item_id}")),
                    },
                ));
            }
        }

        // 2. Music albums (`music_albums` + fallback `track_metadata`)
        if exclude_kind != Some(MediaCollectionEntryKind::MusicAlbum) {
            let mut seen_albums = HashSet::new();

            let album_rows = music_albums::Entity::find()
                .filter(music_albums::Column::Name.like(like_pat.clone()))
                .order_by_desc(music_albums::Column::AddedAt)
                .limit(80)
                .all(self.db.main())
                .await?;

            for album in album_rows {
                if !live_libs.contains(&album.library_id) {
                    continue;
                }
                let Some(ref album_name) = album.name else {
                    continue;
                };
                let Some(score) = stem_match_rank(&stem, album_name) else {
                    continue;
                };
                seen_albums.insert((
                    album.library_id.clone(),
                    album.artist_key.clone(),
                    album.album_key.clone(),
                ));
                let art = (!album.artwork_item_id.is_empty())
                    .then(|| format!("ma_{}", album.artwork_item_id));
                scored.push((
                    score,
                    RelatedWork {
                        entry_kind: MediaCollectionEntryKind::MusicAlbum,
                        target_id: format!("{}.{}", album.artist_key, album.album_key),
                        library_id: parse_id(&album.library_id, "library id")?,
                        title: album_name.clone(),
                        subtitle: album.artist.filter(|a| !a.trim().is_empty()),
                        artwork_id: art,
                    },
                ));
            }

            let track_rows = track_metadata::Entity::find()
                .join(JoinType::InnerJoin, track_metadata::Relation::Item.def())
                .filter(items::Column::DeletedAt.is_null())
                .filter(
                    Condition::any()
                        .add(track_metadata::Column::Album.like(like_pat.clone()))
                        .add(items::Column::SearchText.like(like_pat.to_lowercase())),
                )
                .order_by_desc(items::Column::CreatedAt)
                .limit(120)
                .select_only()
                .columns([
                    track_metadata::Column::ItemId,
                    track_metadata::Column::LibraryId,
                    track_metadata::Column::ArtistKey,
                    track_metadata::Column::AlbumKey,
                    track_metadata::Column::Album,
                    track_metadata::Column::AlbumArtist,
                    track_metadata::Column::Artist,
                ])
                .into_tuple::<(
                    String,
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                )>()
                .all(self.db.main())
                .await?;

            for (item_id, lib_id, artist_key, album_key, album_opt, album_artist, artist) in
                track_rows
            {
                if !live_libs.contains(&lib_id) {
                    continue;
                }
                let Some(album_name) = album_opt.filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                if !seen_albums.insert((lib_id.clone(), artist_key.clone(), album_key.clone())) {
                    continue;
                }
                let Some(score) = stem_match_rank(&stem, &album_name) else {
                    continue;
                };
                scored.push((
                    score,
                    RelatedWork {
                        entry_kind: MediaCollectionEntryKind::MusicAlbum,
                        target_id: format!("{artist_key}.{album_key}"),
                        library_id: parse_id(&lib_id, "library id")?,
                        title: album_name,
                        subtitle: album_artist
                            .filter(|s| !s.trim().is_empty())
                            .or_else(|| artist.filter(|s| !s.trim().is_empty())),
                        artwork_id: Some(format!("ma_{item_id}")),
                    },
                ));
            }
        }

        // 3. Books & Comics (`reading_metadata` + fallback `items`)
        let want_books = exclude_kind != Some(MediaCollectionEntryKind::Book);
        let want_comics = exclude_kind != Some(MediaCollectionEntryKind::Comic);
        if want_books || want_comics {
            let mut seen_reading_ids = HashSet::new();
            let reading_rows = reading_metadata::Entity::find()
                .join(JoinType::InnerJoin, reading_metadata::Relation::Item.def())
                .filter(items::Column::DeletedAt.is_null())
                .filter(
                    Condition::any()
                        .add(reading_metadata::Column::Title.like(like_pat.clone()))
                        .add(reading_metadata::Column::SeriesName.like(like_pat.clone()))
                        .add(items::Column::SearchText.like(like_pat.to_lowercase())),
                )
                .order_by_asc(reading_metadata::Column::SortTitle)
                .limit(100)
                .all(self.db.main())
                .await?;

            for row in reading_rows {
                if !live_libs.contains(&row.library_id) {
                    continue;
                }
                let kind = if row.kind == "comic" {
                    MediaCollectionEntryKind::Comic
                } else {
                    MediaCollectionEntryKind::Book
                };
                if (kind == MediaCollectionEntryKind::Book && !want_books)
                    || (kind == MediaCollectionEntryKind::Comic && !want_comics)
                {
                    continue;
                }
                let rank = stem_match_rank(&stem, &row.title).or_else(|| {
                    row.series_name
                        .as_deref()
                        .and_then(|s| stem_match_rank(&stem, s))
                });
                let Some(score) = rank else { continue };
                seen_reading_ids.insert(row.item_id.clone());
                let subtitle = parse_first_author(&row.authors_json).or(row.series_name);
                scored.push((
                    score,
                    RelatedWork {
                        entry_kind: kind,
                        target_id: row.item_id.clone(),
                        library_id: parse_id(&row.library_id, "library id")?,
                        title: row.title,
                        subtitle,
                        artwork_id: row.has_cover.then(|| format!("rd_{}", row.item_id)),
                    },
                ));
            }

            let raw_items = items::Entity::find()
                .filter(items::Column::DeletedAt.is_null())
                .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
                .filter(items::Column::Kind.is_in(["book", "comic"]))
                .filter(
                    Condition::any()
                        .add(items::Column::Title.like(like_pat.clone()))
                        .add(items::Column::SearchText.like(like_pat.to_lowercase())),
                )
                .limit(80)
                .all(self.db.main())
                .await?;

            for item in raw_items {
                if !live_libs.contains(&item.library_id) || seen_reading_ids.contains(&item.id) {
                    continue;
                }
                let kind = if item.kind == "comic" {
                    MediaCollectionEntryKind::Comic
                } else {
                    MediaCollectionEntryKind::Book
                };
                if (kind == MediaCollectionEntryKind::Book && !want_books)
                    || (kind == MediaCollectionEntryKind::Comic && !want_comics)
                {
                    continue;
                }
                let Some(score) = stem_match_rank(&stem, &item.title) else {
                    continue;
                };
                scored.push((
                    score,
                    RelatedWork {
                        entry_kind: kind,
                        target_id: item.id.clone(),
                        library_id: parse_id(&item.library_id, "library id")?,
                        title: item.title,
                        subtitle: None,
                        artwork_id: Some(format!("rd_{}", item.id)),
                    },
                ));
            }
        }

        scored.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.title.cmp(&b.1.title))
                .then_with(|| a.1.target_id.cmp(&b.1.target_id))
        });
        Ok(scored.into_iter().take(max_out).map(|(_, w)| w).collect())
    }

    async fn resolve_catalog_entry(
        &self,
        kind: MediaCollectionEntryKind,
        target_id: &str,
        library_id: Option<LibraryId>,
    ) -> Result<Option<ResolvedCatalogMeta>, DbError> {
        match kind {
            MediaCollectionEntryKind::Video => {
                let mut q = video_metadata::Entity::find()
                    .join(JoinType::InnerJoin, video_metadata::Relation::Item.def())
                    .filter(items::Column::DeletedAt.is_null())
                    .filter(
                        Condition::any()
                            .add(video_metadata::Column::WorkKey.eq(target_id))
                            .add(video_metadata::Column::ItemId.eq(target_id)),
                    );
                if let Some(lib) = library_id {
                    q = q.filter(video_metadata::Column::LibraryId.eq(lib.to_string()));
                }
                if let Some(row) = q.one(self.db.main()).await? {
                    return Ok(Some(ResolvedCatalogMeta {
                        library_id: Some(parse_id(&row.library_id, "library id")?),
                        title: row.work_title,
                        subtitle: row.work_year.map(|y| y.to_string()),
                        artwork_id: Some(format!("vp_{}", row.item_id)),
                    }));
                }
                Ok(None)
            }
            MediaCollectionEntryKind::MusicAlbum => {
                if let Some((artist_key, album_key)) = target_id.split_once('.') {
                    let mut aq = music_albums::Entity::find()
                        .filter(music_albums::Column::ArtistKey.eq(artist_key))
                        .filter(music_albums::Column::AlbumKey.eq(album_key));
                    if let Some(lib) = library_id {
                        aq = aq.filter(music_albums::Column::LibraryId.eq(lib.to_string()));
                    }
                    if let Some(album) = aq.one(self.db.main()).await? {
                        return Ok(Some(ResolvedCatalogMeta {
                            library_id: Some(parse_id(&album.library_id, "library id")?),
                            title: album.name.unwrap_or_else(|| "未知专辑".into()),
                            subtitle: album.artist,
                            artwork_id: (!album.artwork_item_id.is_empty())
                                .then(|| format!("ma_{}", album.artwork_item_id)),
                        }));
                    }
                    let mut tq = track_metadata::Entity::find()
                        .join(JoinType::InnerJoin, track_metadata::Relation::Item.def())
                        .filter(items::Column::DeletedAt.is_null())
                        .filter(track_metadata::Column::ArtistKey.eq(artist_key))
                        .filter(track_metadata::Column::AlbumKey.eq(album_key));
                    if let Some(lib) = library_id {
                        tq = tq.filter(track_metadata::Column::LibraryId.eq(lib.to_string()));
                    }
                    if let Some(tr) = tq.one(self.db.main()).await? {
                        return Ok(Some(ResolvedCatalogMeta {
                            library_id: Some(parse_id(&tr.library_id, "library id")?),
                            title: tr.album.unwrap_or_else(|| tr.title.clone()),
                            subtitle: tr.album_artist.or(tr.artist),
                            artwork_id: Some(format!("ma_{}", tr.item_id)),
                        }));
                    }
                }
                Ok(None)
            }
            MediaCollectionEntryKind::Track
            | MediaCollectionEntryKind::Book
            | MediaCollectionEntryKind::Comic
            | MediaCollectionEntryKind::Photo => {
                let Ok(item_id) = target_id.parse::<ItemId>() else {
                    return Ok(None);
                };
                let Some(item) = items::Entity::find_by_id(item_id.to_string())
                    .filter(items::Column::DeletedAt.is_null())
                    .one(self.db.main())
                    .await?
                else {
                    return Ok(None);
                };
                let lib_id = parse_id(&item.library_id, "library id")?;
                match kind {
                    MediaCollectionEntryKind::Track => {
                        let meta = track_metadata::Entity::find_by_id(item.id.clone())
                            .one(self.db.main())
                            .await?;
                        Ok(Some(ResolvedCatalogMeta {
                            library_id: Some(lib_id),
                            title: meta.as_ref().map_or(item.title, |m| m.title.clone()),
                            subtitle: meta.and_then(|m| m.artist.or(m.album)),
                            artwork_id: Some(format!("ma_{}", item.id)),
                        }))
                    }
                    MediaCollectionEntryKind::Book | MediaCollectionEntryKind::Comic => {
                        let meta = reading_metadata::Entity::find_by_id(item.id.clone())
                            .one(self.db.main())
                            .await?;
                        Ok(Some(ResolvedCatalogMeta {
                            library_id: Some(lib_id),
                            title: meta.as_ref().map_or(item.title, |m| m.title.clone()),
                            subtitle: meta.and_then(|m| {
                                parse_first_author(&m.authors_json).or(m.series_name)
                            }),
                            artwork_id: Some(format!("rd_{}", item.id)),
                        }))
                    }
                    MediaCollectionEntryKind::Photo => Ok(Some(ResolvedCatalogMeta {
                        library_id: Some(lib_id),
                        title: item.title,
                        subtitle: None,
                        artwork_id: Some(format!("ph_{}", item.id)),
                    })),
                    _ => Ok(None),
                }
            }
        }
    }
}

fn validate_title(raw: &str) -> Result<String, DbError> {
    let trimmed = raw.trim();
    let count = trimmed.chars().count();
    if count == 0 || count > 255 || trimmed.chars().any(char::is_control) {
        return Err(DbError::Invalid(
            "Collection title must be 1 to 255 non-control characters",
        ));
    }
    Ok(trimmed.to_owned())
}

fn clean_opt_text(raw: Option<&str>, max_chars: usize) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() {
        return None;
    }
    let clean: String = trimmed.chars().filter(|c| !c.is_control()).collect();
    let truncated = truncate_chars(&clean, max_chars);
    (!truncated.is_empty()).then_some(truncated)
}

fn build_summary(row: mc::Model, items: &[ci::Model]) -> Result<MediaCollectionSummary, DbError> {
    let mut preview_artwork_ids = Vec::new();
    let mut kinds = Vec::new();
    let mut seen_kinds = BTreeSet::new();

    if let Some(ref cover) = row.cover_artwork_id
        && !cover.is_empty()
    {
        preview_artwork_ids.push(cover.clone());
    }
    for item in items {
        if let Ok(k) = item.entry_kind.parse::<MediaCollectionEntryKind>()
            && seen_kinds.insert(k.as_str())
        {
            kinds.push(k);
        }
        if let Some(ref art) = item.artwork_id
            && !art.is_empty()
            && preview_artwork_ids.len() < 4
            && !preview_artwork_ids.contains(art)
        {
            preview_artwork_ids.push(art.clone());
        }
    }

    Ok(MediaCollectionSummary {
        id: parse_id(&row.id, "collection id")?,
        owner_id: parse_id(&row.owner_id, "owner id")?,
        title: row.title,
        description: row.description,
        cover_artwork_id: row.cover_artwork_id,
        preview_artwork_ids,
        item_count: u32::try_from(items.len()).unwrap_or(u32::MAX),
        kinds,
        created_at: ts(row.created_at)?,
        updated_at: ts(row.updated_at)?,
    })
}

fn to_entry(row: ci::Model) -> Result<MediaCollectionEntry, DbError> {
    let entry_kind: MediaCollectionEntryKind = row.entry_kind.parse().map_err(|_| {
        DbError::Corrupt(format!(
            "invalid collection entry_kind `{}`",
            row.entry_kind
        ))
    })?;
    Ok(MediaCollectionEntry {
        id: parse_id(&row.id, "collection entry id")?,
        collection_id: parse_id(&row.collection_id, "collection id")?,
        entry_kind,
        target_id: row.target_id,
        library_id: parse_opt_id(row.library_id.as_deref(), "library id")?,
        title: row.title,
        subtitle: row.subtitle,
        artwork_id: row.artwork_id,
        position: row.position,
        added_at: ts(row.added_at)?,
    })
}

pub(crate) fn parse_first_author(authors_json: &str) -> Option<String> {
    let list: Vec<String> = serde_json::from_str(authors_json).ok()?;
    let joined = list
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join(" / ");
    (!joined.is_empty()).then_some(joined)
}

pub(crate) fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Normalizes a media title down to its cross-media work stem so e.g.
/// `"铃芽之旅"` / `"铃芽之旅 原声带"` / `"铃芽之旅（官方小说）"` / `"Dune: Part Two"` /
/// `"Dune (Original Motion Picture Soundtrack)"` produce the same canonical stem.
pub fn normalize_work_stem(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();
    if s.is_empty() {
        return String::new();
    }

    // Strip bracketed qualifiers (`(...)`, `（...）`, `[...]`, `【...】`)
    for (open, close) in [('(', ')'), ('（', '）'), ('[', ']'), ('【', '】')] {
        while let Some(start) = s.find(open) {
            if let Some(rel_end) = s[start..].find(close) {
                let end = start + rel_end + close.len_utf8();
                s.replace_range(start..end, " ");
            } else {
                break;
            }
        }
    }

    // Strip subtitle after `:` / `：` / ` - ` when the right-hand side is an edition/part/OST tag
    for sep in [": ", "：", " - ", " — ", " – "] {
        if let Some((left, right)) = s.split_once(sep)
            && !left.trim().is_empty()
            && is_edition_or_media_qualifier(right.trim())
        {
            s = left.trim().to_owned();
        }
    }

    // Repeatedly strip trailing media/OST/volume suffixes
    loop {
        let before = s.clone();
        s = strip_trailing_media_suffix(s.trim()).to_owned();
        if s == before {
            break;
        }
    }

    // Keep alphanumeric & CJK characters, collapsing punctuation into single spaces
    let mut cleaned = String::with_capacity(s.len());
    let mut prev_space = true;
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            cleaned.push(ch);
            prev_space = false;
        } else if !prev_space {
            cleaned.push(' ');
            prev_space = true;
        }
    }
    cleaned.trim().to_owned()
}

fn is_edition_or_media_qualifier(rhs: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "part ",
        "season ",
        "book ",
        "vol",
        "volume",
        "chapter",
        "episode",
        "original",
        "soundtrack",
        "ost",
        "score",
        "novel",
        "manga",
        "comic",
        "deluxe",
        "edition",
        "remaster",
        "原声",
        "小说",
        "漫画",
        "剧场版",
        "电影版",
        "全集",
        "合集",
        "第",
        "上册",
        "下册",
    ];
    KEYWORDS.iter().any(|kw| rhs.contains(kw))
}

fn strip_trailing_media_suffix(input: &str) -> &str {
    const SUFFIXES: &[&str] = &[
        "original motion picture soundtrack",
        "motion picture soundtrack",
        "original television soundtrack",
        "original series soundtrack",
        "original anime soundtrack",
        "original soundtrack",
        "original score",
        "the official movie novelization",
        "official novelization",
        "novelization",
        "soundtrack",
        "o.s.t.",
        "ost",
        "deluxe edition",
        "expanded edition",
        "special edition",
        "collector's edition",
        "电影原声带",
        "动画原声带",
        "游戏原声带",
        "原声音乐",
        "原声大碟",
        "原声带",
        "官方小说",
        "轻小说",
        "小说版",
        "漫画版",
        "剧场版",
        "电影版",
        "完全版",
        "全集",
        "上册",
        "下册",
    ];
    let trimmed = input.trim_end_matches(|c: char| c.is_whitespace() || "-_:·/".contains(c));
    for suffix in SUFFIXES {
        if let Some(prefix) = trimmed.strip_suffix(suffix) {
            let p = prefix.trim_end_matches(|c: char| c.is_whitespace() || "-_:·/".contains(c));
            if !p.is_empty() {
                return p;
            }
        }
    }

    // Strip trailing `vol 1`, `book 02`, `season 1`, `part 2`, `第01卷`, `第1季`
    for marker in [" vol ", " vol.", " volume ", " book ", " season ", " part "] {
        if let Some((prefix, rest)) = trimmed.rsplit_once(marker)
            && !prefix.trim().is_empty()
            && rest
                .trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.')
        {
            return prefix.trim();
        }
    }
    if let Some((prefix, rest)) = trimmed.rsplit_once('第')
        && !prefix.trim().is_empty()
        && rest.ends_with(['卷', '册', '季', '部', '话', '集'])
    {
        return prefix.trim();
    }

    trimmed
}

fn is_searchable_stem(stem: &str) -> bool {
    let char_count = stem.chars().count();
    if char_count < 2 {
        return false;
    }
    let has_non_ascii = !stem.is_ascii();
    has_non_ascii || char_count >= 3
}

fn stem_probe_token(stem: &str) -> String {
    stem.split_whitespace()
        .max_by_key(|w| w.chars().count())
        .unwrap_or(stem)
        .to_owned()
}

/// Returns `Some(0)` for an exact stem match, `Some(1)` for a clean word/CJK boundary prefix match.
fn stem_match_rank(query_stem: &str, candidate_title: &str) -> Option<u8> {
    let cand_stem = normalize_work_stem(candidate_title);
    if !is_searchable_stem(&cand_stem) {
        return None;
    }
    if cand_stem == query_stem {
        return Some(0);
    }
    let has_cjk = !query_stem.is_ascii();
    if let Some(rest) = cand_stem.strip_prefix(query_stem)
        && (rest.starts_with(' ') || (has_cjk && query_stem.chars().count() >= 2))
    {
        return Some(1);
    }
    if let Some(rest) = query_stem.strip_prefix(&cand_stem)
        && (rest.starts_with(' ') || (has_cjk && cand_stem.chars().count() >= 2))
    {
        return Some(1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_cross_media_work_stems() {
        assert_eq!(normalize_work_stem("铃芽之旅"), "铃芽之旅");
        assert_eq!(normalize_work_stem("铃芽之旅 原声带"), "铃芽之旅");
        assert_eq!(normalize_work_stem("铃芽之旅（官方小说）"), "铃芽之旅");
        assert_eq!(normalize_work_stem("铃芽之旅 第01卷"), "铃芽之旅");
        assert_eq!(normalize_work_stem("Dune: Part Two"), "dune");
        assert_eq!(
            normalize_work_stem("Dune (Original Motion Picture Soundtrack)"),
            "dune"
        );
        assert_eq!(normalize_work_stem("Dune - Book 1"), "dune");
        assert_eq!(
            stem_match_rank("dune", "Dune (Original Soundtrack)"),
            Some(0)
        );
        assert_eq!(stem_match_rank("dune", "Dunesday"), None);
    }
}
