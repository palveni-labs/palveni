import { Link } from "@tanstack/react-router";
import { Heart, Play } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { playerStore } from "@/player/store";
import { musicApi, type MusicArtist } from "../api";
import { MusicArtwork } from "../artwork";
import { useToggleArtistFavorite } from "../queries";

/** Circular portrait artist card for the Artist Wall. */
export function ArtistCard({ artist, className }: { artist: MusicArtist; className?: string }) {
  const { t } = useTranslation();
  const [loadingPlay, setLoadingPlay] = useState(false);
  const artistName = artist.name ?? t("musicApp.artists.unknown");

  const playArtist = async () => {
    if (loadingPlay) return;
    setLoadingPlay(true);
    try {
      const detail = await musicApi.artist(artist.libraryId, artist.id);
      if (detail.topTracks.length > 0) playerStore.playQueue(detail.topTracks, 0, { shuffle: false });
    } finally {
      setLoadingPlay(false);
    }
  };

  return (
    <article className={cn("group flex min-w-0 flex-col items-center gap-2.5 text-center", className)}>
      <div className="relative aspect-square w-full overflow-hidden rounded-full shadow-art ring-1 ring-line/50">
        <Link
          to="/music/artists/$libraryId/$artistId"
          params={{ libraryId: artist.libraryId, artistId: artist.id }}
          aria-hidden="true"
          tabIndex={-1}
          className="block size-full rounded-full outline-none"
        >
          <MusicArtwork
            itemId={artist.artworkItemId}
            size={480}
            round
            className="size-full transition-transform duration-300 ease-skin group-hover:scale-[1.04]"
          />
        </Link>
        <button
          type="button"
          disabled={loadingPlay}
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            void playArtist();
          }}
          aria-label={`${t("musicApp.actions.play")} ${artistName}`}
          className="absolute end-3 bottom-3 grid size-10 place-items-center rounded-full bg-white text-[#121316] opacity-0 shadow-overlay transition-all duration-200 group-hover:opacity-100 group-focus-within:opacity-100 hover:scale-105 active:scale-95 focus-visible:outline-2 focus-visible:outline-white"
        >
          <Play className="ms-0.5 size-4.5 fill-current" aria-hidden />
        </button>
      </div>

      <div className="grid w-full min-w-0 gap-0.5 px-1">
        <div className="flex min-w-0 items-center justify-center gap-1">
          <Link
            to="/music/artists/$libraryId/$artistId"
            params={{ libraryId: artist.libraryId, artistId: artist.id }}
            className="truncate text-sm font-strong text-fg hover:underline"
          >
            {artistName}
          </Link>
          {artist.favorite ? <Heart className="size-3 shrink-0 fill-failed text-failed" aria-hidden /> : null}
        </div>
        <span data-num className="truncate text-xs text-fg-muted tabular-nums">
          {t("musicApp.artists.albumsCount", { count: artist.albumCount })}
        </span>
      </div>
    </article>
  );
}

/** Compact list row for Artists list view. */
export function ArtistListRow({ artist }: { artist: MusicArtist }) {
  const { t } = useTranslation();
  const toggleFav = useToggleArtistFavorite();
  const artistName = artist.name ?? t("musicApp.artists.unknown");

  const playArtist = async () => {
    const detail = await musicApi.artist(artist.libraryId, artist.id);
    if (detail.topTracks.length > 0) playerStore.playQueue(detail.topTracks, 0, { shuffle: false });
  };

  return (
    <div className="group grid h-full w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 transition-colors hover:bg-hover sm:grid-cols-[minmax(0,1.5fr)_6.5rem_6.5rem_auto]">
      <div className="flex min-w-0 items-center gap-3">
        <button
          type="button"
          onClick={() => void playArtist()}
          aria-label={`${t("musicApp.actions.play")} ${artistName}`}
          className="group/art relative shrink-0"
        >
          <MusicArtwork itemId={artist.artworkItemId} size={96} round className="size-10 ring-1 ring-line/50" />
          <span
            aria-hidden
            className="absolute inset-0 grid place-items-center rounded-full bg-black/50 text-white opacity-0 transition-opacity group-hover/art:opacity-100"
          >
            <Play className="ms-0.5 size-4 fill-current" />
          </span>
        </button>
        <div className="grid min-w-0">
          <Link
            to="/music/artists/$libraryId/$artistId"
            params={{ libraryId: artist.libraryId, artistId: artist.id }}
            className="truncate text-sm font-strong text-fg hover:underline"
          >
            {artistName}
          </Link>
          <span data-num className="text-xs text-fg-muted tabular-nums sm:hidden">
            {t("musicApp.artists.albumsCount", { count: artist.albumCount })} · {t("musicApp.artists.tracksCount", { count: artist.trackCount })}
          </span>
        </div>
      </div>

      <span data-num className="text-end text-sm text-fg-muted tabular-nums max-sm:hidden">
        {t("musicApp.artists.albumsCount", { count: artist.albumCount })}
      </span>

      <span data-num className="text-end text-sm text-fg-muted tabular-nums max-sm:hidden">
        {t("musicApp.artists.tracksCount", { count: artist.trackCount })}
      </span>

      <button
        type="button"
        aria-label={artist.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
        aria-pressed={artist.favorite}
        onClick={() => toggleFav.mutate({ libraryId: artist.libraryId, artistId: artist.id, favorite: !artist.favorite })}
        className={cn(
          "grid size-8 place-items-center rounded-control text-fg-muted transition-opacity hover:bg-sunken hover:text-fg",
          artist.favorite ? "opacity-100" : "opacity-0 group-hover:opacity-100 focus-visible:opacity-100",
        )}
      >
        <Heart className={cn("size-4", artist.favorite && "fill-failed text-failed")} aria-hidden />
      </button>
    </div>
  );
}
