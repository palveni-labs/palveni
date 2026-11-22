/**
 * HTMLAudioElement driver for the global music player:
 * - Playback ticket acquisition (`POST /items/{id}/playback`), renewal before expiry, and transcode
 *   fallback on decode error (matching `original-player.tsx`).
 * - Quality selection (`original` / `320` / `192` / `128`) with offset seeking for transcoded streams.
 * - Loudness normalization (`GET /items/{id}/loudness`).
 * - Next-track ticket + audio preloading.
 * - Play-count completion reporting (>50% or 4 min → `completed: true` with 409 conflict retry).
 * - OS Media Session metadata, position state, and hardware media key handlers.
 */
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { isApiError } from "@/api/errors";
import { musicApi, type MusicTrack, type PlaybackInfo, type TranscodeFormat } from "@/modules/music/api";
import { useMusicArtworkUrl } from "@/modules/music/artwork-cache";
import { musicKeys } from "@/modules/music/queries";
import { playerStore, shouldReportPlayCount, usePlayer, type AudioQuality } from "./store";

const AUDIO_FORMATS: ReadonlyArray<readonly [TranscodeFormat, string]> = [
  ["mp3", "audio/mpeg"],
  ["aac", 'audio/mp4; codecs="mp4a.40.2"'],
  ["opus", 'audio/ogg; codecs="opus"'],
];

interface LoadedSource {
  trackId: string;
  playInstanceId: number;
  quality: AudioQuality;
  ticket: PlaybackInfo;
  format: TranscodeFormat | undefined;
  bitrateKbps: number | undefined;
  startOffsetSec: number;
}

interface PreloadedTicket {
  trackId: string;
  quality: AudioQuality;
  format: TranscodeFormat | undefined;
  bitrateKbps: number | undefined;
  ticket: PlaybackInfo;
}

function supportedFormats(audio: HTMLAudioElement | null): TranscodeFormat[] {
  if (!audio || typeof audio.canPlayType !== "function") return ["mp3", "aac", "opus"];
  const playable = AUDIO_FORMATS.filter(([, mime]) => audio.canPlayType(mime) !== "").map(([format]) => format);
  return playable.length > 0 ? playable : ["mp3"];
}

async function fetchPlaybackTicket(
  trackId: string,
  options: { formats: Array<TranscodeFormat | undefined>; bitrateKbps?: number; startMs?: number; signal?: AbortSignal },
): Promise<{ ticket: PlaybackInfo; format: TranscodeFormat | undefined }> {
  for (let index = 0; index < options.formats.length; index++) {
    const format = options.formats[index];
    try {
      const ticket = await musicApi.playback(
        trackId,
        format ? { format, bitrateKbps: options.bitrateKbps, startMs: options.startMs } : { format: "original" },
        options.signal,
      );
      return { ticket, format };
    } catch (error) {
      if (options.signal?.aborted) throw error;
      if (isApiError(error) && error.code === "transcode_format_unavailable" && index < options.formats.length - 1) continue;
      throw error;
    }
  }
  throw new Error("No playable audio format");
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

export function useAudioEngine() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const player = usePlayer();
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const preloadAudioRef = useRef<HTMLAudioElement | null>(null);
  const sourceRef = useRef<LoadedSource | null>(null);
  const preloadedRef = useRef<PreloadedTicket | null>(null);
  const loadAbortRef = useRef<AbortController | null>(null);
  const renewalTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reportingRef = useRef<number | null>(null);

  const currentTrack = player.currentTrack;
  const artworkUrl = useMusicArtworkUrl(currentTrack?.artworkItemId ?? currentTrack?.id, 480, Boolean(currentTrack));

  // Create detached audio elements once on mount.
  useEffect(() => {
    if (typeof Audio === "undefined") return;
    const audio = new Audio();
    audio.preload = "auto";
    audioRef.current = audio;

    const preload = new Audio();
    preload.preload = "auto";
    preload.muted = true;
    preloadAudioRef.current = preload;

    return () => {
      loadAbortRef.current?.abort();
      if (renewalTimerRef.current) clearTimeout(renewalTimerRef.current);
      audio.pause();
      audio.removeAttribute("src");
      audio.load();
      preload.removeAttribute("src");
      preload.load();
      audioRef.current = null;
      preloadAudioRef.current = null;
    };
  }, []);

  // Report completed=true for play-count increment (with 409 conflict retry).
  const reportPlayCount = useRef(async (track: MusicTrack, playInstanceId: number) => {
    if (reportingRef.current === playInstanceId) return;
    reportingRef.current = playInstanceId;
    playerStore.markPlayCountReported();
    try {
      for (let attempt = 0; attempt < 2; attempt++) {
        const state = await musicApi.state(track.id);
        try {
          const updated = await musicApi.progress(track.id, {
            completed: true,
            positionMs: 0,
            expectedVersion: state.playbackVersion,
          });
          playerStore.patchTrackState(track.id, {
            playCount: updated.playCount,
            lastPlayedAt: updated.lastPlayedAt,
            favorite: updated.favorite,
            stars: updated.stars,
          });
          void queryClient.invalidateQueries({ queryKey: musicKeys.all });
          return;
        } catch (error) {
          if (isApiError(error) && error.status === 409 && attempt === 0) continue;
          return;
        }
      }
    } catch {
      // Best-effort reporting; never interrupt playback.
    }
  });

  // Load (or reload) audio stream for a given track, quality, and start offset.
  const loadStreamRef = useRef(async (
    track: MusicTrack,
    playInstanceId: number,
    quality: AudioQuality,
    startSec: number,
    autoPlay: boolean,
    fallbackFormats?: TranscodeFormat[],
  ) => {
    const audio = audioRef.current;
    if (!audio) return;

    loadAbortRef.current?.abort();
    if (renewalTimerRef.current) {
      clearTimeout(renewalTimerRef.current);
      renewalTimerRef.current = null;
    }

    const controller = new AbortController();
    loadAbortRef.current = controller;

    if (autoPlay) playerStore.reportStatus("loading", { error: null });

    const bitrateKbps = quality === "original" ? undefined : Number(quality);
    const formats: Array<TranscodeFormat | undefined> = fallbackFormats
      ? fallbackFormats
      : quality === "original"
        ? [undefined]
        : supportedFormats(audio);

    try {
      let ticket: PlaybackInfo;
      let format: TranscodeFormat | undefined;

      // Reuse preloaded ticket when starting from the beginning with matching track & quality.
      const preloaded = preloadedRef.current;
      if (
        !fallbackFormats &&
        startSec < 1 &&
        preloaded &&
        preloaded.trackId === track.id &&
        preloaded.quality === quality &&
        Date.now() < Date.parse(preloaded.ticket.expiresAt) - 30_000
      ) {
        ticket = preloaded.ticket;
        format = preloaded.format;
        preloadedRef.current = null;
      } else {
        const fetched = await fetchPlaybackTicket(track.id, {
          formats,
          bitrateKbps,
          startMs: formatStartMs(formats[0], startSec),
          signal: controller.signal,
        });
        ticket = fetched.ticket;
        format = fetched.format;
      }

      if (controller.signal.aborted) return;

      const startOffsetSec = ticket.mode === "transcode" ? Math.max(0, startSec) : 0;
      const loaded: LoadedSource = {
        trackId: track.id,
        playInstanceId,
        quality,
        ticket,
        format,
        bitrateKbps,
        startOffsetSec,
      };
      sourceRef.current = loaded;

      audio.volume = playerStore.effectiveVolume();
      audio.src = ticket.url;
      audio.load();

      if (ticket.mode === "direct" && startSec > 0) {
        const applyInitialSeek = () => {
          const end = Number.isFinite(audio.duration) ? Math.max(0, audio.duration - 0.5) : startSec;
          audio.currentTime = Math.min(startSec, end);
        };
        if (audio.readyState >= 1) applyInitialSeek();
        else audio.addEventListener("loadedmetadata", applyInitialSeek, { once: true });
      }

      // Schedule ticket renewal before expiry for long tracks or paused sessions.
      const msUntilRenew = Math.max(15_000, Date.parse(ticket.expiresAt) - Date.now() - 45_000);
      if (Number.isFinite(msUntilRenew)) {
        renewalTimerRef.current = setTimeout(() => {
          const snap = playerStore.getSnapshot();
          if (snap.currentTrack?.id === track.id && snap.playInstanceId === playInstanceId) {
            void loadStreamRef.current(track, playInstanceId, snap.quality, snap.currentTimeSec, snap.playing);
          }
        }, msUntilRenew);
      }

      playerStore.reportStatus(autoPlay ? "loading" : "paused", { mode: ticket.mode, error: null });

      if (autoPlay) {
        try {
          await audio.play();
        } catch (playErr) {
          if (controller.signal.aborted) return;
          // Autoplay restriction or transient abort: fall back to paused rather than crashing.
          if (playErr instanceof DOMException && (playErr.name === "NotAllowedError" || playErr.name === "AbortError")) {
            playerStore.reportStatus("paused", { mode: ticket.mode });
          } else {
            throw playErr;
          }
        }
      }
    } catch (error) {
      if (controller.signal.aborted || isAbortError(error)) return;
      playerStore.reportStatus("error", { error: t("musicApp.player.playbackError") });
    }
  });

  // Wire HTMLAudioElement DOM events once.
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;

    const onPlaying = () => {
      playerStore.reportStatus("playing", { mode: sourceRef.current?.ticket.mode ?? null, error: null });
    };
    const onPause = () => {
      const snap = playerStore.getSnapshot();
      if (!audio.ended && snap.status !== "loading" && snap.status !== "error") {
        playerStore.reportStatus("paused");
      }
    };
    const onWaiting = () => {
      if (playerStore.getSnapshot().playing) playerStore.reportStatus("loading");
    };
    const onLoadedMetadata = () => {
      const src = sourceRef.current;
      const trackDuration = (playerStore.getSnapshot().currentTrack?.durationMs ?? 0) / 1000;
      const effectiveDuration =
        src?.ticket.mode === "transcode"
          ? trackDuration || (Number.isFinite(audio.duration) ? src.startOffsetSec + audio.duration : 0)
          : Number.isFinite(audio.duration) && audio.duration > 0
            ? audio.duration
            : trackDuration;
      playerStore.reportTime((src?.startOffsetSec ?? 0) + audio.currentTime, effectiveDuration);
    };
    const onTimeUpdate = () => {
      const src = sourceRef.current;
      const snap = playerStore.getSnapshot();
      if (!src || !snap.currentTrack || src.trackId !== snap.currentTrack.id) return;
      const currentSec = src.startOffsetSec + audio.currentTime;
      const trackDuration = (snap.currentTrack.durationMs ?? 0) / 1000;
      const durationSec =
        src.ticket.mode === "transcode"
          ? trackDuration || snap.durationSec
          : Number.isFinite(audio.duration) && audio.duration > 0
            ? audio.duration
            : trackDuration;
      const bufferedSec =
        audio.buffered.length > 0 ? src.startOffsetSec + audio.buffered.end(audio.buffered.length - 1) : snap.bufferedSec;
      playerStore.reportTime(currentSec, durationSec, bufferedSec);

      if (!snap.playCountReported && snap.playing && shouldReportPlayCount(currentSec, durationSec)) {
        void reportPlayCount.current(snap.currentTrack, snap.playInstanceId);
      }
    };
    const onEnded = () => {
      const snap = playerStore.getSnapshot();
      if (snap.currentTrack && !snap.playCountReported) {
        void reportPlayCount.current(snap.currentTrack, snap.playInstanceId);
      }
      playerStore.next(false);
    };
    const onError = () => {
      const src = sourceRef.current;
      const snap = playerStore.getSnapshot();
      if (!src || !snap.currentTrack || src.trackId !== snap.currentTrack.id) return;
      // Expired ticket? Renew seamlessly at current position.
      if (Date.now() >= Date.parse(src.ticket.expiresAt) - 5_000) {
        void loadStreamRef.current(snap.currentTrack, snap.playInstanceId, snap.quality, snap.currentTimeSec, snap.playing);
        return;
      }
      // Decode or unsupported codec error on direct stream -> fallback to transcoding!
      const code = audio.error?.code;
      if (src.ticket.mode === "direct" && (code === 3 || code === 4)) {
        const playable = supportedFormats(audio);
        if (playable.length > 0) {
          void loadStreamRef.current(snap.currentTrack, snap.playInstanceId, snap.quality, snap.currentTimeSec, snap.playing, playable);
          return;
        }
      }
      playerStore.reportStatus("error", { error: t("musicApp.player.playbackError") });
    };

    audio.addEventListener("playing", onPlaying);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("waiting", onWaiting);
    audio.addEventListener("loadedmetadata", onLoadedMetadata);
    audio.addEventListener("timeupdate", onTimeUpdate);
    audio.addEventListener("ended", onEnded);
    audio.addEventListener("error", onError);
    return () => {
      audio.removeEventListener("playing", onPlaying);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("waiting", onWaiting);
      audio.removeEventListener("loadedmetadata", onLoadedMetadata);
      audio.removeEventListener("timeupdate", onTimeUpdate);
      audio.removeEventListener("ended", onEnded);
      audio.removeEventListener("error", onError);
    };
  }, [t]);

  // React to track / playInstanceId / quality changes.
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    if (!currentTrack) {
      loadAbortRef.current?.abort();
      if (renewalTimerRef.current) clearTimeout(renewalTimerRef.current);
      sourceRef.current = null;
      audio.pause();
      audio.removeAttribute("src");
      audio.load();
      return;
    }

    const src = sourceRef.current;
    const needsLoad =
      !src ||
      src.trackId !== currentTrack.id ||
      src.playInstanceId !== player.playInstanceId ||
      src.quality !== player.quality;

    if (needsLoad) {
      // Only fetch stream immediately if playing OR switching quality while already loaded;
      // for a hydrated paused queue on boot, defer loading until the user presses Play.
      const isQualitySwitch = Boolean(src && src.trackId === currentTrack.id && src.quality !== player.quality);
      if (player.playing || isQualitySwitch) {
        const startSec = playerStore.getSnapshot().currentTimeSec;
        void loadStreamRef.current(currentTrack, player.playInstanceId, player.quality, startSec, player.playing);
      }
      return;
    }

    // Play / pause sync on already-loaded source.
    if (player.playing && audio.paused) {
      void audio.play().catch(() => {
        playerStore.reportStatus("paused");
      });
    } else if (!player.playing && !audio.paused) {
      audio.pause();
    }
  }, [currentTrack, player.playInstanceId, player.quality, player.playing]);

  // Handle explicit seek requests.
  useEffect(() => {
    const req = player.seekRequest;
    const audio = audioRef.current;
    const src = sourceRef.current;
    if (!req || !audio || !currentTrack) return;
    playerStore.clearSeekRequest(req.seq);
    if (!src || src.trackId !== currentTrack.id) return;
    if (src.ticket.mode === "transcode") {
      void loadStreamRef.current(currentTrack, player.playInstanceId, player.quality, req.sec, player.playing);
    } else if (Number.isFinite(audio.duration) || audio.readyState >= 1) {
      audio.currentTime = req.sec;
    }
  }, [player.seekRequest, currentTrack, player.playInstanceId, player.quality, player.playing]);

  // Sync volume, mute, and loudness scaling.
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.volume = playerStore.effectiveVolume();
  }, [player.volume, player.muted, player.loudnessScale, player.loudnessEnabled]);

  // Fetch loudness measurement for the active track.
  useEffect(() => {
    if (!currentTrack) return;
    const controller = new AbortController();
    void musicApi
      .loudness(currentTrack.id, controller.signal)
      .then((res) => {
        if (!controller.signal.aborted && playerStore.getSnapshot().currentTrack?.id === currentTrack.id) {
          playerStore.setLoudnessMeasurement(res.measurement ?? null);
        }
      })
      .catch(() => {
        // Feature unavailable or unmeasured: keep unity gain.
      });
    return () => controller.abort();
  }, [currentTrack]);

  // Preload the next track's playback ticket & audio stream once the current track is playing.
  const nextTrack = playerStore.peekNextTrack();
  useEffect(() => {
    if (!player.playing || !nextTrack) return;
    const already = preloadedRef.current;
    if (already && already.trackId === nextTrack.id && already.quality === player.quality) return;

    const controller = new AbortController();
    const timer = setTimeout(() => {
      const bitrateKbps = player.quality === "original" ? undefined : Number(player.quality);
      const formats: Array<TranscodeFormat | undefined> =
        player.quality === "original" ? [undefined] : supportedFormats(audioRef.current);
      void fetchPlaybackTicket(nextTrack.id, { formats, bitrateKbps, signal: controller.signal })
        .then(({ ticket, format }) => {
          if (controller.signal.aborted) return;
          preloadedRef.current = { trackId: nextTrack.id, quality: player.quality, format, bitrateKbps, ticket };
          if (ticket.mode === "direct" && preloadAudioRef.current) {
            preloadAudioRef.current.src = ticket.url;
            preloadAudioRef.current.load();
          }
        })
        .catch(() => {
          // Preloading is best-effort.
        });
    }, 1500);

    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [player.playing, nextTrack, player.quality]);

  // Media Session API (OS media keys, lockscreen, notification artwork & scrubber).
  useEffect(() => {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator)) return;
    const session = navigator.mediaSession;
    if (!currentTrack) {
      session.metadata = null;
      session.playbackState = "none";
      return;
    }
    if (typeof MediaMetadata !== "undefined") {
      session.metadata = new MediaMetadata({
        title: currentTrack.title,
        artist: currentTrack.artist ?? currentTrack.albumArtist ?? "",
        album: currentTrack.album ?? "",
        artwork: artworkUrl ? [{ src: artworkUrl, sizes: "480x480", type: "image/png" }] : [],
      });
    }
    session.playbackState = player.playing ? "playing" : "paused";
  }, [currentTrack, artworkUrl, player.playing]);

  useEffect(() => {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator)) return;
    const session = navigator.mediaSession;
    if (!currentTrack || typeof session.setPositionState !== "function" || player.durationSec <= 0) return;
    try {
      session.setPositionState({
        duration: player.durationSec,
        playbackRate: 1,
        position: Math.min(player.currentTimeSec, player.durationSec),
      });
    } catch {
      // Ignore browser quirks when duration is temporarily unavailable.
    }
  }, [currentTrack, player.currentTimeSec, player.durationSec]);

  useEffect(() => {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator)) return;
    const session = navigator.mediaSession;
    const safeSet = (action: MediaSessionAction, handler: MediaSessionActionHandler | null) => {
      try {
        session.setActionHandler(action, handler);
      } catch {
        // Action unsupported in this browser.
      }
    };
    safeSet("play", () => playerStore.play());
    safeSet("pause", () => playerStore.pause());
    safeSet("previoustrack", () => playerStore.previous());
    safeSet("nexttrack", () => playerStore.next(true));
    safeSet("seekto", (details) => {
      if (details.seekTime != null) playerStore.seek(details.seekTime);
    });
    safeSet("seekbackward", (details) => playerStore.seekBy(-(details.seekOffset ?? 10)));
    safeSet("seekforward", (details) => playerStore.seekBy(details.seekOffset ?? 10));
    safeSet("stop", () => playerStore.pause());
    return () => {
      for (const action of ["play", "pause", "previoustrack", "nexttrack", "seekto", "seekbackward", "seekforward", "stop"] as const) {
        safeSet(action, null);
      }
    };
  }, []);
}

function formatStartMs(format: TranscodeFormat | undefined, startSec: number): number | undefined {
  if (!format || startSec <= 0) return undefined;
  return Math.floor(startSec * 1000);
}
