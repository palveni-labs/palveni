import { Popover, Spinner, Switch } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import {
  AudioLines,
  Heart,
  ListMusic,
  Maximize2,
  MessageSquareQuote,
  Pause,
  Play,
  Repeat,
  Repeat1,
  RotateCw,
  Shuffle,
  SkipBack,
  SkipForward,
  SlidersHorizontal,
  Volume2,
  VolumeX,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { MusicArtwork } from "@/modules/music/artwork";
import { StarRating } from "@/modules/music/components/star-rating";
import { formatSeconds } from "@/modules/music/format";
import { useRateTrack, useToggleTrackFavorite } from "@/modules/music/queries";
import { useSlot } from "@/skins/scope";
import { Scrubber } from "./scrubber";
import { playerStore, usePlayer, type AudioQuality } from "./store";

const QUALITY_OPTIONS: Array<{ id: AudioQuality; labelKey: "qualityOriginal" | "quality320" | "quality192" | "quality128" }> = [
  { id: "original", labelKey: "qualityOriginal" },
  { id: "320", labelKey: "quality320" },
  { id: "192", labelKey: "quality192" },
  { id: "128", labelKey: "quality128" },
];

/**
 * Persistent global player bar:
 * - Desktop: 3-column translucent frosted dock at the bottom of the workbench.
 * - Phone: compact mini-bar floating above the bottom tab bar; tapping opens Now Playing.
 */
export function PlayerBar() {
  const { t } = useTranslation();
  const player = usePlayer();
  const navPhone = useSlot("navPhone");
  const toggleFav = useToggleTrackFavorite();
  const rateTrack = useRateTrack();

  const track = player.currentTrack;
  if (!track) return null;

  const duration = player.durationSec || (track.durationMs ?? 0) / 1000;
  const progressPct = duration > 0 ? Math.min(100, (player.currentTimeSec / duration) * 100) : 0;
  const RepeatIcon = player.repeat === "one" ? Repeat1 : Repeat;
  const repeatLabel =
    player.repeat === "one"
      ? t("musicApp.player.repeatOne")
      : player.repeat === "all"
        ? t("musicApp.player.repeatAll")
        : t("musicApp.player.repeatOff");
  const qualityBadge = player.quality === "original" ? (track.codec?.toUpperCase() ?? t("musicApp.player.qualityOriginal")) : `${player.quality}k`;

  return (
    <>
      {/* Desktop dock */}
      <section
        aria-label={t("musicApp.player.region")}
        className="relative z-30 hidden h-[4.75rem] shrink-0 grid-cols-[minmax(0,1fr)_minmax(18rem,1.45fr)_minmax(0,1fr)] items-center gap-4 border-t border-line bg-surface/88 px-4 backdrop-blur-xl md:grid"
      >
        {/* Left: Artwork + Title/Artist + Favorite + Rating */}
        <div className="group/meta flex min-w-0 items-center gap-3">
          <button
            type="button"
            onClick={() => playerStore.openNowPlaying("lyrics")}
            aria-label={t("musicApp.player.openNowPlaying")}
            className="group relative shrink-0 rounded-tile outline-none focus-visible:ring-2 focus-visible:ring-focus"
          >
            <MusicArtwork itemId={track.artworkItemId ?? track.id} size={96} eager className="size-12 shadow-art ring-1 ring-line/60" />
            <span
              aria-hidden
              className="absolute inset-0 grid place-items-center rounded-tile bg-black/45 text-white opacity-0 transition-opacity group-hover:opacity-100"
            >
              <Maximize2 className="size-4" />
            </span>
          </button>

          <div className="grid min-w-0 max-w-60 gap-0.5">
            <div className="flex min-w-0 items-center gap-1.5">
              {track.albumId ? (
                <Link
                  to="/music/albums/$libraryId/$albumId"
                  params={{ libraryId: track.libraryId, albumId: track.albumId }}
                  className="truncate text-sm font-strong text-fg hover:underline"
                >
                  {track.title}
                </Link>
              ) : (
                <span className="truncate text-sm font-strong text-fg">{track.title}</span>
              )}
              {player.error ? (
                <button
                  type="button"
                  onClick={() => playerStore.play()}
                  title={player.error}
                  className="inline-flex shrink-0 items-center gap-1 rounded-sm bg-failed-soft px-1.5 py-0.5 text-[0.7rem] font-mid text-failed"
                >
                  <RotateCw className="size-3" aria-hidden />
                  {t("musicApp.player.retry")}
                </button>
              ) : null}
            </div>
            <div className="flex min-w-0 items-center gap-2 text-xs text-fg-muted">
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
              <StarRating
                value={track.state.stars}
                size="xs"
                onChange={(stars) => rateTrack.mutate({ trackId: track.id, stars })}
                className={cn(
                  "shrink-0 transition-opacity max-xl:hidden",
                  track.state.stars > 0 ? "opacity-100" : "opacity-0 group-hover/meta:opacity-100 focus-within:opacity-100",
                )}
              />
            </div>
          </div>

          <button
            type="button"
            aria-label={track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
            aria-pressed={track.state.favorite}
            onClick={() => toggleFav.mutate({ trackId: track.id, favorite: !track.state.favorite })}
            className="grid size-8 shrink-0 place-items-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <Heart className={cn("size-4", track.state.favorite && "fill-failed text-failed")} aria-hidden />
          </button>
        </div>

        {/* Center: Transport + Scrubber */}
        <div className="flex min-w-0 flex-col items-center gap-1">
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              aria-label={player.shuffle ? t("musicApp.player.shuffleOn") : t("musicApp.player.shuffleOff")}
              aria-pressed={player.shuffle}
              onClick={() => playerStore.toggleShuffle()}
              className={cn(
                "grid size-8 place-items-center rounded-control transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                player.shuffle ? "bg-selected/15 text-fg" : "text-fg-muted hover:bg-hover hover:text-fg",
              )}
            >
              <Shuffle className="size-3.5" aria-hidden />
            </button>
            <button
              type="button"
              aria-label={t("musicApp.player.previous")}
              onClick={() => playerStore.previous()}
              className="grid size-8 place-items-center rounded-control text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <SkipBack className="size-4 fill-current" aria-hidden />
            </button>
            <button
              type="button"
              aria-label={player.playing ? t("musicApp.player.pause") : t("musicApp.player.play")}
              onClick={() => playerStore.togglePlay()}
              className="grid size-9 place-items-center rounded-full bg-action text-action-fg shadow-panel transition-transform hover:bg-action-hover active:scale-95 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--pv-focus)"
            >
              {player.status === "loading" ? (
                <Spinner size="sm" color="current" />
              ) : player.playing ? (
                <Pause className="size-4 fill-current" aria-hidden />
              ) : (
                <Play className="ms-0.5 size-4 fill-current" aria-hidden />
              )}
            </button>
            <button
              type="button"
              aria-label={t("musicApp.player.next")}
              onClick={() => playerStore.next(true)}
              className="grid size-8 place-items-center rounded-control text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <SkipForward className="size-4 fill-current" aria-hidden />
            </button>
            <button
              type="button"
              aria-label={repeatLabel}
              aria-pressed={player.repeat !== "off"}
              onClick={() => playerStore.cycleRepeat()}
              className={cn(
                "grid size-8 place-items-center rounded-control transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                player.repeat !== "off" ? "bg-selected/15 text-fg" : "text-fg-muted hover:bg-hover hover:text-fg",
              )}
            >
              <RepeatIcon className="size-3.5" aria-hidden />
            </button>
          </div>

          <div className="flex w-full max-w-xl items-center gap-2.5">
            <span data-num className="w-9 text-end text-[0.7rem] text-fg-muted tabular-nums">
              {formatSeconds(player.currentTimeSec)}
            </span>
            <Scrubber
              compact
              value={player.currentTimeSec}
              max={duration}
              buffered={player.bufferedSec}
              label={t("musicApp.player.seek")}
              valueText={`${formatSeconds(player.currentTimeSec)} / ${formatSeconds(duration)}`}
              onCommit={(sec) => playerStore.seek(sec)}
              className="min-w-0 flex-1"
            />
            <span data-num className="w-9 text-[0.7rem] text-fg-muted tabular-nums">
              {formatSeconds(duration)}
            </span>
          </div>
        </div>

        {/* Right: Quality + Loudness, Lyrics, Queue, Volume */}
        <div className="flex items-center justify-end gap-1">
          <Popover>
            <Popover.Trigger
              aria-label={t("musicApp.player.quality")}
              className="inline-flex h-7 items-center gap-1 rounded-chip border border-line bg-sunken/70 px-2 text-[0.7rem] font-mid text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              {player.loudnessEnabled && player.loudnessDb < 0 ? <AudioLines className="size-3 text-fg" aria-hidden /> : <SlidersHorizontal className="size-3" aria-hidden />}
              <span className="tabular-nums">{qualityBadge}</span>
            </Popover.Trigger>
            <Popover.Content placement="top end" className="w-64 p-3">
              <Popover.Dialog aria-label={t("musicApp.player.quality")} className="grid gap-3 outline-none">
                <div className="grid gap-1.5">
                  <p className="text-xs font-strong text-fg">{t("musicApp.player.quality")}</p>
                  <div className="grid grid-cols-2 gap-1">
                    {QUALITY_OPTIONS.map((option) => (
                      <button
                        key={option.id}
                        type="button"
                        aria-pressed={player.quality === option.id}
                        onClick={() => playerStore.setQuality(option.id)}
                        className={cn(
                          "rounded-control px-2.5 py-1.5 text-xs font-mid transition-colors",
                          player.quality === option.id ? "bg-action text-action-fg font-strong" : "bg-sunken text-fg hover:bg-hover",
                        )}
                      >
                        {t(`musicApp.player.${option.labelKey}`)}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="flex items-center justify-between gap-3 border-t border-line pt-2.5">
                  <div className="min-w-0">
                    <p className="text-xs font-strong text-fg">{t("musicApp.player.loudness")}</p>
                    <p className="text-[0.7rem] text-fg-muted">
                      {player.loudnessEnabled && player.loudnessDb < 0
                        ? t("musicApp.player.loudnessActive", { db: player.loudnessDb })
                        : t("musicApp.player.loudnessHint")}
                    </p>
                  </div>
                  <Switch
                    isSelected={player.loudnessEnabled}
                    onChange={(enabled) => playerStore.setLoudnessEnabled(enabled)}
                    aria-label={t("musicApp.player.loudness")}
                  />
                </div>
              </Popover.Dialog>
            </Popover.Content>
          </Popover>

          <button
            type="button"
            aria-label={t("musicApp.player.lyrics")}
            onClick={() => playerStore.toggleNowPlaying("lyrics")}
            className="grid size-8 place-items-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <MessageSquareQuote className="size-4" aria-hidden />
          </button>

          <button
            type="button"
            aria-label={t("musicApp.player.queue")}
            aria-expanded={player.queueOpen}
            onClick={() => playerStore.toggleQueue()}
            className={cn(
              "inline-flex h-8 items-center gap-1 rounded-control px-2 text-xs transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              player.queueOpen ? "bg-selected/15 font-strong text-fg" : "text-fg-muted hover:bg-hover hover:text-fg",
            )}
          >
            <ListMusic className="size-4" aria-hidden />
            <span data-num className="tabular-nums">{player.queue.length}</span>
          </button>

          <div className="flex items-center gap-1.5 ps-1 max-lg:hidden">
            <button
              type="button"
              aria-label={player.muted ? t("musicApp.player.unmute") : t("musicApp.player.mute")}
              onClick={() => playerStore.toggleMute()}
              className="grid size-8 place-items-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              {player.muted || player.volume === 0 ? <VolumeX className="size-4" aria-hidden /> : <Volume2 className="size-4" aria-hidden />}
            </button>
            <Scrubber
              compact
              value={player.muted ? 0 : player.volume}
              max={1}
              step={0.01}
              label={t("musicApp.player.volume")}
              valueText={`${Math.round((player.muted ? 0 : player.volume) * 100)}%`}
              onChange={(vol) => playerStore.setVolume(vol)}
              onCommit={(vol) => playerStore.setVolume(vol)}
              className="w-24"
            />
          </div>
        </div>
      </section>

      {/* Phone compact floating mini-bar */}
      <section
        aria-label={t("musicApp.player.region")}
        className={cn(
          "fixed inset-x-2 z-30 overflow-hidden rounded-panel border border-line bg-raised/92 shadow-raised backdrop-blur-xl md:hidden",
          navPhone === "bottom-bar" ? "bottom-[calc(4rem+env(safe-area-inset-bottom)+0.375rem)]" : "bottom-[calc(env(safe-area-inset-bottom)+0.5rem)]",
        )}
      >
        <div className="flex h-14 items-center gap-2.5 ps-2 pe-1.5">
          <button
            type="button"
            onClick={() => playerStore.openNowPlaying("lyrics")}
            aria-label={t("musicApp.player.openNowPlaying")}
            className="flex min-w-0 flex-1 items-center gap-2.5 text-start outline-none"
          >
            <MusicArtwork itemId={track.artworkItemId ?? track.id} size={96} eager className="size-10 rounded-[calc(var(--pv-radius-tile)*0.75)]" />
            <span className="grid min-w-0 flex-1">
              <span className="truncate text-sm font-strong text-fg">{track.title}</span>
              <span className="truncate text-xs text-fg-muted">{track.artist ?? t("musicApp.artists.unknown")}</span>
            </span>
          </button>
          <button
            type="button"
            aria-label={track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
            aria-pressed={track.state.favorite}
            onClick={() => toggleFav.mutate({ trackId: track.id, favorite: !track.state.favorite })}
            className="grid size-9 shrink-0 place-items-center rounded-control text-fg-muted"
          >
            <Heart className={cn("size-4", track.state.favorite && "fill-failed text-failed")} aria-hidden />
          </button>
          <button
            type="button"
            aria-label={player.playing ? t("musicApp.player.pause") : t("musicApp.player.play")}
            onClick={() => playerStore.togglePlay()}
            className="grid size-9 shrink-0 place-items-center rounded-full bg-action text-action-fg"
          >
            {player.playing ? <Pause className="size-4 fill-current" aria-hidden /> : <Play className="ms-0.5 size-4 fill-current" aria-hidden />}
          </button>
          <button
            type="button"
            aria-label={t("musicApp.player.next")}
            onClick={() => playerStore.next(true)}
            className="grid size-9 shrink-0 place-items-center rounded-control text-fg"
          >
            <SkipForward className="size-4 fill-current" aria-hidden />
          </button>
        </div>
        <div aria-hidden className="h-0.5 w-full bg-progress-rest">
          <div className="h-full bg-action" style={{ width: `${progressPct}%` }} />
        </div>
      </section>
    </>
  );
}
