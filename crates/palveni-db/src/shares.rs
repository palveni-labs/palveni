//! Public and password-protected share links (`share_links`) with atomic view
//! counting, expiry/revocation enforcement, and GPS coordinate stripping.

use std::collections::{HashMap, HashSet};

use palveni_core::collections::MediaCollectionEntryKind;
use palveni_core::shares::{PublicShareItem, ShareLinkSummary, ShareMediaKind, ShareTargetKind};
use palveni_core::{
    CollectionId, ItemId, LibraryId, PhotoAlbumId, ShareId, UserId, ids::PlaylistId,
};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};

use crate::convert::{now_ms, opt_ts, parse_id, parse_opt_id, truncate_chars, ts};
use crate::entities::{
    items, libraries,
    media_collection_items::{self as ci, Column as CICol},
    media_collections, music_albums, photo_album_items, photo_albums, photo_metadata,
    playlist_entries, playlists, reading_metadata,
    share_links::{self as sl, Column as SCol},
    track_metadata, video_metadata,
};
use crate::error::is_unique_violation;
use crate::{Db, DbError};

pub struct Shares<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct NewShareLink {
    pub token: String,
    pub creator_id: UserId,
    pub target_kind: ShareTargetKind,
    pub target_id: String,
    pub library_id: Option<LibraryId>,
    pub title: Option<String>,
    pub password_hash: Option<String>,
    pub allow_download: bool,
    pub strip_gps: bool,
    pub expires_at: Option<i64>,
    pub max_views: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct ShareRecord {
    pub id: ShareId,
    pub token: String,
    pub creator_id: UserId,
    pub target_kind: ShareTargetKind,
    pub target_id: String,
    pub library_id: Option<LibraryId>,
    pub title: String,
    pub password_hash: Option<String>,
    pub allow_download: bool,
    pub strip_gps: bool,
    pub expires_at: Option<i64>,
    pub max_views: Option<i32>,
    pub view_count: i64,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevokeOutcome {
    Revoked,
    NotFound,
    Forbidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShareUnavailableReason {
    Revoked,
    Expired,
    MaxViewsReached,
}

impl ShareRecord {
    pub fn check_availability(
        &self,
        now: i64,
        enforce_max_views: bool,
    ) -> Result<(), ShareUnavailableReason> {
        if self.revoked_at.is_some() {
            return Err(ShareUnavailableReason::Revoked);
        }
        if self.expires_at.is_some_and(|exp| exp <= now) {
            return Err(ShareUnavailableReason::Expired);
        }
        if enforce_max_views
            && self
                .max_views
                .is_some_and(|mv| self.view_count >= i64::from(mv))
        {
            return Err(ShareUnavailableReason::MaxViewsReached);
        }
        Ok(())
    }
}

impl Shares<'_> {
    pub async fn create(&self, input: NewShareLink) -> Result<ShareLinkSummary, DbError> {
        let token = input.token.trim();
        if token.len() < 12
            || token.len() > 64
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(DbError::Invalid("Invalid share token"));
        }
        if let Some(mv) = input.max_views
            && mv < 1
        {
            return Err(DbError::Invalid("maxViews must be at least 1"));
        }

        let (canonical_target_id, resolved_lib, default_title) = self
            .resolve_target_header(
                input.creator_id,
                input.target_kind,
                input.target_id.trim(),
                input.library_id,
            )
            .await?;

        let final_title = match input
            .title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(custom) => {
                if custom.chars().count() > 255 || custom.chars().any(char::is_control) {
                    return Err(DbError::Invalid("Invalid share title"));
                }
                custom.to_owned()
            }
            None => truncate_chars(&default_title, 255),
        };

        let now = now_ms();
        let id = ShareId::generate();
        let model = sl::Model {
            id: id.to_string(),
            token: token.to_owned(),
            creator_id: input.creator_id.to_string(),
            target_kind: input.target_kind.as_str().to_owned(),
            target_id: canonical_target_id,
            library_id: input.library_id.or(resolved_lib).map(|l| l.to_string()),
            title: final_title,
            password_hash: input.password_hash,
            allow_download: i32::from(input.allow_download),
            strip_gps: i32::from(input.strip_gps),
            expires_at: input.expires_at,
            max_views: input.max_views,
            view_count: 0,
            revoked_at: None,
            created_at: now,
        };

        sl::Entity::insert(sl::ActiveModel {
            id: ActiveValue::Set(model.id.clone()),
            token: ActiveValue::Set(model.token.clone()),
            creator_id: ActiveValue::Set(model.creator_id.clone()),
            target_kind: ActiveValue::Set(model.target_kind.clone()),
            target_id: ActiveValue::Set(model.target_id.clone()),
            library_id: ActiveValue::Set(model.library_id.clone()),
            title: ActiveValue::Set(model.title.clone()),
            password_hash: ActiveValue::Set(model.password_hash.clone()),
            allow_download: ActiveValue::Set(model.allow_download),
            strip_gps: ActiveValue::Set(model.strip_gps),
            expires_at: ActiveValue::Set(model.expires_at),
            max_views: ActiveValue::Set(model.max_views),
            view_count: ActiveValue::Set(0),
            revoked_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now),
        })
        .exec_without_returning(self.db.main())
        .await
        .map_err(|err| {
            if is_unique_violation(&err) {
                DbError::Conflict("Share token already exists".into())
            } else {
                DbError::from(err)
            }
        })?;

        to_summary(model, now)
    }

    pub async fn list_for_user(
        &self,
        user_id: UserId,
        is_admin: bool,
    ) -> Result<Vec<ShareLinkSummary>, DbError> {
        let mut q = sl::Entity::find();
        if !is_admin {
            q = q.filter(SCol::CreatorId.eq(user_id.to_string()));
        }
        let rows = q
            .order_by_desc(SCol::CreatedAt)
            .order_by_desc(SCol::Id)
            .limit(200)
            .all(self.db.main())
            .await?;
        let now = now_ms();
        rows.into_iter().map(|r| to_summary(r, now)).collect()
    }

    pub async fn revoke(
        &self,
        id: ShareId,
        actor_id: UserId,
        is_admin: bool,
    ) -> Result<RevokeOutcome, DbError> {
        let Some(row) = sl::Entity::find_by_id(id.to_string())
            .one(self.db.main())
            .await?
        else {
            return Ok(RevokeOutcome::NotFound);
        };
        if !is_admin && row.creator_id != actor_id.to_string() {
            return Ok(RevokeOutcome::Forbidden);
        }
        if row.revoked_at.is_none() {
            sl::Entity::update_many()
                .col_expr(SCol::RevokedAt, Expr::val(Some(now_ms())))
                .filter(SCol::Id.eq(id.to_string()))
                .filter(SCol::RevokedAt.is_null())
                .exec(self.db.main())
                .await?;
        }
        Ok(RevokeOutcome::Revoked)
    }

    pub async fn find_by_token(&self, token: &str) -> Result<Option<ShareRecord>, DbError> {
        let clean = token.trim();
        if clean.is_empty() || clean.len() > 64 {
            return Ok(None);
        }
        let Some(row) = sl::Entity::find()
            .filter(SCol::Token.eq(clean))
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        to_record(row).map(Some)
    }

    /// Atomically increments `view_count` iff the share is not revoked, not expired,
    /// and strictly below `max_views`.
    pub async fn consume_view(&self, id: ShareId, now: i64) -> Result<bool, DbError> {
        let cond = Condition::all()
            .add(SCol::Id.eq(id.to_string()))
            .add(SCol::RevokedAt.is_null())
            .add(
                Condition::any()
                    .add(SCol::ExpiresAt.is_null())
                    .add(SCol::ExpiresAt.gt(now)),
            )
            .add(
                Condition::any()
                    .add(SCol::MaxViews.is_null())
                    .add(Expr::col(SCol::ViewCount).lt(Expr::col(SCol::MaxViews))),
            );

        let res = sl::Entity::update_many()
            .col_expr(SCol::ViewCount, Expr::col(SCol::ViewCount).add(1))
            .filter(cond)
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    /// Resolves all viewable/playable items exposed by `share`, enforcing `strip_gps`
    /// on all photo coordinates so raw GPS values never leave the database when stripped.
    pub async fn resolve_items(
        &self,
        share: &ShareRecord,
    ) -> Result<Vec<PublicShareItem>, DbError> {
        match share.target_kind {
            ShareTargetKind::Item => {
                let Ok(item_id) = share.target_id.parse::<ItemId>() else {
                    return Ok(Vec::new());
                };
                Ok(self
                    .resolve_single_item(item_id, share.strip_gps)
                    .await?
                    .into_iter()
                    .collect())
            }
            ShareTargetKind::VideoWork => {
                self.resolve_video_work(&share.target_id, share.library_id)
                    .await
            }
            ShareTargetKind::MusicAlbum => {
                self.resolve_music_album(&share.target_id, share.library_id)
                    .await
            }
            ShareTargetKind::Playlist => {
                let Ok(pid) = share.target_id.parse::<PlaylistId>() else {
                    return Ok(Vec::new());
                };
                self.resolve_playlist(pid).await
            }
            ShareTargetKind::PhotoAlbum => {
                let Ok(aid) = share.target_id.parse::<PhotoAlbumId>() else {
                    return Ok(Vec::new());
                };
                self.resolve_photo_album(aid, share.strip_gps).await
            }
            ShareTargetKind::Collection => {
                let Ok(cid) = share.target_id.parse::<CollectionId>() else {
                    return Ok(Vec::new());
                };
                self.resolve_collection(cid, share.strip_gps).await
            }
        }
    }

    async fn resolve_target_header(
        &self,
        creator_id: UserId,
        kind: ShareTargetKind,
        target_id: &str,
        library_id: Option<LibraryId>,
    ) -> Result<(String, Option<LibraryId>, String), DbError> {
        if target_id.is_empty() || target_id.chars().count() > 128 {
            return Err(DbError::Invalid("Invalid targetId"));
        }
        match kind {
            ShareTargetKind::Item => {
                let item_id: ItemId = target_id.parse().map_err(|_| DbError::NotFound("item"))?;
                let item = items::Entity::find_by_id(item_id.to_string())
                    .filter(items::Column::DeletedAt.is_null())
                    .one(self.db.main())
                    .await?
                    .ok_or(DbError::NotFound("item"))?;
                let lib = parse_id(&item.library_id, "library id")?;
                Ok((item_id.to_string(), Some(lib), item.title))
            }
            ShareTargetKind::VideoWork => {
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
                let row = q
                    .one(self.db.main())
                    .await?
                    .ok_or(DbError::NotFound("video work"))?;
                let lib = parse_id(&row.library_id, "library id")?;
                Ok((row.work_key, Some(lib), row.work_title))
            }
            ShareTargetKind::MusicAlbum => {
                let (artist_key, album_key) = target_id
                    .split_once('.')
                    .ok_or(DbError::NotFound("music album"))?;
                let mut aq = music_albums::Entity::find()
                    .filter(music_albums::Column::ArtistKey.eq(artist_key))
                    .filter(music_albums::Column::AlbumKey.eq(album_key));
                if let Some(lib) = library_id {
                    aq = aq.filter(music_albums::Column::LibraryId.eq(lib.to_string()));
                }
                if let Some(album) = aq.one(self.db.main()).await? {
                    let lib = parse_id(&album.library_id, "library id")?;
                    let title = album.name.unwrap_or_else(|| "音乐专辑".into());
                    return Ok((target_id.to_owned(), Some(lib), title));
                }
                let mut tq = track_metadata::Entity::find()
                    .join(JoinType::InnerJoin, track_metadata::Relation::Item.def())
                    .filter(items::Column::DeletedAt.is_null())
                    .filter(track_metadata::Column::ArtistKey.eq(artist_key))
                    .filter(track_metadata::Column::AlbumKey.eq(album_key));
                if let Some(lib) = library_id {
                    tq = tq.filter(track_metadata::Column::LibraryId.eq(lib.to_string()));
                }
                let row = tq
                    .one(self.db.main())
                    .await?
                    .ok_or(DbError::NotFound("music album"))?;
                let lib = parse_id(&row.library_id, "library id")?;
                let title = row.album.unwrap_or(row.title);
                Ok((target_id.to_owned(), Some(lib), title))
            }
            ShareTargetKind::Playlist => {
                let pid: PlaylistId = target_id
                    .parse()
                    .map_err(|_| DbError::NotFound("playlist"))?;
                let pl = playlists::Entity::find_by_id(pid.to_string())
                    .filter(playlists::Column::Deleted.eq(false))
                    .filter(playlists::Column::UserId.eq(creator_id.to_string()))
                    .one(self.db.main())
                    .await?
                    .ok_or(DbError::NotFound("playlist"))?;
                Ok((pid.to_string(), None, pl.name))
            }
            ShareTargetKind::PhotoAlbum => {
                let aid: PhotoAlbumId = target_id
                    .parse()
                    .map_err(|_| DbError::NotFound("photo album"))?;
                let album = photo_albums::Entity::find_by_id(aid.to_string())
                    .filter(photo_albums::Column::UserId.eq(creator_id.to_string()))
                    .one(self.db.main())
                    .await?
                    .ok_or(DbError::NotFound("photo album"))?;
                Ok((aid.to_string(), None, album.title))
            }
            ShareTargetKind::Collection => {
                let cid: CollectionId = target_id
                    .parse()
                    .map_err(|_| DbError::NotFound("collection"))?;
                let coll = media_collections::Entity::find_by_id(cid.to_string())
                    .filter(media_collections::Column::OwnerId.eq(creator_id.to_string()))
                    .one(self.db.main())
                    .await?
                    .ok_or(DbError::NotFound("collection"))?;
                Ok((cid.to_string(), None, coll.title))
            }
        }
    }

    async fn resolve_single_item(
        &self,
        item_id: ItemId,
        strip_gps: bool,
    ) -> Result<Option<PublicShareItem>, DbError> {
        let Some(item) = items::Entity::find_by_id(item_id.to_string())
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(items::Column::DeletedAt.is_null())
            .filter(libraries::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };

        match item.kind.as_str() {
            "video" => {
                let meta = video_metadata::Entity::find_by_id(item.id.clone())
                    .one(self.db.main())
                    .await?;
                Ok(Some(PublicShareItem {
                    item_id,
                    media_kind: ShareMediaKind::Video,
                    title: meta
                        .as_ref()
                        .map_or_else(|| item.title.clone(), |m| m.title.clone()),
                    subtitle: meta.as_ref().and_then(format_video_subtitle),
                    duration_ms: meta.as_ref().and_then(|m| m.runtime_ms),
                    width: None,
                    height: None,
                    taken_at: None,
                    camera: None,
                    latitude: None,
                    longitude: None,
                    format: None,
                    size_bytes: item.size_bytes,
                    has_cover: true,
                }))
            }
            "track" => {
                let meta = track_metadata::Entity::find_by_id(item.id.clone())
                    .one(self.db.main())
                    .await?;
                Ok(Some(PublicShareItem {
                    item_id,
                    media_kind: ShareMediaKind::Audio,
                    title: meta
                        .as_ref()
                        .map_or_else(|| item.title.clone(), |m| m.title.clone()),
                    subtitle: meta.as_ref().and_then(|m| {
                        format_track_subtitle(m.artist.as_deref(), m.album.as_deref())
                    }),
                    duration_ms: meta.as_ref().and_then(|m| m.duration_ms),
                    width: None,
                    height: None,
                    taken_at: None,
                    camera: None,
                    latitude: None,
                    longitude: None,
                    format: meta.and_then(|m| m.codec),
                    size_bytes: item.size_bytes,
                    has_cover: true,
                }))
            }
            "photo" => {
                let meta = photo_metadata::Entity::find_by_id(item.id.clone())
                    .one(self.db.main())
                    .await?;
                Ok(Some(build_photo_share_item(
                    item_id,
                    item.title,
                    item.size_bytes,
                    meta.as_ref(),
                    strip_gps,
                )?))
            }
            "book" | "comic" => {
                let media_kind = if item.kind == "comic" {
                    ShareMediaKind::Comic
                } else {
                    ShareMediaKind::Book
                };
                let meta = reading_metadata::Entity::find_by_id(item.id.clone())
                    .one(self.db.main())
                    .await?;
                Ok(Some(PublicShareItem {
                    item_id,
                    media_kind,
                    title: meta
                        .as_ref()
                        .map_or_else(|| item.title.clone(), |m| m.title.clone()),
                    subtitle: meta.as_ref().and_then(|m| {
                        crate::collections::parse_first_author(&m.authors_json)
                            .or_else(|| m.series_name.clone())
                    }),
                    duration_ms: None,
                    width: None,
                    height: None,
                    taken_at: None,
                    camera: None,
                    latitude: None,
                    longitude: None,
                    format: meta.as_ref().map(|m| m.format.clone()),
                    size_bytes: item.size_bytes,
                    has_cover: meta.is_none_or(|m| m.has_cover),
                }))
            }
            _ => Ok(None),
        }
    }

    async fn resolve_video_work(
        &self,
        work_key: &str,
        library_id: Option<LibraryId>,
    ) -> Result<Vec<PublicShareItem>, DbError> {
        let mut q = video_metadata::Entity::find()
            .join(JoinType::InnerJoin, video_metadata::Relation::Item.def())
            .filter(items::Column::DeletedAt.is_null())
            .filter(video_metadata::Column::WorkKey.eq(work_key));
        if let Some(lib) = library_id {
            q = q.filter(video_metadata::Column::LibraryId.eq(lib.to_string()));
        }
        let rows = q
            .order_by_asc(video_metadata::Column::Season)
            .order_by_asc(video_metadata::Column::EpisodeSort)
            .order_by_asc(video_metadata::Column::ItemId)
            .limit(250)
            .select_only()
            .columns([
                video_metadata::Column::ItemId,
                video_metadata::Column::WorkTitle,
                video_metadata::Column::Title,
                video_metadata::Column::Season,
                video_metadata::Column::Episode,
                video_metadata::Column::WorkYear,
                video_metadata::Column::RuntimeMs,
            ])
            .column(items::Column::SizeBytes)
            .into_tuple::<(
                String,
                String,
                String,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i64>,
                Option<i64>,
            )>()
            .all(self.db.main())
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for (item_id, work_title, ep_title, season, episode, year, runtime_ms, size_bytes) in rows {
            let subtitle = match (season, episode) {
                (Some(s), Some(e)) => Some(format!("S{s:02}E{e:02}")),
                (Some(s), None) => Some(format!("S{s:02}")),
                _ => year.map(|y| y.to_string()),
            };
            let title = if season.is_some() {
                ep_title
            } else {
                work_title
            };
            out.push(PublicShareItem {
                item_id: parse_id(&item_id, "video item id")?,
                media_kind: ShareMediaKind::Video,
                title,
                subtitle,
                duration_ms: runtime_ms,
                width: None,
                height: None,
                taken_at: None,
                camera: None,
                latitude: None,
                longitude: None,
                format: None,
                size_bytes,
                has_cover: true,
            });
        }
        Ok(out)
    }

    async fn resolve_music_album(
        &self,
        album_id: &str,
        library_id: Option<LibraryId>,
    ) -> Result<Vec<PublicShareItem>, DbError> {
        let Some((artist_key, album_key)) = album_id.split_once('.') else {
            return Ok(Vec::new());
        };
        let mut q = track_metadata::Entity::find()
            .join(JoinType::InnerJoin, track_metadata::Relation::Item.def())
            .filter(items::Column::DeletedAt.is_null())
            .filter(track_metadata::Column::ArtistKey.eq(artist_key))
            .filter(track_metadata::Column::AlbumKey.eq(album_key));
        if let Some(lib) = library_id {
            q = q.filter(track_metadata::Column::LibraryId.eq(lib.to_string()));
        }
        let rows = q
            .order_by_asc(track_metadata::Column::DiscNumber)
            .order_by_asc(track_metadata::Column::TrackNumber)
            .order_by_asc(track_metadata::Column::TrackSort)
            .limit(300)
            .select_only()
            .columns([
                track_metadata::Column::ItemId,
                track_metadata::Column::Title,
                track_metadata::Column::Artist,
                track_metadata::Column::Album,
                track_metadata::Column::DurationMs,
                track_metadata::Column::Codec,
            ])
            .column(items::Column::SizeBytes)
            .into_tuple::<(
                String,
                String,
                Option<String>,
                Option<String>,
                Option<i64>,
                Option<String>,
                Option<i64>,
            )>()
            .all(self.db.main())
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for (item_id, title, artist, album, duration_ms, codec, size_bytes) in rows {
            out.push(PublicShareItem {
                item_id: parse_id(&item_id, "track item id")?,
                media_kind: ShareMediaKind::Audio,
                title,
                subtitle: format_track_subtitle(artist.as_deref(), album.as_deref()),
                duration_ms,
                width: None,
                height: None,
                taken_at: None,
                camera: None,
                latitude: None,
                longitude: None,
                format: codec,
                size_bytes,
                has_cover: true,
            });
        }
        Ok(out)
    }

    async fn resolve_playlist(
        &self,
        playlist_id: PlaylistId,
    ) -> Result<Vec<PublicShareItem>, DbError> {
        let Some(_pl) = playlists::Entity::find_by_id(playlist_id.to_string())
            .filter(playlists::Column::Deleted.eq(false))
            .one(self.db.main())
            .await?
        else {
            return Ok(Vec::new());
        };
        let rows = playlist_entries::Entity::find()
            .join(JoinType::InnerJoin, playlist_entries::Relation::Item.def())
            .join_rev(JoinType::LeftJoin, track_metadata::Relation::Item.def())
            .filter(playlist_entries::Column::PlaylistId.eq(playlist_id.to_string()))
            .filter(items::Column::DeletedAt.is_null())
            .order_by_asc(playlist_entries::Column::Position)
            .limit(300)
            .select_only()
            .column(items::Column::Id)
            .column_as(items::Column::Title, "item_title")
            .column(items::Column::SizeBytes)
            .column(track_metadata::Column::Title)
            .column(track_metadata::Column::Artist)
            .column(track_metadata::Column::Album)
            .column(track_metadata::Column::DurationMs)
            .column(track_metadata::Column::Codec)
            .into_tuple::<(
                String,
                String,
                Option<i64>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<i64>,
                Option<String>,
            )>()
            .all(self.db.main())
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for (id, item_title, size_bytes, meta_title, artist, album, duration_ms, codec) in rows {
            out.push(PublicShareItem {
                item_id: parse_id(&id, "track id")?,
                media_kind: ShareMediaKind::Audio,
                title: meta_title.unwrap_or(item_title),
                subtitle: format_track_subtitle(artist.as_deref(), album.as_deref()),
                duration_ms,
                width: None,
                height: None,
                taken_at: None,
                camera: None,
                latitude: None,
                longitude: None,
                format: codec,
                size_bytes,
                has_cover: true,
            });
        }
        Ok(out)
    }

    async fn resolve_photo_album(
        &self,
        album_id: PhotoAlbumId,
        strip_gps: bool,
    ) -> Result<Vec<PublicShareItem>, DbError> {
        let entries = photo_album_items::Entity::find()
            .join(JoinType::InnerJoin, photo_album_items::Relation::Item.def())
            .filter(photo_album_items::Column::AlbumId.eq(album_id.to_string()))
            .filter(items::Column::DeletedAt.is_null())
            .order_by_asc(photo_album_items::Column::Position)
            .limit(500)
            .select_only()
            .columns([
                items::Column::Id,
                items::Column::Title,
                items::Column::SizeBytes,
            ])
            .into_tuple::<(String, String, Option<i64>)>()
            .all(self.db.main())
            .await?;

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let item_ids: Vec<String> = entries.iter().map(|(id, _, _)| id.clone()).collect();
        let meta_map: HashMap<String, photo_metadata::Model> = photo_metadata::Entity::find()
            .filter(photo_metadata::Column::ItemId.is_in(item_ids))
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|m| (m.item_id.clone(), m))
            .collect();

        let mut out = Vec::with_capacity(entries.len());
        for (id, title, size_bytes) in entries {
            let item_id = parse_id(&id, "photo item id")?;
            out.push(build_photo_share_item(
                item_id,
                title,
                size_bytes,
                meta_map.get(&id),
                strip_gps,
            )?);
        }
        Ok(out)
    }

    async fn resolve_collection(
        &self,
        collection_id: CollectionId,
        strip_gps: bool,
    ) -> Result<Vec<PublicShareItem>, DbError> {
        let rows = ci::Entity::find()
            .filter(CICol::CollectionId.eq(collection_id.to_string()))
            .order_by_asc(CICol::Position)
            .order_by_asc(CICol::Id)
            .limit(200)
            .all(self.db.main())
            .await?;

        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for row in rows {
            let Ok(kind) = row.entry_kind.parse::<MediaCollectionEntryKind>() else {
                continue;
            };
            let lib_id = parse_opt_id(row.library_id.as_deref(), "library id")?;
            let sub_items = match kind {
                MediaCollectionEntryKind::Video => {
                    self.resolve_video_work(&row.target_id, lib_id).await?
                }
                MediaCollectionEntryKind::MusicAlbum => {
                    self.resolve_music_album(&row.target_id, lib_id).await?
                }
                MediaCollectionEntryKind::Track
                | MediaCollectionEntryKind::Book
                | MediaCollectionEntryKind::Comic
                | MediaCollectionEntryKind::Photo => {
                    if let Ok(iid) = row.target_id.parse::<ItemId>() {
                        self.resolve_single_item(iid, strip_gps)
                            .await?
                            .into_iter()
                            .collect()
                    } else {
                        Vec::new()
                    }
                }
            };
            for item in sub_items {
                if seen.insert(item.item_id) {
                    out.push(item);
                }
            }
        }
        Ok(out)
    }
}

fn build_photo_share_item(
    item_id: ItemId,
    title: String,
    size_bytes: Option<i64>,
    meta: Option<&photo_metadata::Model>,
    strip_gps: bool,
) -> Result<PublicShareItem, DbError> {
    let camera = meta.and_then(|m| {
        let make = m.camera_make.as_deref().unwrap_or("").trim();
        let model = m.camera_model.as_deref().unwrap_or("").trim();
        if make.is_empty() && model.is_empty() {
            None
        } else if model.to_lowercase().starts_with(&make.to_lowercase()) || make.is_empty() {
            Some(model.to_owned())
        } else if model.is_empty() {
            Some(make.to_owned())
        } else {
            Some(format!("{make} {model}"))
        }
    });
    let taken_at = match meta {
        Some(m) => Some(ts(m.taken_at)?),
        None => None,
    };
    Ok(PublicShareItem {
        item_id,
        media_kind: ShareMediaKind::Photo,
        title,
        subtitle: camera.clone(),
        duration_ms: None,
        width: meta.and_then(|m| m.width),
        height: meta.and_then(|m| m.height),
        taken_at,
        camera,
        latitude: if strip_gps {
            None
        } else {
            meta.and_then(|m| m.latitude)
        },
        longitude: if strip_gps {
            None
        } else {
            meta.and_then(|m| m.longitude)
        },
        format: None,
        size_bytes,
        has_cover: true,
    })
}

fn format_video_subtitle(m: &video_metadata::Model) -> Option<String> {
    match (m.season, m.episode) {
        (Some(s), Some(e)) => Some(format!("{} · S{s:02}E{e:02}", m.work_title)),
        _ => m.work_year.map(|y| y.to_string()),
    }
}

fn format_track_subtitle(artist: Option<&str>, album: Option<&str>) -> Option<String> {
    let parts: Vec<&str> = [artist, album]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn to_record(row: sl::Model) -> Result<ShareRecord, DbError> {
    let target_kind: ShareTargetKind = row.target_kind.parse().map_err(|_| {
        DbError::Corrupt(format!("invalid share target_kind `{}`", row.target_kind))
    })?;
    Ok(ShareRecord {
        id: parse_id(&row.id, "share id")?,
        token: row.token,
        creator_id: parse_id(&row.creator_id, "creator id")?,
        target_kind,
        target_id: row.target_id,
        library_id: parse_opt_id(row.library_id.as_deref(), "library id")?,
        title: row.title,
        password_hash: row.password_hash,
        allow_download: row.allow_download != 0,
        strip_gps: row.strip_gps != 0,
        expires_at: row.expires_at,
        max_views: row.max_views,
        view_count: row.view_count,
        revoked_at: row.revoked_at,
        created_at: row.created_at,
    })
}

fn to_summary(row: sl::Model, now: i64) -> Result<ShareLinkSummary, DbError> {
    let rec = to_record(row)?;
    let expired = rec.expires_at.is_some_and(|exp| exp <= now)
        || rec
            .max_views
            .is_some_and(|mv| rec.view_count >= i64::from(mv));
    Ok(ShareLinkSummary {
        id: rec.id,
        share_path: format!("/s/{}", rec.token),
        token: rec.token,
        creator_id: rec.creator_id,
        target_kind: rec.target_kind,
        target_id: rec.target_id,
        library_id: rec.library_id,
        title: rec.title,
        has_password: rec.password_hash.is_some(),
        allow_download: rec.allow_download,
        strip_gps: rec.strip_gps,
        expires_at: opt_ts(rec.expires_at)?,
        max_views: rec.max_views,
        view_count: rec.view_count,
        revoked_at: opt_ts(rec.revoked_at)?,
        expired,
        created_at: ts(rec.created_at)?,
    })
}
