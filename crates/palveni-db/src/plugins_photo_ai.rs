//! Persistence for installed recipe/widget plugins, local photo AI tags,
//! user-scoped photo people profiles and photo-to-person tags.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use palveni_core::photos::{PhotoPersonRef, PhotoPersonSummary};
use palveni_core::plugins::PluginRuntimeKind;
use palveni_core::{ItemId, ItemKind, LibraryId, LibraryKind, PhotoPersonId, UserId, clock};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait, JoinType,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};
use serde_json::Value;

use crate::convert::{now_ms, parse_id, parse_opt_id, truncate_chars, ts};
use crate::entities::{
    items, libraries,
    photo_ai_tag::{self as ai_tag, Column as PAT},
    photo_album_items::{self as album_items, Column as AI},
    photo_albums::{self as albums, Column as A},
    photo_metadata::{self as meta, Column as M},
    photo_person::{self as people, Column as PP},
    photo_person_tag::{self as person_tags, Column as PPT},
    plugin_installation::{self as plugin_inst, Column as PI},
    user_item_state::{self as item_state, Column as S},
};
use crate::items::SORT_GROUP_LEAF;
use crate::{Db, DbError};

pub struct PluginsPhotoAi<'a> {
    pub(crate) db: &'a Db,
}

// ---------------------------------------------------------------------------
// Plugin installation records
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PluginInstallationRecord {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub previous_version: Option<String>,
    pub runtime: PluginRuntimeKind,
    pub provides: Vec<String>,
    pub permissions: Vec<String>,
    pub granted_permissions: Vec<String>,
    pub settings_schema: Value,
    pub settings: Value,
    pub enabled: bool,
    pub source: String,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct PluginInstallationUpsert {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub previous_version: Option<String>,
    pub runtime: PluginRuntimeKind,
    pub provides: Vec<String>,
    pub permissions: Vec<String>,
    pub granted_permissions: Vec<String>,
    pub settings_schema: Value,
    pub settings: Value,
    pub enabled: bool,
    pub source: String,
}

fn parse_string_vec(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn parse_json_object(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
}

fn encode_json<T: serde::Serialize>(value: &T, what: &str) -> Result<String, DbError> {
    serde_json::to_string(value)
        .map_err(|err| DbError::Internal(format!("cannot encode {what}: {err}")))
}

fn plugin_from_model(row: plugin_inst::Model) -> Result<PluginInstallationRecord, DbError> {
    Ok(PluginInstallationRecord {
        plugin_id: row.plugin_id,
        name: row.name,
        version: row.version,
        previous_version: row.previous_version,
        runtime: PluginRuntimeKind::parse(&row.runtime),
        provides: parse_string_vec(&row.provides_json),
        permissions: parse_string_vec(&row.permissions_json),
        granted_permissions: parse_string_vec(&row.granted_permissions_json),
        settings_schema: parse_json_object(&row.settings_schema_json),
        settings: parse_json_object(&row.settings_json),
        enabled: row.enabled != 0,
        source: row.source,
        installed_at: ts(row.installed_at)?,
        updated_at: ts(row.updated_at)?,
    })
}

// ---------------------------------------------------------------------------
// Photo AI & People DTOs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct UnanalyzedPhotoRow {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub path: String,
    pub modified_at: Option<i64>,
    pub taken_at: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub orientation: i32,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct PhotoAiTagUpsert {
    pub item_id: ItemId,
    pub dhash: Option<i64>,
    pub dominant_color: Option<String>,
    pub dominant_hex: Option<String>,
    pub scene_tags: Vec<String>,
    pub time_of_day: Option<String>,
    pub season: Option<String>,
    pub analyzed_at: i64,
}

#[derive(Clone, Debug)]
pub struct PhotoAiTagRecord {
    pub item_id: ItemId,
    pub dhash: Option<i64>,
    pub dominant_color: Option<String>,
    pub dominant_hex: Option<String>,
    pub scene_tags: Vec<String>,
    pub time_of_day: Option<String>,
    pub season: Option<String>,
    pub analyzed_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct PhotoAiCandidateRow {
    pub id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub file_name: String,
    pub path: Option<String>,
    pub size_bytes: Option<i64>,
    pub taken_at_ms: i64,
    pub taken_at: DateTime<Utc>,
    pub day_bucket: String,
    pub month_bucket: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub orientation: i32,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub content_hash: Option<String>,
    pub favorite: bool,
    pub dhash: Option<i64>,
    pub dominant_color: Option<String>,
    pub dominant_hex: Option<String>,
    pub scene_tags: Vec<String>,
    pub time_of_day: Option<String>,
    pub season: Option<String>,
    pub album_titles: Vec<String>,
    pub people: Vec<PhotoPersonRef>,
}

#[derive(sea_orm::FromQueryResult)]
struct UnanalyzedRowRaw {
    item_id: String,
    library_id: String,
    path: Option<String>,
    modified_at: Option<i64>,
    taken_at: Option<i64>,
    width: Option<i32>,
    height: Option<i32>,
    orientation: Option<i32>,
    camera_make: Option<String>,
    camera_model: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(sea_orm::FromQueryResult)]
struct PhotoCandidateRaw {
    item_id: String,
    library_id: String,
    title: String,
    path: Option<String>,
    size_bytes: Option<i64>,
    modified_at: Option<i64>,
    created_at: i64,
    taken_at: Option<i64>,
    day_bucket: Option<String>,
    month_bucket: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    orientation: Option<i32>,
    camera_make: Option<String>,
    camera_model: Option<String>,
    lens_model: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    content_hash: Option<String>,
    favorite_order: Option<i64>,
    dhash: Option<i64>,
    dominant_color: Option<String>,
    dominant_hex: Option<String>,
    scene_tags_json: Option<String>,
    time_of_day: Option<String>,
    season: Option<String>,
}

fn allowed_lib_strings(allowed: Option<&HashSet<LibraryId>>) -> Option<Vec<String>> {
    allowed.map(|set| set.iter().map(ToString::to_string).collect())
}

fn file_name_from_path(path: Option<&str>, fallback_title: &str) -> String {
    path.and_then(|p| p.rsplit(['/', '\\']).next())
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_title)
        .to_owned()
}

fn validate_person_name(raw: &str) -> Result<String, DbError> {
    let trimmed = raw.trim();
    let count = trimmed.chars().count();
    if count == 0 || count > 128 {
        return Err(DbError::Invalid("Person name must be 1 to 128 characters"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(DbError::Invalid(
            "Person name must not contain control characters",
        ));
    }
    Ok(trimmed.to_owned())
}

impl PluginsPhotoAi<'_> {
    // -----------------------------------------------------------------------
    // Installed Recipe & Widget Plugins
    // -----------------------------------------------------------------------

    pub async fn list_installed_plugins(&self) -> Result<Vec<PluginInstallationRecord>, DbError> {
        let rows = plugin_inst::Entity::find()
            .order_by_asc(PI::PluginId)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(plugin_from_model).collect()
    }

    pub async fn get_installed_plugin(
        &self,
        plugin_id: &str,
    ) -> Result<Option<PluginInstallationRecord>, DbError> {
        let row = plugin_inst::Entity::find_by_id(plugin_id.to_owned())
            .one(self.db.main())
            .await?;
        row.map(plugin_from_model).transpose()
    }

    pub async fn upsert_plugin_installation(
        &self,
        input: PluginInstallationUpsert,
    ) -> Result<PluginInstallationRecord, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let now = now_ms();
        let existing = plugin_inst::Entity::find_by_id(input.plugin_id.clone())
            .one(&txn)
            .await?;

        let provides_json = encode_json(&input.provides, "provides")?;
        let permissions_json = encode_json(&input.permissions, "permissions")?;
        let granted_json = encode_json(&input.granted_permissions, "granted_permissions")?;
        let schema_json = encode_json(&input.settings_schema, "settings_schema")?;
        let settings_json = encode_json(&input.settings, "settings")?;

        let (installed_at, previous_version) = match existing {
            Some(prev) => {
                let next_prev = if prev.version != input.version {
                    Some(prev.version)
                } else {
                    prev.previous_version.or(input.previous_version)
                };
                (prev.installed_at, next_prev)
            }
            None => (now, input.previous_version),
        };

        let active = plugin_inst::ActiveModel {
            plugin_id: Set(truncate_chars(&input.plugin_id, 128)),
            name: Set(truncate_chars(&input.name, 128)),
            version: Set(truncate_chars(&input.version, 32)),
            previous_version: Set(previous_version.as_deref().map(|v| truncate_chars(v, 32))),
            runtime: Set(truncate_chars(input.runtime.as_str(), 32)),
            provides_json: Set(provides_json),
            permissions_json: Set(permissions_json),
            granted_permissions_json: Set(granted_json),
            settings_schema_json: Set(schema_json),
            settings_json: Set(settings_json),
            enabled: Set(i32::from(input.enabled)),
            source: Set(truncate_chars(&input.source, 32)),
            installed_at: Set(installed_at),
            updated_at: Set(now),
        };

        plugin_inst::Entity::insert(active)
            .on_conflict(
                OnConflict::column(PI::PluginId)
                    .update_columns([
                        PI::Name,
                        PI::Version,
                        PI::PreviousVersion,
                        PI::Runtime,
                        PI::ProvidesJson,
                        PI::PermissionsJson,
                        PI::GrantedPermissionsJson,
                        PI::SettingsSchemaJson,
                        PI::SettingsJson,
                        PI::Enabled,
                        PI::Source,
                        PI::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&txn)
            .await?;

        let stored = plugin_inst::Entity::find_by_id(truncate_chars(&input.plugin_id, 128))
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("plugin"))?;
        txn.commit().await?;
        plugin_from_model(stored)
    }

    pub async fn update_plugin_installation(
        &self,
        plugin_id: &str,
        enabled: Option<bool>,
        settings: Option<Value>,
    ) -> Result<PluginInstallationRecord, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let existing = plugin_inst::Entity::find_by_id(plugin_id.to_owned())
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("plugin"))?;

        let mut active: plugin_inst::ActiveModel = existing.into();
        if let Some(flag) = enabled {
            active.enabled = Set(i32::from(flag));
        }
        if let Some(value) = settings {
            active.settings_json = Set(encode_json(&value, "settings")?);
        }
        active.updated_at = Set(now_ms());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        plugin_from_model(updated)
    }

    pub async fn rollback_plugin_installation(
        &self,
        plugin_id: &str,
    ) -> Result<PluginInstallationRecord, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let existing = plugin_inst::Entity::find_by_id(plugin_id.to_owned())
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("plugin"))?;

        let Some(prev_version) = existing.previous_version.clone() else {
            txn.rollback().await?;
            return Err(DbError::Invalid(
                "No previous version is available to roll back",
            ));
        };

        let mut active: plugin_inst::ActiveModel = existing.into();
        active.version = Set(prev_version);
        active.previous_version = Set(None);
        active.updated_at = Set(now_ms());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        plugin_from_model(updated)
    }

    pub async fn delete_plugin_installation(&self, plugin_id: &str) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let res = plugin_inst::Entity::delete_by_id(plugin_id.to_owned())
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(res.rows_affected > 0)
    }

    // -----------------------------------------------------------------------
    // Photo AI Tags
    // -----------------------------------------------------------------------

    pub async fn list_unanalyzed_photos(
        &self,
        allowed_library_ids: Option<&HashSet<LibraryId>>,
        limit: usize,
    ) -> Result<Vec<UnanalyzedPhotoRow>, DbError> {
        let allowed = allowed_lib_strings(allowed_library_ids);
        if let Some(ref libs) = allowed
            && libs.is_empty()
        {
            return Ok(Vec::new());
        }

        let mut select = items::Entity::find()
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::LeftJoin,
                meta::Entity::belongs_to(items::Entity)
                    .from(M::ItemId)
                    .to(items::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::LeftJoin,
                ai_tag::Entity::belongs_to(items::Entity)
                    .from(PAT::ItemId)
                    .to(items::Column::Id)
                    .into(),
            )
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::Path.is_not_null())
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()))
            .filter(PAT::ItemId.is_null());

        if let Some(libs) = allowed {
            select = select.filter(items::Column::LibraryId.is_in(libs));
        }

        let rows = select
            .select_only()
            .column_as(items::Column::Id, "item_id")
            .column_as(items::Column::LibraryId, "library_id")
            .column_as(items::Column::Path, "path")
            .column_as(items::Column::ModifiedAt, "modified_at")
            .column_as(M::TakenAt, "taken_at")
            .column_as(M::Width, "width")
            .column_as(M::Height, "height")
            .column_as(M::Orientation, "orientation")
            .column_as(M::CameraMake, "camera_make")
            .column_as(M::CameraModel, "camera_model")
            .column_as(M::Latitude, "latitude")
            .column_as(M::Longitude, "longitude")
            .order_by_desc(items::Column::Id)
            .limit(u64::try_from(limit.max(1)).unwrap_or(200))
            .into_model::<UnanalyzedRowRaw>()
            .all(self.db.main())
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(path) = row.path else { continue };
            out.push(UnanalyzedPhotoRow {
                item_id: parse_id(&row.item_id, "item id")?,
                library_id: parse_id(&row.library_id, "library id")?,
                path,
                modified_at: row.modified_at,
                taken_at: row.taken_at,
                width: row.width,
                height: row.height,
                orientation: row.orientation.unwrap_or(1),
                camera_make: row.camera_make,
                camera_model: row.camera_model,
                latitude: row.latitude,
                longitude: row.longitude,
            });
        }
        Ok(out)
    }

    pub async fn count_photo_ai_progress(
        &self,
        allowed_library_ids: Option<&HashSet<LibraryId>>,
    ) -> Result<(u64, u64), DbError> {
        let allowed = allowed_lib_strings(allowed_library_ids);
        if let Some(ref libs) = allowed
            && libs.is_empty()
        {
            return Ok((0, 0));
        }

        let base = || {
            let mut sel = items::Entity::find()
                .join(
                    JoinType::InnerJoin,
                    items::Entity::belongs_to(libraries::Entity)
                        .from(items::Column::LibraryId)
                        .to(libraries::Column::Id)
                        .into(),
                )
                .join_rev(
                    JoinType::LeftJoin,
                    ai_tag::Entity::belongs_to(items::Entity)
                        .from(PAT::ItemId)
                        .to(items::Column::Id)
                        .into(),
                )
                .filter(items::Column::DeletedAt.is_null())
                .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
                .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
                .filter(items::Column::Path.is_not_null())
                .filter(libraries::Column::DeletedAt.is_null())
                .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()));
            if let Some(ref libs) = allowed {
                sel = sel.filter(items::Column::LibraryId.is_in(libs.clone()));
            }
            sel
        };

        let tagged = base()
            .filter(PAT::ItemId.is_not_null())
            .count(self.db.main())
            .await?;
        let remaining = base()
            .filter(PAT::ItemId.is_null())
            .count(self.db.main())
            .await?;
        Ok((tagged, remaining))
    }

    pub async fn upsert_photo_ai_tags(&self, rows: &[PhotoAiTagUpsert]) -> Result<(), DbError> {
        if rows.is_empty() {
            return Ok(());
        }
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let mut models = Vec::with_capacity(rows.len());
        for row in rows {
            let tags_json = encode_json(&row.scene_tags, "scene_tags")?;
            models.push(ai_tag::ActiveModel {
                item_id: Set(row.item_id.to_string()),
                dhash: Set(row.dhash),
                dominant_color: Set(row.dominant_color.as_deref().map(|s| truncate_chars(s, 32))),
                dominant_hex: Set(row.dominant_hex.as_deref().map(|s| truncate_chars(s, 16))),
                scene_tags_json: Set(tags_json),
                time_of_day: Set(row.time_of_day.as_deref().map(|s| truncate_chars(s, 32))),
                season: Set(row.season.as_deref().map(|s| truncate_chars(s, 32))),
                analyzed_at: Set(row.analyzed_at),
            });
        }

        ai_tag::Entity::insert_many(models)
            .on_conflict(
                OnConflict::column(PAT::ItemId)
                    .update_columns([
                        PAT::Dhash,
                        PAT::DominantColor,
                        PAT::DominantHex,
                        PAT::SceneTagsJson,
                        PAT::TimeOfDay,
                        PAT::Season,
                        PAT::AnalyzedAt,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    pub async fn get_photo_ai_tag(
        &self,
        item_id: ItemId,
    ) -> Result<Option<PhotoAiTagRecord>, DbError> {
        let row = ai_tag::Entity::find_by_id(item_id.to_string())
            .one(self.db.main())
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(PhotoAiTagRecord {
            item_id: parse_id(&row.item_id, "item id")?,
            dhash: row.dhash,
            dominant_color: row.dominant_color,
            dominant_hex: row.dominant_hex,
            scene_tags: parse_string_vec(&row.scene_tags_json),
            time_of_day: row.time_of_day,
            season: row.season,
            analyzed_at: ts(row.analyzed_at)?,
        }))
    }

    pub async fn list_accessible_photos_for_ai(
        &self,
        user_id: UserId,
        allowed_library_ids: Option<&HashSet<LibraryId>>,
    ) -> Result<Vec<PhotoAiCandidateRow>, DbError> {
        let allowed = allowed_lib_strings(allowed_library_ids);
        if let Some(ref libs) = allowed
            && libs.is_empty()
        {
            return Ok(Vec::new());
        }

        let uid = user_id.to_string();
        let mut select = items::Entity::find()
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::LeftJoin,
                meta::Entity::belongs_to(items::Entity)
                    .from(M::ItemId)
                    .to(items::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::LeftJoin,
                ai_tag::Entity::belongs_to(items::Entity)
                    .from(PAT::ItemId)
                    .to(items::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::LeftJoin,
                item_state::Entity::belongs_to(items::Entity)
                    .from(S::ItemId)
                    .to(items::Column::Id)
                    .on_condition(move |_left, _right| {
                        Condition::all()
                            .add(Expr::col((item_state::Entity, S::UserId)).eq(uid.clone()))
                    })
                    .into(),
            )
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()));

        if let Some(libs) = allowed {
            select = select.filter(items::Column::LibraryId.is_in(libs));
        }

        let rows = select
            .select_only()
            .column_as(items::Column::Id, "item_id")
            .column_as(items::Column::LibraryId, "library_id")
            .column_as(items::Column::Title, "title")
            .column_as(items::Column::Path, "path")
            .column_as(items::Column::SizeBytes, "size_bytes")
            .column_as(items::Column::ModifiedAt, "modified_at")
            .column_as(items::Column::CreatedAt, "created_at")
            .column_as(M::TakenAt, "taken_at")
            .column_as(M::DayBucket, "day_bucket")
            .column_as(M::MonthBucket, "month_bucket")
            .column_as(M::Width, "width")
            .column_as(M::Height, "height")
            .column_as(M::Orientation, "orientation")
            .column_as(M::CameraMake, "camera_make")
            .column_as(M::CameraModel, "camera_model")
            .column_as(M::LensModel, "lens_model")
            .column_as(M::Latitude, "latitude")
            .column_as(M::Longitude, "longitude")
            .column_as(M::ContentHash, "content_hash")
            .column_as(S::FavoriteOrder, "favorite_order")
            .column_as(PAT::Dhash, "dhash")
            .column_as(PAT::DominantColor, "dominant_color")
            .column_as(PAT::DominantHex, "dominant_hex")
            .column_as(PAT::SceneTagsJson, "scene_tags_json")
            .column_as(PAT::TimeOfDay, "time_of_day")
            .column_as(PAT::Season, "season")
            .into_model::<PhotoCandidateRaw>()
            .all(self.db.main())
            .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Load user's album memberships
        let album_rows = albums::Entity::find()
            .filter(A::UserId.eq(user_id.to_string()))
            .join_rev(
                JoinType::InnerJoin,
                album_items::Entity::belongs_to(albums::Entity)
                    .from(AI::AlbumId)
                    .to(A::Id)
                    .into(),
            )
            .select_only()
            .column(AI::ItemId)
            .column(A::Title)
            .into_tuple::<(String, String)>()
            .all(self.db.main())
            .await?;
        let mut albums_by_item: HashMap<String, Vec<String>> = HashMap::new();
        for (item_id, title) in album_rows {
            albums_by_item.entry(item_id).or_default().push(title);
        }

        // Load user's photo people tags
        let person_rows = people::Entity::find()
            .filter(PP::OwnerId.eq(user_id.to_string()))
            .join_rev(
                JoinType::InnerJoin,
                person_tags::Entity::belongs_to(people::Entity)
                    .from(PPT::PersonId)
                    .to(PP::Id)
                    .into(),
            )
            .select_only()
            .column(PPT::ItemId)
            .column(PP::Id)
            .column(PP::Name)
            .order_by_asc(PP::Name)
            .into_tuple::<(String, String, String)>()
            .all(self.db.main())
            .await?;
        let mut people_by_item: HashMap<String, Vec<PhotoPersonRef>> = HashMap::new();
        for (item_id, pid, name) in person_rows {
            people_by_item
                .entry(item_id)
                .or_default()
                .push(PhotoPersonRef {
                    id: parse_id(&pid, "person id")?,
                    name,
                });
        }

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let taken_ms = row.taken_at.or(row.modified_at).unwrap_or(row.created_at);
            let taken_dt = ts(taken_ms)?;
            let day_bucket = row
                .day_bucket
                .unwrap_or_else(|| taken_dt.format("%Y-%m-%d").to_string());
            let month_bucket = row
                .month_bucket
                .unwrap_or_else(|| taken_dt.format("%Y-%m").to_string());
            let file_name = file_name_from_path(row.path.as_deref(), &row.title);
            let item_key = row.item_id.clone();

            out.push(PhotoAiCandidateRow {
                id: parse_id(&row.item_id, "item id")?,
                library_id: parse_id(&row.library_id, "library id")?,
                title: row.title,
                file_name,
                path: row.path,
                size_bytes: row.size_bytes,
                taken_at_ms: taken_ms,
                taken_at: taken_dt,
                day_bucket,
                month_bucket,
                width: row.width,
                height: row.height,
                orientation: row.orientation.unwrap_or(1),
                camera_make: row.camera_make,
                camera_model: row.camera_model,
                lens_model: row.lens_model,
                latitude: row.latitude,
                longitude: row.longitude,
                content_hash: row.content_hash,
                favorite: row.favorite_order.is_some_and(|v| v > 0),
                dhash: row.dhash,
                dominant_color: row.dominant_color,
                dominant_hex: row.dominant_hex,
                scene_tags: row
                    .scene_tags_json
                    .as_deref()
                    .map(parse_string_vec)
                    .unwrap_or_default(),
                time_of_day: row.time_of_day,
                season: row.season,
                album_titles: albums_by_item.remove(&item_key).unwrap_or_default(),
                people: people_by_item.remove(&item_key).unwrap_or_default(),
            });
        }

        out.sort_by(|a, b| {
            b.taken_at_ms
                .cmp(&a.taken_at_ms)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Photo People & Tagging
    // -----------------------------------------------------------------------

    async fn assemble_people<C: ConnectionTrait>(
        conn: &C,
        owner_id: UserId,
        filter_person: Option<PhotoPersonId>,
        allowed_library_ids: Option<&HashSet<LibraryId>>,
    ) -> Result<Vec<PhotoPersonSummary>, DbError> {
        let mut query = people::Entity::find().filter(PP::OwnerId.eq(owner_id.to_string()));
        if let Some(pid) = filter_person {
            query = query.filter(PP::Id.eq(pid.to_string()));
        }
        let rows = query
            .order_by_asc(PP::Name)
            .order_by_desc(PP::CreatedAt)
            .all(conn)
            .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let person_ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
        let allowed = allowed_lib_strings(allowed_library_ids);

        let mut tags_by_person: HashMap<String, Vec<String>> = HashMap::new();
        let can_query_items = !matches!(allowed.as_ref(), Some(libs) if libs.is_empty());
        if can_query_items {
            let mut tag_sel = person_tags::Entity::find()
                .join(JoinType::InnerJoin, person_tags::Relation::Item.def())
                .join(
                    JoinType::InnerJoin,
                    items::Entity::belongs_to(libraries::Entity)
                        .from(items::Column::LibraryId)
                        .to(libraries::Column::Id)
                        .into(),
                )
                .filter(PPT::PersonId.is_in(person_ids))
                .filter(items::Column::DeletedAt.is_null())
                .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
                .filter(libraries::Column::DeletedAt.is_null())
                .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()));
            if let Some(libs) = allowed {
                tag_sel = tag_sel.filter(items::Column::LibraryId.is_in(libs));
            }
            let pairs = tag_sel
                .select_only()
                .column(PPT::PersonId)
                .column(PPT::ItemId)
                .order_by_desc(PPT::CreatedAt)
                .order_by_desc(PPT::ItemId)
                .into_tuple::<(String, String)>()
                .all(conn)
                .await?;
            for (pid, item_id) in pairs {
                tags_by_person.entry(pid).or_default().push(item_id);
            }
        }

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let tagged_items = tags_by_person.remove(&row.id).unwrap_or_default();
            let photo_count = u64::try_from(tagged_items.len()).unwrap_or(u64::MAX);
            let mut preview_item_ids = Vec::with_capacity(4);
            for raw_id in tagged_items.iter().take(4) {
                preview_item_ids.push(parse_id(raw_id, "preview item id")?);
            }
            let stored_cover: Option<ItemId> =
                parse_opt_id(row.cover_item_id.as_deref(), "cover item id")?;
            let cover_item_id = match stored_cover {
                Some(cid) => {
                    if tagged_items.is_empty()
                        || tagged_items.iter().any(|id| id == &cid.to_string())
                    {
                        Some(cid)
                    } else {
                        preview_item_ids.first().copied().or(Some(cid))
                    }
                }
                None => preview_item_ids.first().copied(),
            };
            out.push(PhotoPersonSummary {
                id: parse_id(&row.id, "person id")?,
                name: row.name,
                cover_item_id,
                preview_item_ids,
                photo_count,
                created_at: ts(row.created_at)?,
            });
        }
        out.sort_by(|a, b| {
            b.photo_count
                .cmp(&a.photo_count)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(out)
    }

    pub async fn list_people(
        &self,
        owner_id: UserId,
        allowed_library_ids: Option<&HashSet<LibraryId>>,
    ) -> Result<Vec<PhotoPersonSummary>, DbError> {
        Self::assemble_people(self.db.main(), owner_id, None, allowed_library_ids).await
    }

    pub async fn get_person(
        &self,
        owner_id: UserId,
        person_id: PhotoPersonId,
        allowed_library_ids: Option<&HashSet<LibraryId>>,
    ) -> Result<Option<PhotoPersonSummary>, DbError> {
        let mut list = Self::assemble_people(
            self.db.main(),
            owner_id,
            Some(person_id),
            allowed_library_ids,
        )
        .await?;
        Ok(list.pop())
    }

    pub async fn create_person(
        &self,
        owner_id: UserId,
        name: &str,
        cover_item_id: Option<ItemId>,
    ) -> Result<PhotoPersonSummary, DbError> {
        let clean_name = validate_person_name(name)?;
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        if let Some(cover_id) = cover_item_id {
            verify_live_photo(&txn, cover_id).await?;
        }

        let id = PhotoPersonId::generate();
        let now = now_ms();
        people::Entity::insert(people::ActiveModel {
            id: Set(id.to_string()),
            owner_id: Set(owner_id.to_string()),
            name: Set(clean_name),
            cover_item_id: Set(cover_item_id.map(|c| c.to_string())),
            created_at: Set(now),
        })
        .exec_without_returning(&txn)
        .await?;

        let created = Self::assemble_people(&txn, owner_id, Some(id), None)
            .await?
            .pop()
            .ok_or(DbError::NotFound("photo person"))?;
        txn.commit().await?;
        Ok(created)
    }

    pub async fn update_person(
        &self,
        owner_id: UserId,
        person_id: PhotoPersonId,
        name: Option<&str>,
        cover_item_id: Option<ItemId>,
    ) -> Result<PhotoPersonSummary, DbError> {
        let clean_name = name.map(validate_person_name).transpose()?;
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let existing = people::Entity::find_by_id(person_id.to_string())
            .filter(PP::OwnerId.eq(owner_id.to_string()))
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("photo person"))?;

        if let Some(cover_id) = cover_item_id {
            verify_live_photo(&txn, cover_id).await?;
        }

        let mut active: people::ActiveModel = existing.into();
        if let Some(n) = clean_name {
            active.name = Set(n);
        }
        if let Some(cid) = cover_item_id {
            active.cover_item_id = Set(Some(cid.to_string()));
        }
        active.update(&txn).await?;

        let updated = Self::assemble_people(&txn, owner_id, Some(person_id), None)
            .await?
            .pop()
            .ok_or(DbError::NotFound("photo person"))?;
        txn.commit().await?;
        Ok(updated)
    }

    pub async fn delete_person(
        &self,
        owner_id: UserId,
        person_id: PhotoPersonId,
    ) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let res = people::Entity::delete_many()
            .filter(PP::Id.eq(person_id.to_string()))
            .filter(PP::OwnerId.eq(owner_id.to_string()))
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn tag_person_on_item(
        &self,
        owner_id: UserId,
        item_id: ItemId,
        person_id: PhotoPersonId,
        bbox_json: Option<String>,
    ) -> Result<PhotoPersonSummary, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let person = people::Entity::find_by_id(person_id.to_string())
            .filter(PP::OwnerId.eq(owner_id.to_string()))
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("photo person"))?;

        verify_live_photo(&txn, item_id).await?;

        let now = now_ms();
        person_tags::Entity::insert(person_tags::ActiveModel {
            person_id: Set(person_id.to_string()),
            item_id: Set(item_id.to_string()),
            bbox_json: Set(bbox_json),
            created_at: Set(now),
        })
        .on_conflict(
            OnConflict::columns([PPT::PersonId, PPT::ItemId])
                .update_columns([PPT::BboxJson])
                .to_owned(),
        )
        .exec_without_returning(&txn)
        .await?;

        if person.cover_item_id.is_none() {
            let mut active: people::ActiveModel = person.into();
            active.cover_item_id = Set(Some(item_id.to_string()));
            active.update(&txn).await?;
        }

        let updated = Self::assemble_people(&txn, owner_id, Some(person_id), None)
            .await?
            .pop()
            .ok_or(DbError::NotFound("photo person"))?;
        txn.commit().await?;
        Ok(updated)
    }

    pub async fn untag_person_from_item(
        &self,
        owner_id: UserId,
        item_id: ItemId,
        person_id: PhotoPersonId,
    ) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let Some(person) = people::Entity::find_by_id(person_id.to_string())
            .filter(PP::OwnerId.eq(owner_id.to_string()))
            .one(&txn)
            .await?
        else {
            txn.rollback().await?;
            return Ok(false);
        };

        let res = person_tags::Entity::delete_many()
            .filter(PPT::PersonId.eq(person_id.to_string()))
            .filter(PPT::ItemId.eq(item_id.to_string()))
            .exec(&txn)
            .await?;

        if res.rows_affected > 0 && person.cover_item_id.as_deref() == Some(&item_id.to_string()) {
            let next_cover = person_tags::Entity::find()
                .filter(PPT::PersonId.eq(person_id.to_string()))
                .select_only()
                .column(PPT::ItemId)
                .order_by_desc(PPT::CreatedAt)
                .into_tuple::<String>()
                .one(&txn)
                .await?;
            let mut active: people::ActiveModel = person.into();
            active.cover_item_id = Set(next_cover);
            active.update(&txn).await?;
        }

        txn.commit().await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn list_people_for_item(
        &self,
        owner_id: UserId,
        item_id: ItemId,
    ) -> Result<Vec<PhotoPersonRef>, DbError> {
        let rows = people::Entity::find()
            .filter(PP::OwnerId.eq(owner_id.to_string()))
            .join_rev(
                JoinType::InnerJoin,
                person_tags::Entity::belongs_to(people::Entity)
                    .from(PPT::PersonId)
                    .to(PP::Id)
                    .into(),
            )
            .filter(PPT::ItemId.eq(item_id.to_string()))
            .select_only()
            .column(PP::Id)
            .column(PP::Name)
            .order_by_asc(PP::Name)
            .into_tuple::<(String, String)>()
            .all(self.db.main())
            .await?;

        rows.into_iter()
            .map(|(id, name)| {
                Ok(PhotoPersonRef {
                    id: parse_id(&id, "person id")?,
                    name,
                })
            })
            .collect()
    }

    pub async fn photo_item_library(&self, item_id: ItemId) -> Result<Option<LibraryId>, DbError> {
        let row = items::Entity::find_by_id(item_id.to_string())
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
            .select_only()
            .column(items::Column::LibraryId)
            .into_tuple::<String>()
            .one(self.db.main())
            .await?;
        row.map(|lib| parse_id(&lib, "library id")).transpose()
    }
}

async fn verify_live_photo<C: ConnectionTrait>(conn: &C, item_id: ItemId) -> Result<(), DbError> {
    let exists = items::Entity::find_by_id(item_id.to_string())
        .filter(items::Column::DeletedAt.is_null())
        .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
        .select_only()
        .column(items::Column::Id)
        .into_tuple::<String>()
        .one(conn)
        .await?
        .is_some();
    if !exists {
        return Err(DbError::NotFound("photo"));
    }
    Ok(())
}

/// Helper to get current timestamp ms for callers building AI upserts.
pub fn current_time_ms() -> i64 {
    clock::to_millis(clock::now())
}
