import { Button } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { Heart, Play, Shuffle } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { playerStore } from "@/player/store";
import { PageFrame } from "@/ui/frame";
import { ErrorState, LoadingRows } from "@/ui/states";
import { musicApi } from "../api";
import { MusicArtwork } from "../artwork";
import { AlbumCard } from "../components/album-card";
import { MusicHero } from "../components/music-hero";
import { TrackRow } from "../components/track-row";
import { musicKeys, useToggleArtistFavorite } from "../queries";

export function MusicArtistDetailPage() {
  const { libraryId, artistId } = useParams({ from: "/_app/music/artists/$libraryId/$artistId" });
  const { t } = useTranslation();
  const toggleFav = useToggleArtistFavorite();
  const [shuffling, setShuffling] = useState(false);

  const detailQuery = useQuery({
    queryKey: musicKeys.artist(libraryId, artistId),
    queryFn: ({ signal }) => musicApi.artist(libraryId, artistId, signal),
  });

  if (detailQuery.isPending) {
    return (
      <PageFrame>
        <LoadingRows rows={8} />
      </PageFrame>
    );
  }
  if (detailQuery.isError || !detailQuery.data) {
    return (
      <PageFrame>
        <ErrorState error={detailQuery.error} onRetry={() => void detailQuery.refetch()} />
      </PageFrame>
    );
  }

  const { artist, albums, topTracks } = detailQuery.data;
  const artistName = artist.name ?? t("musicApp.artists.unknown");

  const handleShuffleArtist = async () => {
    if (shuffling) return;
    setShuffling(true);
    try {
      const { tracks } = await musicApi.tracks(libraryId, { artistId, limit: 200 });
      const pool = tracks.length > 0 ? tracks : topTracks;
      if (pool.length > 0) playerStore.playShuffled(pool);
    } finally {
      setShuffling(false);
    }
  };

  return (
    <PageFrame>
      <MusicHero
        artworkItemId={artist.artworkItemId}
        artwork={
          <MusicArtwork
            itemId={artist.artworkItemId}
            size={480}
            round
            eager
            className="size-36 shadow-2xl ring-2 ring-white/25 sm:size-44"
          />
        }
        title={artistName}
        meta={
          <>
            <span data-num className="tabular-nums">{t("musicApp.artists.albumsCount", { count: artist.albumCount })}</span>
            <span aria-hidden>·</span>
            <span data-num className="tabular-nums">{t("musicApp.artists.tracksCount", { count: artist.trackCount })}</span>
            {artist.playCount > 0 ? (
              <>
                <span aria-hidden>·</span>
                <span data-num className="tabular-nums">{t("musicApp.albums.playsCount", { count: artist.playCount })}</span>
              </>
            ) : null}
          </>
        }
        actions={
          <>
            <Button
              size="sm"
              className="bg-white font-strong text-[#121316] hover:bg-white/90"
              isDisabled={topTracks.length === 0}
              onPress={() => playerStore.playQueue(topTracks, 0, { shuffle: false })}
            >
              <Play className="fill-current" aria-hidden />
              {t("musicApp.actions.playAll")}
            </Button>
            <Button
              size="sm"
              className="bg-white/16 text-white hover:bg-white/24"
              isDisabled={shuffling || artist.trackCount === 0}
              onPress={() => void handleShuffleArtist()}
            >
              <Shuffle aria-hidden />
              {t("musicApp.actions.shuffle")}
            </Button>
            <Button
              size="sm"
              isIconOnly
              aria-label={artist.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
              aria-pressed={artist.favorite}
              className="bg-white/14 text-white hover:bg-white/22"
              onPress={() => toggleFav.mutate({ libraryId: artist.libraryId, artistId: artist.id, favorite: !artist.favorite })}
            >
              <Heart className={cn("size-4", artist.favorite && "fill-rose-400 text-rose-400")} aria-hidden />
            </Button>
          </>
        }
      />

      {topTracks.length > 0 ? (
        <section aria-label={t("musicApp.artists.topTracks")} className="grid gap-2">
          <h2 className="font-heading text-lg font-title text-heading">{t("musicApp.artists.topTracks")}</h2>
          <div className="divide-y divide-line/60">
            {topTracks.map((track, index) => (
              <TrackRow
                key={track.id}
                track={track}
                indexLabel={index + 1}
                contextTracks={topTracks}
                showCover
                showAlbum
                showArtist={false}
              />
            ))}
          </div>
        </section>
      ) : null}

      {albums.length > 0 ? (
        <section aria-label={t("musicApp.artists.discography")} className="grid gap-3">
          <h2 className="font-heading text-lg font-title text-heading">{t("musicApp.artists.discography")}</h2>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {albums.map((album) => (
              <AlbumCard key={album.id} album={album} showArtist={false} />
            ))}
          </div>
        </section>
      ) : null}
    </PageFrame>
  );
}
