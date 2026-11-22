import { Button } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import {
  ChevronDown,
  Heart,
  ListMusic,
  ListPlus,
  MessageSquareQuote,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { MusicArtwork } from "@/modules/music/artwork";
import { useMusicArtworkUrl } from "@/modules/music/artwork-cache";
import { PlaylistNameDialog, PlaylistPickerDialog } from "@/modules/music/components/playlist-picker-dialog";
import { StarRating } from "@/modules/music/components/star-rating";
import { formatAudioSpecs, formatSeconds } from "@/modules/music/format";
import { useCreatePlaylist, useRateTrack, useToggleTrackFavorite, useTrackLyrics } from "@/modules/music/queries";
import { activeLrcIndex, parseLrc } from "./lrc";
import { QueueList } from "./queue-drawer";
import { Scrubber } from "./scrubber";
import { playerStore, usePlayer, type AudioQuality } from "./store";

const QUALITIES: AudioQuality[] = ["original", "320", "192", "128"];

function LyricsPanel({ trackId, currentTimeSec }: { trackId: string; currentTimeSec: number }) {
  const { t } = useTranslation();
  const lyricsQuery = useTrackLyrics(trackId);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const activeRef = useRef<HTMLButtonElement | null>(null);
  const lyrics = lyricsQuery.data;

  const syncedLines = useMemo(
    () => (lyrics?.format === "lrc" ? parseLrc(lyrics.text) : []),
    [lyrics],
  );
  const activeIndex = useMemo(
    () => (syncedLines.length > 0 ? activeLrcIndex(syncedLines, currentTimeSec * 1000) : -1),
    [syncedLines, currentTimeSec],
  );

  useEffect(() => {
    const container = scrollRef.current;
    const node = activeRef.current;
    if (!container || !node || typeof container.scrollTo !== "function") return;
    const targetTop = Math.max(0, node.offsetTop - container.clientHeight / 2 + node.clientHeight / 2);
    const reduced = typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    container.scrollTo({ top: targetTop, behavior: reduced ? "auto" : "smooth" });
  }, [activeIndex, trackId]);

  if (lyricsQuery.isPending) {
    return (
      <div className="grid h-full place-items-center text-sm text-white/75">
        {t("musicApp.player.lyricsLoading")}
      </div>
    );
  }

  if (!lyrics || !lyrics.text.trim()) {
    return (
      <div className="grid h-full place-items-center text-center text-white/75">
        <div className="grid justify-items-center gap-2">
          <MessageSquareQuote className="size-8 opacity-60" aria-hidden />
          <p className="text-base">{t("musicApp.player.noLyrics")}</p>
        </div>
      </div>
    );
  }

  if (syncedLines.length > 0) {
    return (
      <div
        ref={scrollRef}
        className="no-scrollbar relative h-full overflow-y-auto px-3 py-8 lg:py-[12vh]"
        style={{
          maskImage: "linear-gradient(to bottom, transparent 0%, black 10%, black 90%, transparent 100%)",
          WebkitMaskImage: "linear-gradient(to bottom, transparent 0%, black 10%, black 90%, transparent 100%)",
        }}
      >
        <ul className="grid gap-5">
          {syncedLines.map((line, index) => {
            const active = index === activeIndex;
            const past = index < activeIndex;
            return (
              <li key={`${line.timeMs}-${index}`}>
                <button
                  ref={active ? activeRef : undefined}
                  type="button"
                  onClick={() => playerStore.seek(line.timeMs / 1000)}
                  aria-current={active ? "true" : undefined}
                  className={cn(
                    "w-full rounded-tile px-2 py-1 text-start leading-snug transition-all duration-200 focus-visible:outline-2 focus-visible:outline-white",
                    active
                      ? "text-2xl font-title text-white sm:text-[1.875rem]"
                      : past
                        ? "text-xl font-mid text-white/65 hover:text-white/95"
                        : "text-xl font-mid text-white/75 hover:text-white",
                  )}
                >
                  {line.text}
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    );
  }

  return (
    <div className="no-scrollbar h-full overflow-y-auto px-2 py-6">
      <div className="grid gap-3 text-lg leading-relaxed text-white/90">
        {lyrics.text.split(/\r?\n/).map((line, index) => (
          <p key={index} className={line.trim() ? "" : "h-3"}>
            {line}
          </p>
        ))}
      </div>
    </div>
  );
}

export function NowPlayingOverlay() {
  const { t } = useTranslation();
  const player = usePlayer();
  const toggleFav = useToggleTrackFavorite();
  const rateTrack = useRateTrack();
  const createPlaylist = useCreatePlaylist();
  const [pickerTrackIds, setPickerTrackIds] = useState<string[] | null>(null);
  const [saveQueueOpen, setSaveQueueOpen] = useState(false);

  const track = player.currentTrack;
  const backdropUrl = useMusicArtworkUrl(track?.artworkItemId ?? track?.id, 480, Boolean(player.nowPlayingOpen && track));

  useEffect(() => {
    if (!player.nowPlayingOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") playerStore.closeNowPlaying();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [player.nowPlayingOpen]);

  if (!player.nowPlayingOpen || !track) return null;

  const specs = formatAudioSpecs(track);
  const RepeatIcon = player.repeat === "one" ? Repeat1 : Repeat;
  const repeatLabel =
    player.repeat === "one"
      ? t("musicApp.player.repeatOne")
      : player.repeat === "all"
        ? t("musicApp.player.repeatAll")
        : t("musicApp.player.repeatOff");

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("musicApp.player.nowPlaying")}
      className="fixed inset-0 z-50 flex flex-col overflow-hidden bg-[#101114] text-white"
    >
      {/* Blurred album artwork backdrop with guaranteed dark contrast scrim */}
      <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
        {backdropUrl ? (
          <img
            src={backdropUrl}
            alt=""
            className="size-full scale-125 object-cover opacity-45 blur-3xl saturate-125"
          />
        ) : null}
        <div className="absolute inset-0 bg-gradient-to-b from-[#0d0e11]/75 via-[#0d0e11]/70 to-[#0a0b0e]/92" />
      </div>

      {/* Top bar */}
      <header className="relative z-10 flex items-center justify-between gap-3 px-4 py-3 sm:px-8 sm:py-4">
        <button
          type="button"
          onClick={() => playerStore.closeNowPlaying()}
          aria-label={t("musicApp.player.closeNowPlaying")}
          className="inline-flex h-9 items-center gap-1.5 rounded-full bg-white/10 px-3 text-sm font-mid text-white transition-colors hover:bg-white/20 focus-visible:outline-2 focus-visible:outline-white"
        >
          <ChevronDown className="size-4" aria-hidden />
          <span>{t("musicApp.player.nowPlaying")}</span>
        </button>

        <div className="flex flex-wrap items-center justify-end gap-2">
          {specs ? (
            <span className="rounded-full bg-white/12 px-2.5 py-1 text-xs font-mid text-white/90 max-sm:hidden">
              {specs}
            </span>
          ) : null}
          {player.loudnessEnabled && player.loudnessDb < 0 ? (
            <span className="rounded-full bg-white/12 px-2.5 py-1 text-xs font-mid text-white/90 tabular-nums max-md:hidden">
              {t("musicApp.player.loudnessActive", { db: player.loudnessDb })}
            </span>
          ) : null}
          <div role="tablist" aria-label={t("musicApp.player.nowPlaying")} className="inline-flex rounded-full bg-white/12 p-1">
            {(["lyrics", "queue"] as const).map((tab) => {
              const selected = player.nowPlayingTab === tab;
              return (
                <button
                  key={tab}
                  type="button"
                  role="tab"
                  aria-selected={selected}
                  onClick={() => playerStore.setNowPlayingTab(tab)}
                  className={cn(
                    "inline-flex h-7 items-center gap-1.5 rounded-full px-3 text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-white",
                    selected ? "bg-white text-[#121316] font-strong" : "text-white/85 hover:text-white",
                  )}
                >
                  {tab === "lyrics" ? <MessageSquareQuote className="size-3.5" aria-hidden /> : <ListMusic className="size-3.5" aria-hidden />}
                  {tab === "lyrics" ? t("musicApp.player.lyrics") : t("musicApp.player.queue")}
                </button>
              );
            })}
          </div>
        </div>
      </header>

      {/* Main 2-column stage on desktop, stacked on phone */}
      <div className="relative z-10 mx-auto grid min-h-0 w-full max-w-6xl flex-1 gap-6 overflow-y-auto px-5 pt-2 pb-6 sm:px-8 lg:grid-cols-[minmax(0,25rem)_minmax(0,1fr)] lg:items-center lg:gap-14 lg:overflow-hidden">
        {/* Left: Cover + Meta + Controls */}
        <div className="mx-auto flex w-full max-w-md flex-col gap-5">
          <MusicArtwork
            itemId={track.artworkItemId ?? track.id}
            size={960}
            eager
            className="mx-auto aspect-square w-full max-w-[13.5rem] rounded-2xl shadow-2xl ring-1 ring-white/15 sm:max-w-[18rem] lg:max-w-[23rem]"
          />

          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0 flex-1">
              <h1 className="truncate font-heading text-xl font-title text-white sm:text-2xl">{track.title}</h1>
              <div className="mt-1 flex flex-wrap items-center gap-x-2 text-sm text-white/80">
                {track.artistId ? (
                  <Link
                    to="/music/artists/$libraryId/$artistId"
                    params={{ libraryId: track.libraryId, artistId: track.artistId }}
                    onClick={() => playerStore.closeNowPlaying()}
                    className="truncate hover:text-white hover:underline"
                  >
                    {track.artist ?? t("musicApp.artists.unknown")}
                  </Link>
                ) : (
                  <span className="truncate">{track.artist ?? t("musicApp.artists.unknown")}</span>
                )}
                {track.album && track.albumId ? (
                  <>
                    <span aria-hidden>·</span>
                    <Link
                      to="/music/albums/$libraryId/$albumId"
                      params={{ libraryId: track.libraryId, albumId: track.albumId }}
                      onClick={() => playerStore.closeNowPlaying()}
                      className="truncate hover:text-white hover:underline"
                    >
                      {track.album}
                    </Link>
                  </>
                ) : null}
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-1">
              <button
                type="button"
                aria-label={track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
                aria-pressed={track.state.favorite}
                onClick={() => toggleFav.mutate({ trackId: track.id, favorite: !track.state.favorite })}
                className="grid size-9 place-items-center rounded-full text-white/85 transition-colors hover:bg-white/15 hover:text-white focus-visible:outline-2 focus-visible:outline-white"
              >
                <Heart className={cn("size-5", track.state.favorite && "fill-rose-400 text-rose-400")} aria-hidden />
              </button>
              <button
                type="button"
                aria-label={t("musicApp.actions.addToPlaylist")}
                onClick={() => setPickerTrackIds([track.id])}
                className="grid size-9 place-items-center rounded-full text-white/85 transition-colors hover:bg-white/15 hover:text-white focus-visible:outline-2 focus-visible:outline-white"
              >
                <ListPlus className="size-5" aria-hidden />
              </button>
            </div>
          </div>

          <div className="flex items-center justify-between gap-2">
            <StarRating
              value={track.state.stars}
              dark
              size="md"
              onChange={(stars) => rateTrack.mutate({ trackId: track.id, stars })}
            />
            <div className="inline-flex items-center gap-1 rounded-full bg-white/10 p-0.5">
              {QUALITIES.map((q) => (
                <button
                  key={q}
                  type="button"
                  onClick={() => playerStore.setQuality(q)}
                  className={cn(
                    "rounded-full px-2 py-0.5 text-[0.7rem] font-mid transition-colors",
                    player.quality === q ? "bg-white text-[#121316] font-strong" : "text-white/80 hover:text-white",
                  )}
                >
                  {q === "original" ? t("musicApp.player.qualityOriginal") : q}
                </button>
              ))}
            </div>
          </div>

          {/* Progress */}
          <div className="grid gap-1">
            <Scrubber
              dark
              value={player.currentTimeSec}
              max={player.durationSec || (track.durationMs ?? 0) / 1000}
              buffered={player.bufferedSec}
              label={t("musicApp.player.seek")}
              valueText={`${formatSeconds(player.currentTimeSec)} / ${formatSeconds(player.durationSec)}`}
              onCommit={(sec) => playerStore.seek(sec)}
            />
            <div className="flex justify-between text-xs text-white/75 tabular-nums">
              <span>{formatSeconds(player.currentTimeSec)}</span>
              <span>{formatSeconds(player.durationSec || (track.durationMs ?? 0) / 1000)}</span>
            </div>
          </div>

          {/* Transport */}
          <div className="flex items-center justify-between gap-2">
            <button
              type="button"
              aria-label={player.shuffle ? t("musicApp.player.shuffleOn") : t("musicApp.player.shuffleOff")}
              aria-pressed={player.shuffle}
              onClick={() => playerStore.toggleShuffle()}
              className={cn(
                "grid size-10 place-items-center rounded-full transition-colors focus-visible:outline-2 focus-visible:outline-white",
                player.shuffle ? "bg-white/20 text-white" : "text-white/70 hover:bg-white/10 hover:text-white",
              )}
            >
              <Shuffle className="size-4" aria-hidden />
            </button>
            <button
              type="button"
              aria-label={t("musicApp.player.previous")}
              onClick={() => playerStore.previous()}
              className="grid size-11 place-items-center rounded-full text-white transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
            >
              <SkipBack className="size-6 fill-current" aria-hidden />
            </button>
            <button
              type="button"
              aria-label={player.playing ? t("musicApp.player.pause") : t("musicApp.player.play")}
              onClick={() => playerStore.togglePlay()}
              className="grid size-14 place-items-center rounded-full bg-white text-[#121316] shadow-lg transition-transform hover:scale-105 active:scale-95 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
            >
              {player.playing ? <Pause className="size-6 fill-current" aria-hidden /> : <Play className="ms-0.5 size-6 fill-current" aria-hidden />}
            </button>
            <button
              type="button"
              aria-label={t("musicApp.player.next")}
              onClick={() => playerStore.next(true)}
              className="grid size-11 place-items-center rounded-full text-white transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
            >
              <SkipForward className="size-6 fill-current" aria-hidden />
            </button>
            <button
              type="button"
              aria-label={repeatLabel}
              aria-pressed={player.repeat !== "off"}
              onClick={() => playerStore.cycleRepeat()}
              className={cn(
                "grid size-10 place-items-center rounded-full transition-colors focus-visible:outline-2 focus-visible:outline-white",
                player.repeat !== "off" ? "bg-white/20 text-white" : "text-white/70 hover:bg-white/10 hover:text-white",
              )}
            >
              <RepeatIcon className="size-4" aria-hidden />
            </button>
          </div>

          {/* Volume */}
          <div className="flex items-center gap-3">
            <button
              type="button"
              aria-label={player.muted ? t("musicApp.player.unmute") : t("musicApp.player.mute")}
              onClick={() => playerStore.toggleMute()}
              className="grid size-8 shrink-0 place-items-center rounded-full text-white/80 hover:bg-white/10 hover:text-white"
            >
              {player.muted || player.volume === 0 ? <VolumeX className="size-4" aria-hidden /> : <Volume2 className="size-4" aria-hidden />}
            </button>
            <Scrubber
              dark
              compact
              value={player.muted ? 0 : player.volume}
              max={1}
              step={0.01}
              label={t("musicApp.player.volume")}
              valueText={`${Math.round((player.muted ? 0 : player.volume) * 100)}%`}
              onChange={(vol) => playerStore.setVolume(vol)}
              onCommit={(vol) => playerStore.setVolume(vol)}
            />
          </div>
        </div>

        {/* Right: Lyrics or Queue */}
        <div
          className={cn(
            "flex h-64 min-h-0 flex-col overflow-hidden lg:h-[calc(100svh-8rem)]",
            player.nowPlayingTab === "queue" && "rounded-2xl bg-black/25 p-4 ring-1 ring-white/10",
          )}
        >
          {player.nowPlayingTab === "lyrics" ? (
            <LyricsPanel trackId={track.id} currentTimeSec={player.currentTimeSec} />
          ) : (
            <div className="flex min-h-0 flex-1 flex-col gap-3">
              <div className="flex items-center justify-between px-2">
                <h2 className="text-sm font-strong text-white">
                  {t("musicApp.player.queueCount", { count: player.queue.length })}
                </h2>
                <div className="flex items-center gap-1.5">
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-white/85 hover:bg-white/15 hover:text-white"
                    onPress={() => setSaveQueueOpen(true)}
                  >
                    {t("musicApp.player.saveAsPlaylist")}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-white/85 hover:bg-white/15 hover:text-white"
                    onPress={() => playerStore.clearQueue()}
                  >
                    {t("musicApp.player.clearQueue")}
                  </Button>
                </div>
              </div>
              <div className="no-scrollbar min-h-0 flex-1 overflow-y-auto">
                <QueueList dark />
              </div>
            </div>
          )}
        </div>
      </div>

      <PlaylistPickerDialog trackIds={pickerTrackIds} onClose={() => setPickerTrackIds(null)} />
      <PlaylistNameDialog
        key={saveQueueOpen ? "open" : "closed"}
        isOpen={saveQueueOpen}
        onOpenChange={setSaveQueueOpen}
        title={t("musicApp.playlists.saveQueueTitle")}
        submitLabel={t("common.create")}
        isPending={createPlaylist.isPending}
        onSubmit={async (name) => {
          await createPlaylist.mutateAsync({ name, trackIds: player.queue.map((item) => item.id) });
        }}
      />
    </div>
  );
}
