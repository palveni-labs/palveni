//! Year, genres and stream parameters for tracks, plus the album aggregates and
//! genre rows behind the music catalog. Existing tracks keep their rows; their
//! `tags_version` is NULL, so the next scan reads their files again.

use super::{bigint, bigint_null, id_ref, text_null, varchar, varchar_null};
use crate::entities::{
    items, libraries, music_album_pending as pending, music_albums as albums,
    track_genres as genres,
    track_metadata::{Column as C, Entity as Tracks},
};
use sea_orm::EntityName;
use sea_orm::sea_query::Expr;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const TRACK_INDEXES: [(&str, &[C]); 4] = [
    ("idx_track_title", &[C::LibraryId, C::TitleSort, C::ItemId]),
    (
        "idx_track_track_artist",
        &[C::LibraryId, C::TrackArtistSort, C::ItemId],
    ),
    (
        "idx_track_album_list",
        &[
            C::LibraryId,
            C::AlbumSort,
            C::AlbumKey,
            C::TrackSort,
            C::ItemId,
        ],
    ),
    ("idx_track_year", &[C::LibraryId, C::Year]),
];

fn track_columns() -> Vec<ColumnDef> {
    vec![
        bigint_null(C::Year),
        varchar_null(C::Codec, 16),
        bigint_null(C::BitrateKbps),
        bigint_null(C::SampleRateHz),
        bigint_null(C::BitDepth),
        bigint_null(C::Channels),
        varchar_null(C::TitleSort, 192),
        varchar_null(C::TrackArtistSort, 192),
        bigint_null(C::Shuffle),
        bigint_null(C::TagsVersion),
    ]
}

fn index<T: IntoIden + Copy>(
    name: &str,
    table: impl IntoTableRef,
    columns: &[T],
) -> IndexCreateStatement {
    let mut index = Index::create().name(name).table(table).to_owned();
    for column in columns {
        index.col(*column);
    }
    index
}

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in track_columns() {
            if manager
                .has_column(Tracks.table_name(), col.get_column_name())
                .await?
            {
                continue;
            }
            manager
                .alter_table(Table::alter().table(Tracks).add_column(col).to_owned())
                .await?;
        }
        for (name, columns) in TRACK_INDEXES {
            manager.create_index(index(name, Tracks, columns)).await?;
        }

        use genres::Column as G;
        manager
            .create_table(
                Table::create()
                    .table(genres::Entity)
                    .col(id_ref(G::ItemId))
                    .col(varchar(G::GenreKey, 64))
                    .col(varchar(G::Genre, 128))
                    .col(varchar(G::GenreSort, 192))
                    .col(id_ref(G::LibraryId))
                    .col(varchar(G::ArtistKey, 64))
                    .col(varchar(G::AlbumKey, 64))
                    .col(varchar(G::AlbumId, 129))
                    .primary_key(Index::create().col(G::ItemId).col(G::GenreKey))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_track_genre_item")
                            .from(genres::Entity, G::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        for (name, columns) in [
            (
                "idx_track_genre_album",
                &[G::LibraryId, G::GenreKey, G::ArtistKey, G::AlbumKey][..],
            ),
            (
                "idx_track_genre_lookup",
                &[G::LibraryId, G::ArtistKey, G::AlbumKey][..],
            ),
        ] {
            manager
                .create_index(index(name, genres::Entity, columns))
                .await?;
        }

        use albums::Column as A;
        manager
            .create_table(
                Table::create()
                    .table(albums::Entity)
                    .col(id_ref(A::LibraryId))
                    .col(varchar(A::ArtistKey, 64))
                    .col(varchar(A::AlbumKey, 64))
                    .col(text_null(A::Name))
                    .col(varchar(A::NameSort, 192))
                    .col(text_null(A::Artist))
                    .col(varchar(A::ArtistSort, 192))
                    .col(bigint_null(A::Year))
                    .col(bigint(A::TrackCount))
                    .col(bigint(A::DiscCount))
                    .col(bigint(A::DurationMs))
                    .col(id_ref(A::ArtworkItemId))
                    .col(bigint(A::AddedAt))
                    .col(bigint(A::Shuffle))
                    .primary_key(
                        Index::create()
                            .col(A::LibraryId)
                            .col(A::ArtistKey)
                            .col(A::AlbumKey),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_music_album_library")
                            .from(albums::Entity, A::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        for (name, columns) in [
            (
                "idx_music_album_name",
                &[A::LibraryId, A::NameSort, A::ArtistKey, A::AlbumKey][..],
            ),
            (
                "idx_music_album_artist",
                &[
                    A::LibraryId,
                    A::ArtistSort,
                    A::NameSort,
                    A::ArtistKey,
                    A::AlbumKey,
                ][..],
            ),
            (
                "idx_music_album_year",
                &[A::LibraryId, A::Year, A::NameSort][..],
            ),
            (
                "idx_music_album_added",
                &[A::LibraryId, A::AddedAt, A::ArtistKey, A::AlbumKey][..],
            ),
        ] {
            manager
                .create_index(index(name, albums::Entity, columns))
                .await?;
        }

        use pending::Column as P;
        manager
            .create_table(
                Table::create()
                    .table(pending::Entity)
                    .col(id_ref(P::LibraryId))
                    .col(varchar(P::ArtistKey, 64))
                    .col(varchar(P::AlbumKey, 64))
                    .col(bigint(P::Stamp))
                    .primary_key(
                        Index::create()
                            .col(P::LibraryId)
                            .col(P::ArtistKey)
                            .col(P::AlbumKey),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_music_album_pending_library")
                            .from(pending::Entity, P::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        // Existing music groups are computed on first use; new tags arrive with the next scan.
        let existing = Query::select()
            .columns([C::LibraryId, C::ArtistKey, C::AlbumKey])
            .expr(Expr::val(0i64))
            .from(Tracks)
            .and_where(Expr::col(C::ArtistKey).ne("spoken"))
            .group_by_columns([C::LibraryId, C::ArtistKey, C::AlbumKey])
            .to_owned();
        let insert = Query::insert()
            .into_table(pending::Entity)
            .columns([P::LibraryId, P::ArtistKey, P::AlbumKey, P::Stamp])
            .select_from(existing)
            .map_err(|err| DbErr::Custom(err.to_string()))?
            .to_owned();
        manager.exec_stmt(insert).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            pending::Entity.table_name(),
            albums::Entity.table_name(),
            genres::Entity.table_name(),
        ] {
            manager
                .drop_table(
                    Table::drop()
                        .table(Alias::new(table))
                        .if_exists()
                        .to_owned(),
                )
                .await?;
        }
        for (name, _) in TRACK_INDEXES {
            manager
                .drop_index(Index::drop().name(name).table(Tracks).to_owned())
                .await?;
        }
        for col in track_columns().into_iter().rev() {
            let name = col.get_column_name();
            if !manager.has_column(Tracks.table_name(), &name).await? {
                continue;
            }
            manager
                .alter_table(
                    Table::alter()
                        .table(Tracks)
                        .drop_column(Alias::new(name))
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
