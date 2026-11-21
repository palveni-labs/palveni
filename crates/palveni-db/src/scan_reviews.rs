//! Durable, versioned snapshots of missing physical media reported by a scan.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use palveni_core::{ItemId, LibraryId};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, TransactionTrait,
};
use sha2::{Digest, Sha256};

use crate::convert::{now_ms, parse_id, sha256_hex};
use crate::entities::{items, libraries, scan_review_items as entries, scan_reviews as reviews};
use crate::items::{SORT_GROUP_LEAF, write_scan_batch};
use crate::{Db, DbError, revision};

// Four bind parameters per stored item, including on SQLite builds limited to 999.
const BATCH: usize = 200;
const SAMPLE: usize = 10;

pub struct ScanReviews<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingReview {
    pub scope: String,
    pub revision: String,
    pub missing_count: u64,
    pub sample_paths: Vec<String>,
}

fn fingerprint(source: &str, sorted: &[&(ItemId, String)]) -> String {
    let mut hash = Sha256::new();
    hash.update((source.len() as u64).to_be_bytes());
    hash.update(source.as_bytes());
    for (id, path) in sorted {
        hash.update(id.as_uuid().as_bytes());
        hash.update((path.len() as u64).to_be_bytes());
        hash.update(path.as_bytes());
    }
    hex::encode(hash.finalize())
}

fn missing_count(count: i64) -> Result<u64, DbError> {
    u64::try_from(count).map_err(|_| DbError::Corrupt("negative scan review item count".to_owned()))
}

fn sorted_paths(items: &[(ItemId, String)]) -> Vec<&(ItemId, String)> {
    let mut sorted: Vec<_> = items.iter().collect();
    sorted.sort_unstable_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    sorted
}

async fn find_generation<C: ConnectionTrait>(
    conn: &C,
    library: LibraryId,
    scope_hash: &str,
    digest: &str,
) -> Result<Option<String>, DbError> {
    Ok(reviews::Entity::find()
        .filter(reviews::Column::LibraryId.eq(library.to_string()))
        .filter(reviews::Column::ScopeHash.eq(scope_hash))
        .filter(reviews::Column::Digest.eq(digest))
        .select_only()
        .column(reviews::Column::Revision)
        .into_tuple()
        .one(conn)
        .await?)
}

// MySQL may compare text case-insensitively or ignore trailing spaces. Only a
// canonical token can identify a stored generation on any backend.
fn revision_key(value: &str) -> &str {
    if value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        value
    } else {
        ""
    }
}

fn by_revision(library: LibraryId, revision: &str) -> sea_orm::Select<reviews::Entity> {
    reviews::Entity::find_by_id(revision_key(revision).to_owned())
        .filter(reviews::Column::LibraryId.eq(library.to_string()))
}

impl ScanReviews<'_> {
    /// All scopes and at most ten paths per scope from the same statement snapshot.
    pub async fn list(&self, library: LibraryId) -> Result<Vec<MissingReview>, DbError> {
        let rows = entries::Entity::find()
            .inner_join(reviews::Entity)
            .filter(reviews::Column::LibraryId.eq(library.to_string()))
            .filter(entries::Column::Ordinal.lt(SAMPLE as i64))
            .select_only()
            .column(reviews::Column::Scope)
            .column(reviews::Column::Revision)
            .column(reviews::Column::MissingCount)
            .column(entries::Column::Path)
            .order_by_asc(entries::Column::Ordinal)
            .into_tuple::<(String, String, i64, String)>()
            .all(self.db.main())
            .await?;
        let mut scopes = BTreeMap::new();
        for (scope, revision, count, path) in rows {
            let missing_count = missing_count(count)?;
            scopes
                .entry(scope.clone())
                .or_insert_with(|| MissingReview {
                    scope,
                    revision,
                    missing_count,
                    sample_paths: Vec::new(),
                })
                .sample_paths
                .push(path);
        }
        Ok(scopes.into_values().collect())
    }

    pub async fn contains(&self, library: LibraryId, revision: &str) -> Result<bool, DbError> {
        Ok(by_revision(library, revision).count(self.db.main()).await? != 0)
    }

    pub async fn matches_source(
        &self,
        library: LibraryId,
        revision: &str,
        source: &str,
    ) -> Result<bool, DbError> {
        Ok(by_revision(library, revision)
            .filter(reviews::Column::SourceHash.eq(sha256_hex(source)))
            .count(self.db.main())
            .await?
            != 0)
    }

    pub async fn record(
        &self,
        library: LibraryId,
        scope: &str,
        missing: &[(ItemId, String)],
    ) -> Result<MissingReview, DbError> {
        self.record_on(library, scope, "", missing).await
    }

    pub async fn record_on(
        &self,
        library: LibraryId,
        scope: &str,
        source: &str,
        missing: &[(ItemId, String)],
    ) -> Result<MissingReview, DbError> {
        let sorted = sorted_paths(missing);
        let mut ids = HashSet::with_capacity(missing.len());
        if missing.is_empty()
            || sorted
                .iter()
                .any(|(id, path)| !ids.insert(*id) || !Path::new(path).starts_with(scope))
            || sorted.windows(2).any(|pair| pair[0].1 == pair[1].1)
        {
            return Err(DbError::Invalid(
                "expected distinct missing items and paths in the reviewed scope",
            ));
        }
        let digest = fingerprint(source, &sorted);
        let key = sha256_hex(scope);
        let summary = |revision| MissingReview {
            scope: scope.to_owned(),
            revision,
            missing_count: missing.len() as u64,
            sample_paths: sorted
                .iter()
                .take(SAMPLE)
                .map(|row| row.1.clone())
                .collect(),
        };
        if let Some(saved) = find_generation(self.db.main(), library, &key, &digest).await? {
            return Ok(summary(saved));
        }
        let txn = self.db.main().begin().await?;
        let generation = revision::next_revision(&txn).await?;
        if let Some(saved) = find_generation(&txn, library, &key, &digest).await? {
            txn.rollback().await?;
            return Ok(summary(saved));
        }
        if libraries::Entity::find_by_id(library.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .count(&txn)
            .await?
            == 0
        {
            return Err(DbError::NotFound("library"));
        }
        reviews::Entity::delete_many()
            .filter(reviews::Column::LibraryId.eq(library.to_string()))
            .filter(reviews::Column::ScopeHash.eq(&key))
            .exec(&txn)
            .await?;
        // Content alone would let an old confirmation match a later identical outage.
        let revision = sha256_hex(&format!("{generation}:{digest}"));
        reviews::Entity::insert(reviews::ActiveModel {
            revision: Set(revision.clone()),
            library_id: Set(library.to_string()),
            scope_hash: Set(key),
            scope: Set(scope.to_owned()),
            digest: Set(digest),
            source_hash: Set(sha256_hex(source)),
            missing_count: Set(missing.len() as i64),
        })
        .exec_without_returning(&txn)
        .await?;
        for (batch, rows) in sorted.chunks(BATCH).enumerate() {
            let entries = rows
                .iter()
                .enumerate()
                .map(|(n, row)| entries::ActiveModel {
                    revision: Set(revision.clone()),
                    ordinal: Set((batch * BATCH + n) as i64),
                    item_id: Set(row.0.to_string()),
                    path: Set(row.1.clone()),
                });
            entries::Entity::insert_many(entries)
                .exec_without_returning(&txn)
                .await?;
        }
        txn.commit().await?;
        Ok(summary(revision))
    }

    pub async fn clear(&self, library: LibraryId, revision: &str) -> Result<bool, DbError> {
        Ok(
            reviews::Entity::delete_by_id(revision_key(revision).to_owned())
                .filter(reviews::Column::LibraryId.eq(library.to_string()))
                .exec(self.db.main())
                .await?
                .rows_affected
                == 1,
        )
    }

    pub async fn paths(
        &self,
        library: LibraryId,
        revision: &str,
        offset: u64,
        limit: u32,
    ) -> Result<Option<(Vec<String>, u64)>, DbError> {
        let Some(count) = by_revision(library, revision)
            .select_only()
            .column(reviews::Column::MissingCount)
            .into_tuple::<i64>()
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let total = missing_count(count)?;
        let size = total
            .saturating_sub(offset)
            .min(u64::from(limit.clamp(1, 100)));
        if size == 0 {
            return Ok(Some((Vec::new(), total)));
        }
        let paths = entries::Entity::find()
            .filter(entries::Column::Revision.eq(revision))
            .filter(entries::Column::Ordinal.gte(offset as i64))
            .select_only()
            .column(entries::Column::Path)
            .order_by_asc(entries::Column::Ordinal)
            .limit(size)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?;
        // A changed or cleared revision cannot be recreated. A missing page means the
        // generation was consumed between the header and page reads on READ COMMITTED.
        Ok((paths.len() as u64 == size).then_some((paths, total)))
    }

    /// Commit all tombstones and consume their review together, including music metadata.
    /// The scanner re-enumerates storage and supplies any unused folders and CUE tracks.
    pub async fn apply_confirmed(
        &self,
        library: LibraryId,
        revision: &str,
        tombstones: &[ItemId],
    ) -> Result<bool, DbError> {
        self.apply_confirmed_on(library, revision, "", tombstones)
            .await
    }

    pub async fn apply_confirmed_on(
        &self,
        library: LibraryId,
        revision: &str,
        source: &str,
        tombstones: &[ItemId],
    ) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        let rev = revision::next_revision(&txn).await?;
        let Some(header) = by_revision(library, revision).one(&txn).await? else {
            txn.rollback().await?;
            return Ok(false);
        };
        let expected = missing_count(header.missing_count)?;
        if header.source_hash != sha256_hex(source) || (tombstones.len() as u64) < expected {
            txn.rollback().await?;
            return Ok(false);
        }
        let mut physical = Vec::new();
        for chunk in tombstones.chunks(BATCH) {
            let rows = items::Entity::find()
                .filter(items::Column::LibraryId.eq(library.to_string()))
                .filter(items::Column::Id.is_in(chunk.iter().map(ToString::to_string)))
                .filter(items::Column::DeletedAt.is_null())
                .select_only()
                .column(items::Column::Id)
                .column(items::Column::Path)
                .column(items::Column::SortGroup)
                .into_tuple::<(String, Option<String>, i32)>()
                .all(&txn)
                .await?;
            if rows.len() != chunk.len() {
                txn.rollback().await?;
                return Ok(false);
            }
            for (id, path, group) in rows {
                let Some(path) = path.filter(|p| Path::new(p).starts_with(&header.scope)) else {
                    txn.rollback().await?;
                    return Ok(false);
                };
                if group == SORT_GROUP_LEAF {
                    physical.push((parse_id(&id, "reviewed item id")?, path));
                }
            }
        }
        if physical.len() as u64 != expected
            || fingerprint(source, &sorted_paths(&physical)) != header.digest
        {
            txn.rollback().await?;
            return Ok(false);
        }
        let consumed = reviews::Entity::delete_by_id(header.revision)
            .filter(reviews::Column::LibraryId.eq(library.to_string()))
            .exec(&txn)
            .await?;
        if consumed.rows_affected != 1 {
            txn.rollback().await?;
            return Ok(false);
        }
        let now = now_ms();
        for chunk in tombstones.chunks(BATCH) {
            write_scan_batch(&txn, library, &[], chunk, now, rev).await?;
        }
        txn.commit().await?;
        Ok(true)
    }
}
