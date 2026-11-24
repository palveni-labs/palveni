//! Manual and scraped metadata overrides (`metadata_overrides`, main store).
//!
//! Upserting an override also propagates matching display fields into
//! `video_metadata` / `video_genres`, `reading_metadata`, and `items` so the
//! library reflects edits immediately without waiting for a rescan.

use palveni_core::metadata::{MetadataOverride, MetadataProvider};
use palveni_core::{ItemId, clock};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait};

use crate::Db;
use crate::convert::{parse_id, truncate_chars, ts};
use crate::entities::{items, metadata_override, reading_metadata, video_genres, video_metadata};
use crate::error::DbError;
use crate::items::SORT_TITLE_MAX_CHARS;
use crate::revision::next_revision;

const TITLE_MAX_CHARS: usize = 255;
const WORK_KEY_MAX_CHARS: usize = 128;
const EXTERNAL_ID_MAX_CHARS: usize = 128;

pub struct MetadataOverrides<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct MetadataOverrideUpsert {
    pub item_id: ItemId,
    pub work_key: Option<String>,
    pub provider: MetadataProvider,
    pub external_id: Option<String>,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
}

fn rating_to_tenths(rating: f64) -> Option<i32> {
    if !rating.is_finite() {
        return None;
    }
    let clamped = rating.clamp(0.0, 10.0);
    let rounded = (clamped * 10.0).round();
    format!("{rounded:.0}")
        .parse::<i32>()
        .ok()
        .map(|v| v.clamp(0, 100))
}

fn clean_opt(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .map(|s| truncate_chars(&s, max_chars))
}

fn clean_genres(genres: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for raw in genres {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cleaned = truncate_chars(trimmed, 64);
        if !out
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&cleaned))
        {
            out.push(cleaned);
        }
        if out.len() >= 24 {
            break;
        }
    }
    out
}

fn to_domain(row: metadata_override::Model) -> Result<MetadataOverride, DbError> {
    let genres = row
        .genres_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default();
    let provider = MetadataProvider::from_db(&row.provider).unwrap_or(MetadataProvider::Manual);
    Ok(MetadataOverride {
        item_id: parse_id(&row.item_id, "metadata override item id")?,
        work_key: row.work_key,
        provider,
        external_id: row.external_id,
        title: row.title,
        original_title: row.original_title,
        year: row.year,
        overview: row.overview,
        rating: row.rating,
        genres,
        poster_url: row.poster_url,
        backdrop_url: row.backdrop_url,
        updated_at: ts(row.updated_at)?,
    })
}

impl MetadataOverrides<'_> {
    pub async fn get(&self, item_id: ItemId) -> Result<Option<MetadataOverride>, DbError> {
        metadata_override::Entity::find_by_id(item_id.to_string())
            .one(self.db.main())
            .await?
            .map(to_domain)
            .transpose()
    }

    pub async fn upsert(&self, input: MetadataOverrideUpsert) -> Result<MetadataOverride, DbError> {
        let now = clock::now();
        let now_ms = clock::to_millis(now);
        let item_id_str = input.item_id.to_string();

        let title = clean_opt(input.title, TITLE_MAX_CHARS);
        let original_title = clean_opt(input.original_title, TITLE_MAX_CHARS);
        let overview = input
            .overview
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let external_id = clean_opt(input.external_id, EXTERNAL_ID_MAX_CHARS);
        let poster_url = input
            .poster_url
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let backdrop_url = input
            .backdrop_url
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let genres = clean_genres(input.genres);
        let genres_json = if genres.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&genres)
                    .map_err(|e| DbError::Internal(format!("cannot encode genres: {e}")))?,
            )
        };

        let txn = self.db.main().begin().await?;
        let rev = next_revision(&txn).await?;

        let item = items::Entity::find_by_id(item_id_str.clone())
            .filter(items::Column::DeletedAt.is_null())
            .one(&txn)
            .await?;
        if item.is_none() {
            txn.rollback().await?;
            return Err(DbError::NotFound("item"));
        }

        let existing_video = video_metadata::Entity::find_by_id(item_id_str.clone())
            .one(&txn)
            .await?;

        let work_key = clean_opt(input.work_key, WORK_KEY_MAX_CHARS)
            .or_else(|| existing_video.as_ref().map(|v| v.work_key.clone()));

        let model = metadata_override::ActiveModel {
            item_id: ActiveValue::Set(item_id_str.clone()),
            work_key: ActiveValue::Set(work_key.clone()),
            provider: ActiveValue::Set(input.provider.as_str().to_owned()),
            external_id: ActiveValue::Set(external_id.clone()),
            title: ActiveValue::Set(title.clone()),
            original_title: ActiveValue::Set(original_title.clone()),
            year: ActiveValue::Set(input.year),
            overview: ActiveValue::Set(overview.clone()),
            rating: ActiveValue::Set(input.rating),
            genres_json: ActiveValue::Set(genres_json.clone()),
            poster_url: ActiveValue::Set(poster_url.clone()),
            backdrop_url: ActiveValue::Set(backdrop_url.clone()),
            updated_at: ActiveValue::Set(now_ms),
        };

        let on_conflict = OnConflict::column(metadata_override::Column::ItemId)
            .update_columns([
                metadata_override::Column::WorkKey,
                metadata_override::Column::Provider,
                metadata_override::Column::ExternalId,
                metadata_override::Column::Title,
                metadata_override::Column::OriginalTitle,
                metadata_override::Column::Year,
                metadata_override::Column::Overview,
                metadata_override::Column::Rating,
                metadata_override::Column::GenresJson,
                metadata_override::Column::PosterUrl,
                metadata_override::Column::BackdropUrl,
                metadata_override::Column::UpdatedAt,
            ])
            .to_owned();

        metadata_override::Entity::insert(model)
            .on_conflict(on_conflict)
            .exec_without_returning(&txn)
            .await?;

        // Propagate display title into `items` so browsing & search update immediately.
        if let Some(ref new_title) = title {
            let lower = new_title.to_lowercase();
            items::Entity::update_many()
                .col_expr(items::Column::Title, Expr::val(new_title.clone()))
                .col_expr(
                    items::Column::SortTitle,
                    Expr::val(truncate_chars(&lower, SORT_TITLE_MAX_CHARS)),
                )
                .col_expr(items::Column::SearchText, Expr::val(lower))
                .col_expr(items::Column::UpdatedAt, Expr::val(now_ms))
                .col_expr(items::Column::Revision, Expr::val(rev))
                .filter(items::Column::Id.eq(item_id_str.clone()))
                .exec(&txn)
                .await?;
        }

        // Propagate into `video_metadata` and `video_genres` when present.
        if let Some(video_row) = existing_video {
            let work_item_ids: Vec<String> = video_metadata::Entity::find()
                .filter(video_metadata::Column::LibraryId.eq(video_row.library_id.clone()))
                .filter(video_metadata::Column::WorkKey.eq(video_row.work_key.clone()))
                .select_only()
                .column(video_metadata::Column::ItemId)
                .into_tuple::<String>()
                .all(&txn)
                .await?;

            if !work_item_ids.is_empty() {
                let mut update_work = video_metadata::Entity::update_many()
                    .filter(video_metadata::Column::ItemId.is_in(work_item_ids.clone()));
                let mut touched_work = false;

                if let Some(ref new_title) = title {
                    update_work = update_work
                        .col_expr(
                            video_metadata::Column::WorkTitle,
                            Expr::val(new_title.clone()),
                        )
                        .col_expr(
                            video_metadata::Column::WorkSort,
                            Expr::val(crate::video::sort_key(new_title)),
                        )
                        .col_expr(
                            video_metadata::Column::WorkInitial,
                            Expr::val(crate::video::work_initial(new_title)),
                        );
                    touched_work = true;
                }
                if let Some(ref orig) = original_title {
                    update_work = update_work.col_expr(
                        video_metadata::Column::WorkOriginalTitle,
                        Expr::val(Some(orig.clone())),
                    );
                    touched_work = true;
                }
                if let Some(yr) = input.year {
                    update_work =
                        update_work.col_expr(video_metadata::Column::WorkYear, Expr::val(Some(yr)));
                    touched_work = true;
                }
                if let Some(ref ov) = overview {
                    update_work = update_work.col_expr(
                        video_metadata::Column::WorkOverview,
                        Expr::val(Some(ov.clone())),
                    );
                    touched_work = true;
                }
                if let Some(tenths) = input.rating.and_then(rating_to_tenths) {
                    update_work = update_work
                        .col_expr(video_metadata::Column::WorkRating, Expr::val(Some(tenths)));
                    touched_work = true;
                }

                if touched_work {
                    update_work.exec(&txn).await?;
                }

                // For single-file movies (`season IS NULL`), keep item-level title/overview/rating aligned.
                if video_row.season.is_none() {
                    let mut update_leaf = video_metadata::Entity::update_many()
                        .filter(video_metadata::Column::ItemId.eq(item_id_str.clone()));
                    let mut touched_leaf = false;
                    if let Some(ref new_title) = title {
                        update_leaf = update_leaf
                            .col_expr(video_metadata::Column::Title, Expr::val(new_title.clone()));
                        touched_leaf = true;
                    }
                    if let Some(ref ov) = overview {
                        update_leaf = update_leaf.col_expr(
                            video_metadata::Column::Overview,
                            Expr::val(Some(ov.clone())),
                        );
                        touched_leaf = true;
                    }
                    if let Some(tenths) = input.rating.and_then(rating_to_tenths) {
                        update_leaf = update_leaf
                            .col_expr(video_metadata::Column::Rating, Expr::val(Some(tenths)));
                        touched_leaf = true;
                    }
                    if touched_leaf {
                        update_leaf.exec(&txn).await?;
                    }
                }

                if !genres.is_empty() {
                    video_genres::Entity::delete_many()
                        .filter(video_genres::Column::ItemId.is_in(work_item_ids.clone()))
                        .exec(&txn)
                        .await?;
                    let mut genre_rows = Vec::with_capacity(work_item_ids.len() * genres.len());
                    for id in &work_item_ids {
                        for g in &genres {
                            genre_rows.push(video_genres::ActiveModel {
                                item_id: ActiveValue::Set(id.clone()),
                                name: ActiveValue::Set(g.clone()),
                            });
                        }
                    }
                    if !genre_rows.is_empty() {
                        video_genres::Entity::insert_many(genre_rows)
                            .exec_without_returning(&txn)
                            .await?;
                    }
                }
            }
        }

        // Propagate into `reading_metadata` when present.
        if reading_metadata::Entity::find_by_id(item_id_str.clone())
            .one(&txn)
            .await?
            .is_some()
        {
            let mut update_reading = reading_metadata::Entity::update_many()
                .filter(reading_metadata::Column::ItemId.eq(item_id_str.clone()));
            let mut touched_reading = false;

            if let Some(ref new_title) = title {
                let lower = new_title.to_lowercase();
                update_reading = update_reading
                    .col_expr(
                        reading_metadata::Column::Title,
                        Expr::val(new_title.clone()),
                    )
                    .col_expr(
                        reading_metadata::Column::SortTitle,
                        Expr::val(truncate_chars(&lower, SORT_TITLE_MAX_CHARS)),
                    );
                touched_reading = true;
            }
            if let Some(ref ov) = overview {
                update_reading = update_reading.col_expr(
                    reading_metadata::Column::Summary,
                    Expr::val(Some(ov.clone())),
                );
                touched_reading = true;
            }
            if let Some(ref tags_json) = genres_json {
                update_reading = update_reading.col_expr(
                    reading_metadata::Column::TagsJson,
                    Expr::val(tags_json.clone()),
                );
                touched_reading = true;
            }

            if touched_reading {
                update_reading.exec(&txn).await?;
            }
        }

        txn.commit().await?;

        Ok(MetadataOverride {
            item_id: input.item_id,
            work_key,
            provider: input.provider,
            external_id,
            title,
            original_title,
            year: input.year,
            overview,
            rating: input.rating,
            genres,
            poster_url,
            backdrop_url,
            updated_at: now,
        })
    }

    pub async fn delete(&self, item_id: ItemId) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        next_revision(&txn).await?;
        let res = metadata_override::Entity::delete_by_id(item_id.to_string())
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(res.rows_affected > 0)
    }
}
