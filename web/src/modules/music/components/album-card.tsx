import { Link } from "@tanstack/react-router";
import { Heart, Play } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { playerStore } from "@/player/store";
import { musicApi, type MusicAlbum } from "../api";
import { MusicArtwork } from "../artwork";
import { formatTrackTime } from "../format";
import { useToggleAlbumFavorite } from "../queries";
import { useContextPoint } from "./context-point";
import { AlbumMenu } from "./track-menu";

/** High-craft square album tile with hover play overlay, right-click menu and clean typography. */
export function AlbumCard({
  album,
  showArtist = true,
  className,
}: {
  album: MusicAlbum;
  showArtist?: boolean;
  className?: string;
}) {
  const { t } = useTranslation();
  const [loadingPlay, setLoadingPlay] = useState(false);
  const { point, close, onContextMenu } = useContextPoint();
  const albumTitle = album.name ?? t("musicApp.albums.unknown");

  const playAlbum = async () => {
    if (loadingPlay) return;
    setLoadingPlay(true);
    try {
      const { tracks } = await musicApi.album(album.libraryId, album.id);
      if (tracks.length > 0) playerStore.playQueue(tracks, 0, { shuffle: false });
    } finally {
      setLoadingPlay(false);
    }
  };

  return (
    <article
      onContextMenu={onContextMenu}
      className={cn("group flex min-w-0 flex-col gap-2.5", className)}
    >
      <div className="relative aspect-square w-full overflow-hidden rounded-2xl shadow-art ring-1 ring-white/12 ring-inset transition-all duration-200 ease-out group-hover:-translate-y-1.5 group-hover:shadow-overlay group-hover:ring-white/25">
        <Link
          to="/music/albums/$libraryId/$albumId"
          params={{ libraryId: album.libraryId, albumId: album.id }}
          aria-hidden="true"
          tabIndex={-1}
          className="block size-full outline-none"
        >
          <MusicArtwork
            itemId={album.artworkItemId}
            size={480}
            className="size-full rounded-none transition-transform duration-300 ease-skin group-hover:scale-[1.03]"
          />
        </Link>
        <div className="pointer-events-none absolute inset-x-0 bottom-0 flex items-end justify-between bg-gradient-to-t from-black/65 via-black/18 to-transparent p-3 opacity-0 transition-opacity duration-200 group-hover:opacity-100 group-focus-within:opacity-100">
          <button
            type="button"
            disabled={loadingPlay}
            onClick={(event) => {
              event.preventDefault();
              event.stopPropagation();
              void playAlbum();
            }}
            aria-label={`${t("musicApp.actions.play")} ${albumTitle}`}
            className="pv-nav-pill-active pointer-events-auto grid size-10 place-items-center rounded-full transition-transform hover:scale-105 active:scale-95 focus-visible:outline-2 focus-visible:outline-white"
          >
            <Play className="ms-0.5 size-4.5 fill-current" aria-hidden />
          </button>
          <div className="pointer-events-auto">
            <AlbumMenu
              album={album}
              contextPoint={point}
              onCloseContext={close}
              triggerClassName="size-8 rounded-full bg-black/55 text-white hover:bg-black/75 hover:text-white backdrop-blur-sm"
            />
          </div>
        </div>
      </div>

      <div className="grid min-w-0 gap-0.5 px-0.5">
        <div className="flex min-w-0 items-center gap-1">
          <Link
            to="/music/albums/$libraryId/$albumId"
            params={{ libraryId: album.libraryId, albumId: album.id }}
            title={albumTitle}
            className="min-w-0 flex-1 truncate text-sm font-strong text-fg hover:underline"
          >
            {albumTitle}
          </Link>
          {album.favorite ? <Heart className="size-3 shrink-0 fill-failed text-failed" aria-hidden /> : null}
        </div>
        <div className="flex min-w-0 items-center gap-1.5 text-xs text-fg-muted">
          {showArtist ? (
            album.artistId ? (
              <Link
                to="/music/artists/$libraryId/$artistId"
                params={{ libraryId: album.libraryId, artistId: album.artistId }}
                className="truncate hover:text-fg hover:underline"
              >
                {album.artist ?? t("musicApp.artists.unknown")}
              </Link>
            ) : (
              <span className="truncate">{album.artist ?? t("musicApp.artists.unknown")}</span>
            )
          ) : null}
          {showArtist && album.year ? <span aria-hidden>·</span> : null}
          {album.year ? <span data-num className="shrink-0 tabular-nums">{album.year}</span> : null}
        </div>
      </div>
    </article>
  );
}

/** Compact list row for Albums list mode. */
export function AlbumListRow({ album }: { album: MusicAlbum }) {
  const { t } = useTranslation();
  const toggleFav = useToggleAlbumFavorite();
  const { point, close, onContextMenu } = useContextPoint();
  const albumTitle = album.name ?? t("musicApp.albums.unknown");

  const playAlbum = async () => {
    const { tracks } = await musicApi.album(album.libraryId, album.id);
    if (tracks.length > 0) playerStore.playQueue(tracks, 0, { shuffle: false });
  };

  return (
    <div
      onContextMenu={onContextMenu}
      className="group grid h-full w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 transition-colors hover:bg-hover sm:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_4.5rem_5rem_auto]"
    >
      <div className="flex min-w-0 items-center gap-3">
        <button
          type="button"
          onClick={() => void playAlbum()}
          aria-label={`${t("musicApp.actions.play")} ${albumTitle}`}
          className="group/art relative shrink-0"
        >
          <MusicArtwork itemId={album.artworkItemId} size={96} className="size-10 rounded-[calc(var(--pv-radius-tile)*0.75)] ring-1 ring-line/50" />
          <span
            aria-hidden
            className="absolute inset-0 grid place-items-center rounded-[calc(var(--pv-radius-tile)*0.75)] bg-black/50 text-white opacity-0 transition-opacity group-hover/art:opacity-100"
          >
            <Play className="ms-0.5 size-4 fill-current" />
          </span>
        </button>
        <div className="grid min-w-0">
          <Link
            to="/music/albums/$libraryId/$albumId"
            params={{ libraryId: album.libraryId, albumId: album.id }}
            className="truncate text-sm font-strong text-fg hover:underline"
          >
            {albumTitle}
          </Link>
          <span className="truncate text-xs text-fg-muted sm:hidden">
            {album.artist ?? t("musicApp.artists.unknown")}{album.year ? ` · ${album.year}` : ""}
          </span>
        </div>
      </div>

      <Link
        to="/music/artists/$libraryId/$artistId"
        params={{ libraryId: album.libraryId, artistId: album.artistId }}
        className="truncate text-sm text-fg-muted hover:text-fg hover:underline max-sm:hidden"
      >
        {album.artist ?? t("musicApp.artists.unknown")}
      </Link>

      <span data-num className="text-end text-sm text-fg-muted tabular-nums max-sm:hidden">
        {album.year ?? "—"}
      </span>

      <span data-num className="text-end text-sm text-fg-muted tabular-nums max-sm:hidden">
        {formatTrackTime(album.durationMs)}
      </span>

      <div className="flex items-center justify-end gap-0.5">
        <button
          type="button"
          aria-label={album.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
          aria-pressed={album.favorite}
          onClick={() => toggleFav.mutate({ libraryId: album.libraryId, albumId: album.id, favorite: !album.favorite })}
          className={cn(
            "grid size-8 place-items-center rounded-control text-fg-muted transition-opacity hover:bg-sunken hover:text-fg",
            album.favorite ? "opacity-100" : "opacity-0 group-hover:opacity-100 focus-visible:opacity-100",
          )}
        >
          <Heart className={cn("size-4", album.favorite && "fill-failed text-failed")} aria-hidden />
        </button>
        <AlbumMenu album={album} contextPoint={point} onCloseContext={close} />
      </div>
    </div>
  );
}
