import { Button, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Disc3, Heart, Music2, Play, Shuffle, UsersRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { playerStore } from "@/player/store";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { musicApi } from "../api";
import { AlbumCard } from "../components/album-card";
import { ArtistCard } from "../components/artist-card";
import { TrackRow } from "../components/track-row";
import { MusicLibraryGuard, MusicLibrarySwitcher } from "../library-context";
import { musicKeys } from "../queries";

export type FavoritesTab = "songs" | "albums" | "artists";

function FavoriteSongs({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const query = useInfiniteQuery({
    queryKey: musicKeys.tracks(libraryId, { favorite: true, sort: "title", limit: 100 }),
    queryFn: ({ pageParam, signal }) => musicApi.tracks(libraryId, { favorite: true, sort: "title", cursor: pageParam, limit: 100 }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const tracks = query.data?.pages.flatMap((p) => p.tracks) ?? [];
  if (query.isPending) return <LoadingRows rows={6} />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />;
  if (tracks.length === 0) return <EmptyState icon={Heart} title={t("musicApp.favorites.emptySongs")} />;

  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="primary" onPress={() => playerStore.playQueue(tracks, 0, { shuffle: false })}>
          <Play className="fill-current" aria-hidden />
          {t("musicApp.actions.playAll")}
        </Button>
        <Button size="sm" variant="outline" onPress={() => playerStore.playShuffled(tracks)}>
          <Shuffle aria-hidden />
          {t("musicApp.actions.shuffle")}
        </Button>
      </div>
      <div className="divide-y divide-line/60">
        {tracks.map((track, index) => (
          <TrackRow key={track.id} track={track} indexLabel={index + 1} contextTracks={tracks} showCover showAlbum />
        ))}
      </div>
      {query.hasNextPage ? (
        <div className="flex justify-center pt-2">
          <Button size="sm" variant="outline" isDisabled={query.isFetchingNextPage} onPress={() => void query.fetchNextPage()}>
            {t("music.more")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

function FavoriteAlbums({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const query = useInfiniteQuery({
    queryKey: musicKeys.albums(libraryId, { favorite: true, sort: "title", limit: 60 }),
    queryFn: ({ pageParam, signal }) => musicApi.albums(libraryId, { favorite: true, sort: "title", cursor: pageParam, limit: 60 }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const albums = query.data?.pages.flatMap((p) => p.albums) ?? [];
  if (query.isPending) return <LoadingRows rows={4} />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />;
  if (albums.length === 0) return <EmptyState icon={Disc3} title={t("musicApp.favorites.emptyAlbums")} />;

  return (
    <div className="pv-grid-square">
      {albums.map((album) => (
        <AlbumCard key={album.id} album={album} />
      ))}
    </div>
  );
}

function FavoriteArtists({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const query = useInfiniteQuery({
    queryKey: musicKeys.artists(libraryId, { favorite: true, sort: "name", limit: 60 }),
    queryFn: ({ pageParam, signal }) => musicApi.artists(libraryId, { favorite: true, sort: "name", cursor: pageParam, limit: 60 }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const artists = query.data?.pages.flatMap((p) => p.artists) ?? [];
  if (query.isPending) return <LoadingRows rows={4} />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />;
  if (artists.length === 0) return <EmptyState icon={UsersRound} title={t("musicApp.favorites.emptyArtists")} />;

  return (
    <div className="pv-grid-square">
      {artists.map((artist) => (
        <ArtistCard key={artist.id} artist={artist} />
      ))}
    </div>
  );
}

export function MusicFavoritesPage() {
  const { t } = useTranslation();
  const { tab = "songs" } = useSearch({ from: "/_app/music/favorites" });
  const navigate = useNavigate({ from: "/music/favorites" });

  return (
    <PageFrame>
      <PageHeader title={t("musicApp.favorites.title")} />
      <MusicLibraryGuard>
        {(library) => (
          <div className="grid gap-5">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <ToggleButtonGroup
                aria-label={t("musicApp.favorites.title")}
                selectionMode="single"
                disallowEmptySelection
                selectedKeys={[tab]}
                onSelectionChange={(keys) => {
                  const next = [...keys][0];
                  if (next === "songs" || next === "albums" || next === "artists") {
                    void navigate({ search: { tab: next }, replace: true });
                  }
                }}
                className="pv-segmented"
              >
                <ToggleButton id="songs">
                  <Music2 className="size-3.5" aria-hidden />
                  {t("musicApp.favorites.songs")}
                </ToggleButton>
                <ToggleButton id="albums">
                  <Disc3 className="size-3.5" aria-hidden />
                  {t("musicApp.favorites.albums")}
                </ToggleButton>
                <ToggleButton id="artists">
                  <UsersRound className="size-3.5" aria-hidden />
                  {t("musicApp.favorites.artists")}
                </ToggleButton>
              </ToggleButtonGroup>
              <MusicLibrarySwitcher />
            </div>

            {tab === "songs" ? (
              <FavoriteSongs libraryId={library.id} />
            ) : tab === "albums" ? (
              <FavoriteAlbums libraryId={library.id} />
            ) : (
              <FavoriteArtists libraryId={library.id} />
            )}
          </div>
        )}
      </MusicLibraryGuard>
    </PageFrame>
  );
}
