use super::*;
use crate::convert::ts;
use crate::entities::track_loudness::{self as loudness, Column as L};
use palveni_core::{
    clock,
    music::{LoudnessMeasurement, LoudnessValues},
};
use sea_orm::FromQueryResult;

#[derive(Debug, FromQueryResult)]
pub struct LoudnessCandidate {
    pub id: String,
    pub revision: i64,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<i64>,
    pub measured_revision: Option<i64>,
}

impl Music<'_> {
    /// A keyset page of catalog recordings, excluding CUE backing files.
    pub async fn loudness_page(
        &self,
        library: LibraryId,
        after: &str,
    ) -> Result<Vec<LoudnessCandidate>, DbError> {
        use items::Column as I;
        Ok(items::Entity::find()
            .join_rev(
                JoinType::LeftJoin,
                sea_orm::RelationTrait::def(&tracks::Relation::Item),
            )
            .filter(visible_recordings())
            .join(
                JoinType::LeftJoin,
                items::Entity::belongs_to(loudness::Entity)
                    .from(I::Id)
                    .to(L::ItemId)
                    .into(),
            )
            .filter(I::LibraryId.eq(library.to_string()))
            .filter(I::Kind.eq("track"))
            .filter(I::DeletedAt.is_null())
            .filter(I::Path.is_not_null())
            .filter(I::Id.gt(after))
            .select_only()
            .columns([I::Id, I::Revision, I::SizeBytes, I::ModifiedAt])
            .column_as(L::Revision, "measured_revision")
            .order_by_asc(I::Id)
            .limit(64)
            .into_model::<LoudnessCandidate>()
            .all(self.db.main())
            .await?)
    }

    pub async fn loudness(&self, id: ItemId) -> Result<Option<LoudnessMeasurement>, DbError> {
        let revision = items::Entity::find_by_id(id.to_string())
            .filter(items::Column::Kind.eq("track"))
            .filter(items::Column::Path.is_not_null())
            .filter(items::Column::DeletedAt.is_null())
            .select_only()
            .column(items::Column::Revision)
            .into_tuple::<i64>()
            .one(self.db.main())
            .await?
            .ok_or(DbError::NotFound("track"))?;
        loudness::Entity::find_by_id(id.to_string())
            .one(self.db.main())
            .await?
            .map(|m| {
                Ok(LoudnessMeasurement {
                    integrated_lufs: m.integrated_lufs,
                    range_lu: m.range_lu,
                    true_peak_dbtp: m.true_peak_dbtp,
                    measured_at: ts(m.measured_at)?,
                    current: m.revision == revision,
                })
            })
            .transpose()
    }

    pub async fn save_loudness(
        &self,
        id: ItemId,
        revision: i64,
        values: LoudnessValues,
    ) -> Result<(), DbError> {
        loudness::Entity::insert(loudness::ActiveModel {
            item_id: Set(id.to_string()),
            revision: Set(revision),
            measured_at: Set(clock::now().timestamp_millis()),
            integrated_lufs: Set(values.integrated_lufs),
            range_lu: Set(values.range_lu),
            true_peak_dbtp: Set(values.true_peak_dbtp),
        })
        .on_conflict(
            OnConflict::column(L::ItemId)
                .update_columns([
                    L::Revision,
                    L::MeasuredAt,
                    L::IntegratedLufs,
                    L::RangeLu,
                    L::TruePeakDbtp,
                ])
                .to_owned(),
        )
        .exec_without_returning(self.db.main())
        .await?;
        Ok(())
    }
}
