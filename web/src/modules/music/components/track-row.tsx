import { Link } from "@tanstack/react-router";
import { Heart, Pause, Play, Volume2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { playerStore, usePlayer } from "@/player/store";
import type { MusicTrack } from "../api";
import { MusicArtwork } from "../artwork";
import { formatTrackTime } from "../format";
import { useRateTrack, useToggleTrackFavorite } from "../queries";
import { StarRating } from "./star-rating";
import { useContextPoint } from "./context-point";
import { TrackMenu } from "./track-menu";

/**
 * Reusable high-craft track row for album tracklists, artist top tracks, playlists, and search.
 */
export function TrackRow({
  track,
  indexLabel,
  contextTracks,
  showCover = false,
  showAlbum = false,
  showArtist = true,
  leadSlot,
  onRemove,
  removeLabel,
  className,
}: {
  track: MusicTrack;
  indexLabel?: number | string;
  contextTracks?: MusicTrack[];
  showCover?: boolean;
  showAlbum?: boolean;
  showArtist?: boolean;
  leadSlot?: ReactNode;
  onRemove?: () => void;
  removeLabel?: string;
  className?: string;
}) {
  const { t } = useTranslation();
  const player = usePlayer();
  const toggleFav = useToggleTrackFavorite();
  const rateTrack = useRateTrack();
  const { point, close, onContextMenu } = useContextPoint();

  const isCurrent = player.currentTrack?.id === track.id;
  const isPlaying = isCurrent && player.playing;

  const activate = () => {
    if (isCurrent) playerStore.togglePlay();
    else playerStore.playTrack(track, contextTracks);
  };

  return (
    <div
      onContextMenu={onContextMenu}
      onDoubleClick={activate}
      className={cn(
        "group flex min-h-13 items-center gap-3 rounded-tile px-2.5 py-1.5 transition-colors",
        isCurrent ? "bg-selected/10" : "hover:bg-hover",
        className,
      )}
    >
      {leadSlot}

      {/* Track index / live play toggle */}
      <button
        type="button"
        onClick={activate}
        aria-label={isPlaying ? t("musicApp.player.pause") : `${t("musicApp.actions.play")} ${track.title}`}
        className="grid size-7 shrink-0 place-items-center rounded-control text-xs text-fg-muted outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        {isPlaying ? (
          <>
            <Volume2 className="size-4 text-fg group-hover:hidden" aria-hidden />
            <Pause className="hidden size-3.5 fill-current text-fg group-hover:block" aria-hidden />
          </>
        ) : (
          <>
            <span data-num className={cn("tabular-nums group-hover:hidden", isCurrent && "font-strong text-fg")}>
              {indexLabel ?? track.trackNumber ?? "—"}
            </span>
            <Play className="hidden ms-0.5 size-3.5 fill-current text-fg group-hover:block" aria-hidden />
          </>
        )}
      </button>

      {showCover ? (
        <button
          type="button"
          onClick={activate}
          aria-hidden
          tabIndex={-1}
          className="shrink-0"
        >
          <MusicArtwork itemId={track.artworkItemId ?? track.id} size={96} className="size-10 rounded-[calc(var(--pv-radius-tile)*0.75)] ring-1 ring-line/50" />
        </button>
      ) : null}

      {/* Title & artist */}
      <div className="grid min-w-0 flex-1 gap-0.5">
        <button
          type="button"
          onClick={activate}
          className={cn("truncate text-start text-sm hover:underline", isCurrent ? "font-strong text-fg" : "font-mid text-fg")}
        >
          {track.title}
        </button>
        {showArtist ? (
          <div className="flex min-w-0 items-center gap-1.5 text-xs text-fg-muted">
            {track.artistId ? (
              <Link
                to="/music/artists/$libraryId/$artistId"
                params={{ libraryId: track.libraryId, artistId: track.artistId }}
                className="truncate hover:text-fg hover:underline"
              >
                {track.artist ?? t("musicApp.artists.unknown")}
              </Link>
            ) : (
              <span className="truncate">{track.artist ?? t("musicApp.artists.unknown")}</span>
            )}
          </div>
        ) : null}
      </div>

      {/* Album column (wide screens) */}
      {showAlbum ? (
        <div className="min-w-0 w-44 shrink-0 max-md:hidden">
          {track.album && track.albumId ? (
            <Link
              to="/music/albums/$libraryId/$albumId"
              params={{ libraryId: track.libraryId, albumId: track.albumId }}
              className="block truncate text-sm text-fg-muted hover:text-fg hover:underline"
            >
              {track.album}
            </Link>
          ) : (
            <span className="block truncate text-sm text-fg-muted">{track.album ?? "—"}</span>
          )}
        </div>
      ) : null}

      {/* Star rating (shown on hover or when rated) */}
      <div
        className={cn(
          "shrink-0 transition-opacity max-lg:hidden",
          track.state.stars > 0 ? "opacity-100" : "opacity-0 group-hover:opacity-100 focus-within:opacity-100",
        )}
      >
        <StarRating
          value={track.state.stars}
          size="xs"
          onChange={(stars) => rateTrack.mutate({ trackId: track.id, stars })}
        />
      </div>

      {/* Play count */}
      {track.state.playCount > 0 ? (
        <span data-num className="w-10 shrink-0 text-end text-xs text-fg-muted tabular-nums max-xl:hidden">
          {track.state.playCount}
        </span>
      ) : (
        <span className="w-10 shrink-0 max-xl:hidden" aria-hidden />
      )}

      {/* Favorite */}
      <button
        type="button"
        aria-label={track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
        aria-pressed={track.state.favorite}
        onClick={() => toggleFav.mutate({ trackId: track.id, favorite: !track.state.favorite })}
        className={cn(
          "grid size-8 shrink-0 place-items-center rounded-control text-fg-muted transition-opacity hover:bg-sunken hover:text-fg focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
          track.state.favorite ? "opacity-100" : "opacity-0 group-hover:opacity-100",
        )}
      >
        <Heart className={cn("size-4", track.state.favorite && "fill-failed text-failed")} aria-hidden />
      </button>

      {/* Duration */}
      <span data-num className="w-11 shrink-0 text-end text-xs text-fg-muted tabular-nums">
        {formatTrackTime(track.durationMs)}
      </span>

      {/* More menu + right-click context menu */}
      <TrackMenu
        track={track}
        contextTracks={contextTracks}
        onRemove={onRemove}
        removeLabel={removeLabel}
        contextPoint={point}
        onCloseContext={close}
      />
    </div>
  );
}
