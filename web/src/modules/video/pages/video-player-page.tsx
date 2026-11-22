import { Spinner } from "@heroui/react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams, useRouter, useSearch } from "@tanstack/react-router";
import Hls from "hls.js";
import {
  AlertCircle,
  ArrowLeft,
  Check,
  Gauge,
  Languages,
  Maximize,
  Minimize,
  Pause,
  PictureInPicture2,
  Play,
  RotateCcw,
  RotateCw,
  Settings,
  SkipForward,
  Subtitles,
  Volume2,
  VolumeX,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import {
  imagePaths,
  videoApi,
  type PlaybackRequest,
  type PlaybackSession,
  type SubtitleTrack,
} from "../api";
import { VideoImage } from "../components/video-image";
import { codecLabel, formatClock, formatEpisodeCode } from "../format";
import { detectClientCapabilities } from "../player/capabilities";
import { findNextSeasonNumber, resolveNextEpisode } from "../player/next-episode";
import { createProgressReporter, type ProgressReporter } from "../player/progress-reporter";
import { offsetWebVtt, subtitleTrackLabel } from "../player/subtitles";
import { useMediaInfo, useVideoEpisodes, useVideoWork, videoKeys } from "../queries";

export interface VideoPlaySearch {
  libraryId?: string;
  workId?: string;
  season?: number;
  startMs?: number;
  fromStart?: boolean;
  title?: string;
}

type QualityChoice = "auto" | "original" | "1080" | "720" | "480";
type ActiveMenu = null | "subtitles" | "audio" | "quality" | "speed";

const SPEED_OPTIONS = [0.5, 0.75, 1, 1.25, 1.5, 2] as const;
const QUALITY_OPTIONS: Array<{ id: QualityChoice; labelKey?: string; label?: string }> = [
  { id: "auto", labelKey: "videoApp.player.qualityAuto" },
  { id: "original", labelKey: "videoApp.player.qualityOriginal" },
  { id: "1080", label: "1080p" },
  { id: "720", label: "720p" },
  { id: "480", label: "480p" },
];

function dispatchExclusiveMedia() {
  window.dispatchEvent(new CustomEvent("palveni:media-exclusive"));
}

export function VideoPlayerPage() {
  const { itemId } = useParams({ from: "/_app/video/play/$itemId" });
  const search = useSearch({ from: "/_app/video/play/$itemId" });
  return (
    <VideoPlayerSession
      key={`${itemId}:${search.fromStart ? 1 : 0}:${search.startMs ?? ""}`}
      itemId={itemId}
      search={search}
    />
  );
}

function VideoPlayerSession({ itemId, search }: { itemId: string; search: VideoPlaySearch }) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "zh-CN";
  const navigate = useNavigate();
  const router = useRouter();
  const queryClient = useQueryClient();

  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const hlsRef = useRef<Hls | null>(null);
  const activeSessionRef = useRef<PlaybackSession | null>(null);
  const reporterRef = useRef<ProgressReporter | null>(null);
  const hideTimerRef = useRef<number | null>(null);
  const lastMediaTimeRef = useRef(0);
  const pendingSeekSecRef = useRef<number | null>(search.startMs && !search.fromStart ? search.startMs / 1000 : null);

  const clientCaps = useMemo(() => detectClientCapabilities(), []);

  // Context metadata (title + next episode)
  const workQuery = useVideoWork(search.libraryId ?? "", search.workId ?? "", Boolean(search.libraryId && search.workId));
  const detail = workQuery.data;
  const effectiveSeason = search.season ?? detail?.nextUp?.seasonNumber ?? null;
  const seasonEpsQuery = useVideoEpisodes(
    search.libraryId ?? "",
    search.workId ?? "",
    effectiveSeason,
    Boolean(search.libraryId && search.workId && effectiveSeason !== null && detail?.work.kind === "series"),
  );
  const nextSeasonNum = useMemo(
    () => (detail && effectiveSeason !== null ? findNextSeasonNumber(detail.seasons, effectiveSeason) : null),
    [detail, effectiveSeason],
  );
  const nextSeasonEpsQuery = useVideoEpisodes(
    search.libraryId ?? "",
    search.workId ?? "",
    nextSeasonNum,
    Boolean(search.libraryId && search.workId && nextSeasonNum !== null && detail?.work.kind === "series"),
  );

  const currentEpisode = useMemo(
    () => (seasonEpsQuery.data ?? []).find((ep) => ep.itemId === itemId || ep.id === itemId) ?? null,
    [seasonEpsQuery.data, itemId],
  );
  const nextEpisodeTarget = useMemo(
    () => resolveNextEpisode(itemId, seasonEpsQuery.data ?? [], nextSeasonEpsQuery.data),
    [itemId, seasonEpsQuery.data, nextSeasonEpsQuery.data],
  );

  // Media info, subtitles, personal state
  const mediaInfoQuery = useMediaInfo(itemId);
  const subtitlesQuery = useQuery({
    queryKey: videoKeys.subtitles(itemId),
    queryFn: ({ signal }) => videoApi.subtitles(itemId, signal),
    staleTime: 60_000,
    retry: false,
  });
  const personalQuery = useQuery({
    queryKey: videoKeys.personal(itemId),
    queryFn: ({ signal }) => videoApi.personal(itemId, signal),
    staleTime: 0,
    retry: false,
  });

  const mediaInfo = mediaInfoQuery.data;
  const subtitleTracks = useMemo(() => subtitlesQuery.data?.tracks ?? [], [subtitlesQuery.data]);
  const audioTracks = useMemo(() => mediaInfo?.audio ?? [], [mediaInfo]);
  const chapters = useMemo(() => mediaInfo?.chapters ?? [], [mediaInfo]);

  // Playback session state
  const [session, setSession] = useState<PlaybackSession | null>(null);
  const [sessionLoading, setSessionLoading] = useState(true);
  const [sessionError, setSessionError] = useState<unknown>(null);
  const [attempt, setAttempt] = useState(0);

  // Selected stream preferences
  const [quality, setQuality] = useState<QualityChoice>("auto");
  const [audioIndex, setAudioIndex] = useState<number | undefined>(undefined);
  const [selectedSubId, setSelectedSubId] = useState<string>("");
  const lastSubIdRef = useRef<string>("");

  // Player UI state
  const [playing, setPlaying] = useState(false);
  const [buffering, setBuffering] = useState(false);
  const [currentTimeSec, setCurrentTimeSec] = useState(0);
  const [videoDurationSec, setVideoDurationSec] = useState(0);
  const [bufferedEndSec, setBufferedEndSec] = useState(0);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [speed, setSpeed] = useState<number>(1);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isPip, setIsPip] = useState(false);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [activeMenu, setActiveMenu] = useState<ActiveMenu>(null);
  const [hoverSeek, setHoverSeek] = useState<{ ratio: number; timeSec: number; chapter: string | null } | null>(null);

  // Resume prompt state (derived cleanly from personal state until user chooses)
  const [resumeDismissed, setResumeDismissed] = useState(Boolean(search.fromStart));
  const resumePromptMs = useMemo(() => {
    if (resumeDismissed || search.fromStart) return null;
    const personal = personalQuery.data;
    if (!personal || personal.completed) return null;
    const saved = search.startMs ?? personal.positionMs;
    return saved > 10_000 ? saved : null;
  }, [resumeDismissed, search.fromStart, search.startMs, personalQuery.data]);

  // Next episode countdown state
  const [countdownSec, setCountdownSec] = useState<number | null>(null);
  const [countdownDismissed, setCountdownDismissed] = useState(false);

  // Display title
  const displayTitle = useMemo(() => {
    if (currentEpisode) {
      const code = formatEpisodeCode(currentEpisode.seasonNumber, currentEpisode.episodeNumber);
      return [currentEpisode.seriesTitle ?? detail?.work.title, code, currentEpisode.title].filter(Boolean).join(" · ");
    }
    if (detail?.work.title) return detail.work.title;
    return search.title ?? t("videoApp.title");
  }, [currentEpisode, detail, search.title, t]);

  // Effective duration in seconds: direct files use real video duration when available; HLS uses mediaInfo duration
  const metaSec = mediaInfo?.durationMs ? mediaInfo.durationMs / 1000 : 0;
  const totalDurationSec =
    session?.protocol === "hls"
      ? metaSec > 0
        ? metaSec
        : videoDurationSec
      : videoDurationSec > 0
        ? videoDurationSec
        : metaSec;

  // Initialize progress reporter once personal state arrives (or fallback to v0)
  useEffect(() => {
    const initial = personalQuery.data;
    const reporter = createProgressReporter({
      initialVersion: initial?.playbackVersion ?? 0,
      initialPositionMs: initial?.positionMs ?? 0,
      intervalMs: 10_000,
      write: async (update) => {
        const saved = await videoApi.reportProgress(itemId, update);
        void queryClient.invalidateQueries({ queryKey: videoKeys.home() });
        return saved;
      },
    });
    reporterRef.current = reporter;
  }, [itemId, personalQuery.data, queryClient]);

  // Request playback session helper (for interactive quality/audio/HLS seek switches)
  const requestSession = useCallback(
    async (opts: {
      startMs?: number;
      qualityChoice?: QualityChoice;
      audioIdx?: number;
      signal?: AbortSignal;
    }) => {
      const qChoice = opts.qualityChoice ?? quality;
      const aIdx = opts.audioIdx !== undefined ? opts.audioIdx : audioIndex;
      const body: PlaybackRequest = {
        startMs: opts.startMs && opts.startMs > 0 ? Math.floor(opts.startMs) : undefined,
        audioIndex: aIdx,
        mode: qChoice === "original" ? "direct" : qChoice === "auto" ? "auto" : "transcode",
        maxHeight: qChoice === "1080" ? 1080 : qChoice === "720" ? 720 : qChoice === "480" ? 480 : undefined,
        client: clientCaps,
      };
      setSessionLoading(true);
      setSessionError(null);
      try {
        const previousId = activeSessionRef.current?.sessionId;
        const next = await videoApi.startPlayback(itemId, body, opts.signal);
        if (opts.signal?.aborted) return;
        activeSessionRef.current = next;
        setSession(next);
        if (previousId && previousId !== next.sessionId) {
          void videoApi.endSession(previousId).catch(() => {});
        }
      } catch (err) {
        if (!opts.signal?.aborted) setSessionError(err);
      } finally {
        if (!opts.signal?.aborted) setSessionLoading(false);
      }
    },
    [itemId, quality, audioIndex, clientCaps],
  );

  // Initial session startup (async promise avoids synchronous setState inside effect)
  useEffect(() => {
    const controller = new AbortController();
    const startMs = search.fromStart ? undefined : search.startMs;
    const body: PlaybackRequest = {
      startMs: startMs && startMs > 0 ? Math.floor(startMs) : undefined,
      mode: "auto",
      client: clientCaps,
    };
    void videoApi
      .startPlayback(itemId, body, controller.signal)
      .then((next) => {
        if (controller.signal.aborted) return;
        activeSessionRef.current = next;
        setSession(next);
        setSessionError(null);
        setSessionLoading(false);
      })
      .catch((err: unknown) => {
        if (controller.signal.aborted) return;
        setSessionError(err);
        setSessionLoading(false);
      });
    return () => {
      controller.abort();
    };
  }, [itemId, attempt, clientCaps, search.fromStart, search.startMs]);

  // Cleanup: flush final progress + DELETE video session on unmount or itemId change
  useEffect(() => {
    return () => {
      const lastPos = lastMediaTimeRef.current;
      if (lastPos > 0) {
        reporterRef.current?.flush(lastPos, false);
      }
      const sid = activeSessionRef.current?.sessionId;
      if (sid) {
        activeSessionRef.current = null;
        void videoApi.endSession(sid).catch(() => {});
      }
      hlsRef.current?.destroy();
      hlsRef.current = null;
    };
  }, [itemId]);

  // Attach media source (direct file or HLS via hls.js / native)
  useEffect(() => {
    const video = videoRef.current;
    if (!video || !session) return;

    hlsRef.current?.destroy();
    hlsRef.current = null;

    if (session.protocol === "hls") {
      if (clientCaps.hls === "native" && video.canPlayType("application/vnd.apple.mpegurl")) {
        video.src = session.url;
      } else if (Hls.isSupported()) {
        const hls = new Hls({ enableWorker: true });
        hlsRef.current = hls;
        hls.loadSource(session.url);
        hls.attachMedia(video);
        hls.on(Hls.Events.ERROR, (_event, data) => {
          if (!data.fatal) return;
          if (data.type === Hls.ErrorTypes.NETWORK_ERROR) hls.startLoad();
          else if (data.type === Hls.ErrorTypes.MEDIA_ERROR) hls.recoverMediaError();
          else {
            hls.destroy();
            setSessionError(new Error(data.details || "HLS playback error"));
          }
        });
      } else {
        video.src = session.url;
      }
    } else {
      video.src = session.url;
    }

    video.playbackRate = speed;

    // Auto-play unless waiting on the resume prompt
    if (!resumePromptMs) {
      dispatchExclusiveMedia();
      void video.play().catch(() => {
        // Autoplay may be blocked by browser policy; user can click play.
      });
    }

    return () => {
      hlsRef.current?.destroy();
      hlsRef.current = null;
    };
  }, [session, clientCaps.hls, resumePromptMs, speed]);

  // Active WebVTT subtitle track with HLS `startMs` time offset
  useEffect(() => {
    const video = videoRef.current;
    if (!video || !selectedSubId || !session) return;
    const track = subtitleTracks.find((tr) => tr.id === selectedSubId);
    if (!track) return;

    const controller = new AbortController();
    let blobUrl: string | null = null;
    let trackEl: HTMLTrackElement | null = null;

    const mount = async () => {
      try {
        const blob = await videoApi.subtitle(itemId, track.id, controller.signal);
        if (controller.signal.aborted) return;
        const offsetMs = session.protocol === "hls" ? session.startMs : 0;
        let finalBlob = blob;
        if (offsetMs > 0) {
          const text = await blob.text();
          if (controller.signal.aborted) return;
          finalBlob = new Blob([offsetWebVtt(text, offsetMs)], { type: "text/vtt; charset=utf-8" });
        }
        blobUrl = URL.createObjectURL(finalBlob);
        trackEl = document.createElement("track");
        trackEl.kind = "subtitles";
        trackEl.label = subtitleTrackLabel(track, locale, t);
        trackEl.srclang = track.language ?? "und";
        trackEl.default = true;
        trackEl.src = blobUrl;
        video.appendChild(trackEl);
        trackEl.track.mode = "showing";
      } catch {
        // Non-fatal: keep video playing if subtitle track fails
      }
    };
    void mount();

    return () => {
      controller.abort();
      if (trackEl) {
        trackEl.track.mode = "disabled";
        trackEl.remove();
      }
      if (blobUrl) URL.revokeObjectURL(blobUrl);
    };
  }, [itemId, selectedSubId, subtitleTracks, session, locale, t]);

  // Controls auto-hide management
  const controlsShown = controlsVisible || !playing || Boolean(activeMenu) || resumePromptMs !== null || countdownSec !== null;

  const revealControls = useCallback(() => {
    setControlsVisible(true);
    if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    if (playing && !activeMenu && !resumePromptMs && countdownSec === null) {
      hideTimerRef.current = window.setTimeout(() => {
        setControlsVisible(false);
      }, 3000);
    }
  }, [playing, activeMenu, resumePromptMs, countdownSec]);

  useEffect(() => {
    if (!playing || activeMenu || resumePromptMs || countdownSec !== null) {
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
      return;
    }
    hideTimerRef.current = window.setTimeout(() => setControlsVisible(false), 3000);
    return () => {
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    };
  }, [playing, activeMenu, resumePromptMs, countdownSec]);

  // Fullscreen & PiP listeners
  useEffect(() => {
    const onFs = () => setIsFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener("fullscreenchange", onFs);
    const video = videoRef.current;
    const onEnterPip = () => setIsPip(true);
    const onLeavePip = () => setIsPip(false);
    video?.addEventListener("enterpictureinpicture", onEnterPip);
    video?.addEventListener("leavepictureinpicture", onLeavePip);
    return () => {
      document.removeEventListener("fullscreenchange", onFs);
      video?.removeEventListener("enterpictureinpicture", onEnterPip);
      video?.removeEventListener("leavepictureinpicture", onLeavePip);
    };
  }, []);

  // Actions
  const goBack = useCallback(() => {
    if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {});
    }
    if (search.libraryId && search.workId) {
      void navigate({
        to: "/video/$libraryId/works/$workId",
        params: { libraryId: search.libraryId, workId: search.workId },
        search: effectiveSeason != null ? { season: effectiveSeason } : {},
      });
    } else if (window.history.length > 1) {
      router.history.back();
    } else {
      void navigate({ to: "/video" });
    }
  }, [navigate, router.history, search.libraryId, search.workId, effectiveSeason]);

  const togglePlay = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    if (resumePromptMs !== null) {
      setResumeDismissed(true);
    }
    if (video.paused || video.ended) {
      dispatchExclusiveMedia();
      void video.play().catch(() => {});
    } else {
      video.pause();
    }
  }, [resumePromptMs]);

  const seekTo = useCallback(
    (rawTargetSec: number) => {
      const video = videoRef.current;
      const targetSec = Math.max(0, totalDurationSec > 0 ? Math.min(totalDurationSec, rawTargetSec) : rawTargetSec);
      setCurrentTimeSec(targetSec);
      lastMediaTimeRef.current = targetSec;

      if (!video || !session) return;

      if (session.protocol === "hls") {
        const startSec = session.startMs / 1000;
        const seekableEnd =
          video.seekable.length > 0
            ? startSec + video.seekable.end(video.seekable.length - 1)
            : startSec + (Number.isFinite(video.duration) ? video.duration : 0);

        if (targetSec >= startSec && targetSec <= seekableEnd + 2) {
          video.currentTime = Math.max(0, targetSec - startSec);
          reporterRef.current?.flush(targetSec, false);
        } else {
          // Beyond transcoded range -> open a new HLS session at target startMs
          void requestSession({ startMs: Math.round(targetSec * 1000) });
          reporterRef.current?.flush(targetSec, false);
        }
      } else {
        const clamped = Number.isFinite(video.duration) && video.duration > 0 ? Math.min(targetSec, Math.max(0, video.duration - 0.1)) : targetSec;
        video.currentTime = clamped;
        reporterRef.current?.flush(clamped, false);
      }
    },
    [session, totalDurationSec, requestSession],
  );

  const skipBy = useCallback(
    (deltaSec: number) => {
      seekTo(lastMediaTimeRef.current + deltaSec);
      revealControls();
    },
    [seekTo, revealControls],
  );

  const updateVolume = useCallback((nextVol: number) => {
    const clamped = Math.max(0, Math.min(1, nextVol));
    setVolume(clamped);
    setMuted(clamped === 0);
    if (videoRef.current) {
      videoRef.current.volume = clamped;
      videoRef.current.muted = clamped === 0;
    }
  }, []);

  const toggleMute = useCallback(() => {
    const video = videoRef.current;
    const nextMuted = !muted;
    setMuted(nextMuted);
    if (video) video.muted = nextMuted;
    revealControls();
  }, [muted, revealControls]);

  const toggleFullscreen = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {});
    } else if (typeof el.requestFullscreen === "function") {
      void el.requestFullscreen().catch(() => {});
    }
  }, []);

  const togglePip = useCallback(() => {
    const video = videoRef.current;
    if (!video || !document.pictureInPictureEnabled) return;
    if (document.pictureInPictureElement) {
      void document.exitPictureInPicture().catch(() => {});
    } else if (typeof video.requestPictureInPicture === "function") {
      void video.requestPictureInPicture().catch(() => {});
    }
  }, []);

  const toggleSubtitlesShortcut = useCallback(() => {
    if (subtitleTracks.length === 0) return;
    if (selectedSubId) {
      lastSubIdRef.current = selectedSubId;
      setSelectedSubId("");
    } else {
      const nextId = lastSubIdRef.current || subtitleTracks[0]!.id;
      setSelectedSubId(nextId);
    }
    revealControls();
  }, [subtitleTracks, selectedSubId, revealControls]);

  const goNextEpisode = useCallback(() => {
    if (!nextEpisodeTarget?.episode.itemId) return;
    setCountdownSec(null);
    void navigate({
      to: "/video/play/$itemId",
      params: { itemId: nextEpisodeTarget.episode.itemId },
      search: {
        libraryId: search.libraryId ?? nextEpisodeTarget.episode.libraryId,
        workId: search.workId ?? nextEpisodeTarget.episode.seriesId ?? undefined,
        season: nextEpisodeTarget.seasonNumber,
        fromStart: true,
      },
      replace: true,
    });
  }, [navigate, nextEpisodeTarget, search.libraryId, search.workId]);

  const handleResumeContinue = () => {
    const targetMs = resumePromptMs ?? 0;
    setResumeDismissed(true);
    const targetSec = targetMs / 1000;
    if (session?.protocol === "hls" && targetMs > 0) {
      void requestSession({ startMs: targetMs });
    } else {
      const video = videoRef.current;
      if (video && Number.isFinite(video.duration) && video.duration > 0) {
        const clamped = Math.min(targetSec, Math.max(0, video.duration - 1));
        video.currentTime = clamped;
        setCurrentTimeSec(clamped);
        lastMediaTimeRef.current = clamped;
        dispatchExclusiveMedia();
        void video.play().catch(() => {});
      } else {
        pendingSeekSecRef.current = targetSec;
        dispatchExclusiveMedia();
        void videoRef.current?.play().catch(() => {});
      }
    }
  };

  const handleResumeRestart = () => {
    setResumeDismissed(true);
    pendingSeekSecRef.current = 0;
    const video = videoRef.current;
    if (video) {
      video.currentTime = 0;
      setCurrentTimeSec(0);
      lastMediaTimeRef.current = 0;
      dispatchExclusiveMedia();
      void video.play().catch(() => {});
    }
  };

  const handleQualityChange = (nextQ: QualityChoice) => {
    setQuality(nextQ);
    setActiveMenu(null);
    const posMs = Math.round(lastMediaTimeRef.current * 1000);
    pendingSeekSecRef.current = lastMediaTimeRef.current;
    void requestSession({ qualityChoice: nextQ, startMs: posMs });
  };

  const handleAudioChange = (nextIdx: number) => {
    setAudioIndex(nextIdx);
    setActiveMenu(null);
    const posMs = Math.round(lastMediaTimeRef.current * 1000);
    pendingSeekSecRef.current = lastMediaTimeRef.current;
    void requestSession({ audioIdx: nextIdx, startMs: posMs });
  };

  const handleSpeedChange = (nextSpeed: number) => {
    setSpeed(nextSpeed);
    if (videoRef.current) videoRef.current.playbackRate = nextSpeed;
    setActiveMenu(null);
  };

  // Next-episode countdown tick
  useEffect(() => {
    if (countdownSec === null) return;
    const timer = window.setTimeout(() => {
      if (countdownSec <= 1) {
        goNextEpisode();
      } else {
        setCountdownSec(countdownSec - 1);
      }
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [countdownSec, goNextEpisode]);

  // Keyboard shortcuts
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT")) {
        return;
      }
      if (event.metaKey || event.ctrlKey || event.altKey) return;

      switch (event.key) {
        case " ":
        case "k":
        case "K":
          event.preventDefault();
          togglePlay();
          break;
        case "ArrowLeft":
          event.preventDefault();
          skipBy(-10);
          break;
        case "ArrowRight":
          event.preventDefault();
          skipBy(10);
          break;
        case "ArrowUp":
          event.preventDefault();
          updateVolume(volume + 0.05);
          revealControls();
          break;
        case "ArrowDown":
          event.preventDefault();
          updateVolume(volume - 0.05);
          revealControls();
          break;
        case "f":
        case "F":
          event.preventDefault();
          toggleFullscreen();
          break;
        case "m":
        case "M":
          event.preventDefault();
          toggleMute();
          break;
        case "c":
        case "C":
          event.preventDefault();
          toggleSubtitlesShortcut();
          break;
        case "n":
        case "N":
          if (nextEpisodeTarget) {
            event.preventDefault();
            goNextEpisode();
          }
          break;
        case "Escape":
          if (activeMenu) {
            event.preventDefault();
            setActiveMenu(null);
          } else if (!document.fullscreenElement) {
            event.preventDefault();
            goBack();
          }
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    activeMenu,
    goBack,
    goNextEpisode,
    nextEpisodeTarget,
    revealControls,
    skipBy,
    toggleFullscreen,
    toggleMute,
    togglePlay,
    toggleSubtitlesShortcut,
    updateVolume,
    volume,
  ]);

  // Seekbar pointer math
  const chapterForSec = useCallback(
    (sec: number): string | null => {
      if (chapters.length === 0) return null;
      const ms = sec * 1000;
      let match: string | null = null;
      for (const ch of chapters) {
        if (ch.startMs <= ms) match = ch.title;
        else break;
      }
      return match;
    },
    [chapters],
  );

  const handleSeekPointer = (event: React.PointerEvent<HTMLDivElement>, commit: boolean) => {
    const rect = event.currentTarget.getBoundingClientRect();
    if (rect.width <= 0) return;
    const ratio = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    const timeSec = ratio * totalDurationSec;
    setHoverSeek({ ratio, timeSec, chapter: chapterForSec(timeSec) });
    if (commit) seekTo(timeSec);
  };

  const progressPct = totalDurationSec > 0 ? Math.min(100, (currentTimeSec / totalDurationSec) * 100) : 0;
  const bufferedPct = totalDurationSec > 0 ? Math.min(100, (bufferedEndSec / totalDurationSec) * 100) : 0;
  const pipSupported = typeof document !== "undefined" && Boolean(document.pictureInPictureEnabled);

  const modeBadgeLabel = session
    ? session.mode === "direct"
      ? t("videoApp.player.modeDirect")
      : session.mode === "remux"
        ? t("videoApp.player.modeRemux")
        : t("videoApp.player.modeTranscode")
    : null;

  return (
    <div
      ref={containerRef}
      role="region"
      aria-label={displayTitle}
      onMouseMove={revealControls}
      onTouchStart={revealControls}
      className={cn(
        "fixed inset-0 z-50 flex flex-col bg-black text-white select-none",
        !controlsShown && playing && "cursor-none",
      )}
    >
      {/* Video surface */}
      <video
        ref={videoRef}
        playsInline
        preload="auto"
        className="size-full object-contain"
        onClick={() => {
          if (activeMenu) {
            setActiveMenu(null);
            return;
          }
          togglePlay();
        }}
        onDoubleClick={toggleFullscreen}
        onLoadedMetadata={(e) => {
          const el = e.currentTarget;
          if (Number.isFinite(el.duration) && el.duration > 0) {
            setVideoDurationSec(el.duration);
          }
          if (pendingSeekSecRef.current !== null && session?.protocol !== "hls") {
            const target = pendingSeekSecRef.current;
            pendingSeekSecRef.current = null;
            if (target > 0) {
              const clamped = Number.isFinite(el.duration) && el.duration > 0 ? Math.min(target, Math.max(0, el.duration - 1)) : target;
              el.currentTime = clamped;
              setCurrentTimeSec(clamped);
              lastMediaTimeRef.current = clamped;
            }
          }
        }}
        onPlay={() => {
          setPlaying(true);
          dispatchExclusiveMedia();
        }}
        onPause={() => {
          setPlaying(false);
          if (lastMediaTimeRef.current > 0) {
            reporterRef.current?.flush(lastMediaTimeRef.current, false);
          }
        }}
        onWaiting={() => setBuffering(true)}
        onPlaying={() => {
          setPlaying(true);
          setBuffering(false);
        }}
        onCanPlay={() => setBuffering(false)}
        onTimeUpdate={(e) => {
          const el = e.currentTarget;
          const offsetSec = session?.protocol === "hls" ? session.startMs / 1000 : 0;
          const mediaSec = offsetSec + el.currentTime;
          setCurrentTimeSec(mediaSec);
          lastMediaTimeRef.current = mediaSec;

          if (el.buffered.length > 0) {
            setBufferedEndSec(offsetSec + el.buffered.end(el.buffered.length - 1));
          }
          if (!el.paused && !el.seeking && !el.ended) {
            reporterRef.current?.tick(mediaSec);
          }

          // Trigger next-episode countdown in final 15s for full-length episodes
          if (
            nextEpisodeTarget &&
            !countdownDismissed &&
            countdownSec === null &&
            totalDurationSec > 30 &&
            totalDurationSec - mediaSec <= 15
          ) {
            setCountdownSec(10);
          }
        }}
        onEnded={() => {
          setPlaying(false);
          reporterRef.current?.flush(0, true);
          if (nextEpisodeTarget && !countdownDismissed && countdownSec === null) {
            setCountdownSec(10);
          }
        }}
      />

      {/* Buffering / Loading overlay */}
      {(sessionLoading || buffering) && !sessionError ? (
        <div
          role="status"
          aria-label={t("videoApp.player.loading")}
          className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-3 bg-black/35"
        >
          <Spinner size="lg" color="current" className="text-white" />
          <span className="text-sm text-white/85">
            {session ? t("videoApp.player.switching") : t("videoApp.player.loading")}
          </span>
        </div>
      ) : null}

      {/* Error state overlay */}
      {sessionError ? (
        <div
          role="alert"
          className="absolute inset-0 z-20 flex flex-col items-center justify-center gap-4 bg-black/85 p-6 text-center backdrop-blur-sm"
        >
          <span className="grid size-12 place-items-center rounded-full bg-red-500/20 text-red-300">
            <AlertCircle className="size-6" aria-hidden />
          </span>
          <div className="grid max-w-md gap-1">
            <h2 className="font-heading text-lg font-title text-white">{t("videoApp.player.errorTitle")}</h2>
            <p className="text-sm text-white/75">{errorText(t, sessionError)}</p>
          </div>
          <div className="flex items-center gap-2.5">
            <button
              type="button"
              onClick={() => {
                setSessionLoading(true);
                setSessionError(null);
                setAttempt((n) => n + 1);
              }}
              className="inline-flex h-9 items-center gap-1.5 rounded-control bg-white px-4 text-sm font-strong text-neutral-950 hover:bg-neutral-200"
            >
              <RotateCw className="size-4" aria-hidden />
              {t("videoApp.player.retry")}
            </button>
            <button
              type="button"
              onClick={goBack}
              className="inline-flex h-9 items-center gap-1.5 rounded-control border border-white/25 bg-white/10 px-4 text-sm font-mid text-white hover:bg-white/20"
            >
              {t("videoApp.player.back")}
            </button>
          </div>
        </div>
      ) : null}

      {/* Resume prompt card */}
      {resumePromptMs !== null && !sessionError ? (
        <div
          role="dialog"
          aria-label={t("videoApp.player.resumeTitle", { time: formatClock(resumePromptMs) })}
          className="absolute bottom-28 left-1/2 z-20 flex w-[min(26rem,calc(100%-2rem))] -translate-x-1/2 flex-col gap-3 rounded-panel border border-white/20 bg-neutral-950/90 p-4 shadow-raised backdrop-blur-md"
        >
          <p className="text-sm font-strong text-white">
            {t("videoApp.player.resumeTitle", { time: formatClock(resumePromptMs) })}
          </p>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleResumeContinue}
              className="inline-flex h-9 flex-1 items-center justify-center gap-1.5 rounded-control bg-white px-4 text-sm font-strong text-neutral-950 hover:bg-neutral-200"
            >
              <Play className="size-4 fill-current" aria-hidden />
              {t("videoApp.player.resumeContinue")}
            </button>
            <button
              type="button"
              onClick={handleResumeRestart}
              className="inline-flex h-9 items-center justify-center gap-1.5 rounded-control border border-white/25 bg-white/10 px-4 text-sm font-mid text-white hover:bg-white/20"
            >
              <RotateCcw className="size-4" aria-hidden />
              {t("videoApp.player.resumeRestart")}
            </button>
          </div>
        </div>
      ) : null}

      {/* Next Episode countdown overlay */}
      {countdownSec !== null && nextEpisodeTarget ? (
        <div
          role="dialog"
          aria-label={t("videoApp.player.nextUpLabel")}
          className="absolute right-5 bottom-28 z-20 flex w-[min(22rem,calc(100%-2.5rem))] flex-col gap-3 rounded-panel border border-white/20 bg-neutral-950/92 p-3.5 shadow-raised backdrop-blur-md"
        >
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs font-mid text-white/75">
              {t("videoApp.player.countdown", { seconds: countdownSec })}
            </span>
            <button
              type="button"
              onClick={() => {
                setCountdownSec(null);
                setCountdownDismissed(true);
              }}
              aria-label={t("videoApp.player.cancel")}
              className="grid size-6 place-items-center rounded-control text-white/70 hover:bg-white/15 hover:text-white"
            >
              <X className="size-4" aria-hidden />
            </button>
          </div>

          <div className="flex items-center gap-3">
            <VideoImage
              path={
                nextEpisodeTarget.episode.thumbItemId
                  ? imagePaths.thumb(nextEpisodeTarget.episode.thumbItemId, 320)
                  : null
              }
              className="aspect-video w-24 shrink-0 rounded-control bg-neutral-800"
            />
            <div className="grid min-w-0 gap-0.5">
              <span className="text-xs font-mid text-white/70">
                {formatEpisodeCode(nextEpisodeTarget.episode.seasonNumber, nextEpisodeTarget.episode.episodeNumber)}
              </span>
              <span className="truncate text-sm font-strong text-white">
                {nextEpisodeTarget.episode.title}
              </span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={goNextEpisode}
              className="inline-flex h-8 flex-1 items-center justify-center gap-1.5 rounded-control bg-white px-3 text-xs font-strong text-neutral-950 hover:bg-neutral-200"
            >
              <SkipForward className="size-3.5" aria-hidden />
              {t("videoApp.player.playNow")}
            </button>
            <button
              type="button"
              onClick={() => {
                setCountdownSec(null);
                setCountdownDismissed(true);
              }}
              className="inline-flex h-8 items-center justify-center rounded-control border border-white/25 bg-white/10 px-3 text-xs font-mid text-white hover:bg-white/20"
            >
              {t("videoApp.player.cancel")}
            </button>
          </div>
        </div>
      ) : null}

      {/* Top Bar */}
      <header
        className={cn(
          "absolute inset-x-0 top-0 z-10 flex items-center gap-3 px-4 py-3.5 transition-opacity duration-200 sm:px-6",
          controlsShown ? "opacity-100" : "pointer-events-none opacity-0",
        )}
        style={{
          backgroundImage: "linear-gradient(180deg, rgba(0,0,0,0.82) 0%, rgba(0,0,0,0.35) 68%, rgba(0,0,0,0) 100%)",
        }}
      >
        <button
          type="button"
          onClick={goBack}
          aria-label={t("videoApp.player.back")}
          className="grid size-9 shrink-0 place-items-center rounded-full bg-white/12 text-white backdrop-blur-xs transition-colors hover:bg-white/24 focus-visible:outline-2 focus-visible:outline-white"
        >
          <ArrowLeft className="size-5" aria-hidden />
        </button>

        <h1 className="min-w-0 flex-1 truncate font-heading text-base font-title text-white sm:text-lg">
          {displayTitle}
        </h1>

        {modeBadgeLabel ? (
          <span
            title={session?.reason ?? undefined}
            className="shrink-0 rounded-chip border border-white/20 bg-black/55 px-2.5 py-1 text-xs font-mid text-white/90 backdrop-blur-xs"
          >
            {modeBadgeLabel}
          </span>
        ) : null}
      </header>

      {/* Bottom Controls Bar */}
      <div
        className={cn(
          "absolute inset-x-0 bottom-0 z-10 flex flex-col gap-1.5 px-3 pt-10 pb-4 transition-opacity duration-200 sm:gap-2 sm:px-6 sm:pb-5",
          controlsShown ? "opacity-100" : "pointer-events-none opacity-0",
        )}
        style={{
          backgroundImage: "linear-gradient(0deg, rgba(0,0,0,0.9) 0%, rgba(0,0,0,0.52) 65%, rgba(0,0,0,0) 100%)",
        }}
      >
        {/* Compact phone time readout above seekbar so all buttons fit cleanly inside 390px */}
        <div className="flex items-center justify-between text-xs font-mid text-white/85 tabular-nums sm:hidden">
          <span data-num>{formatClock(currentTimeSec * 1000)}</span>
          <span data-num>{formatClock(totalDurationSec * 1000)}</span>
        </div>

        {/* Interactive Seek Bar with hover timestamp preview & chapter markers */}
        <div
          role="slider"
          tabIndex={0}
          aria-label={t("videoApp.player.seek")}
          aria-valuemin={0}
          aria-valuemax={Math.round(totalDurationSec)}
          aria-valuenow={Math.round(currentTimeSec)}
          aria-valuetext={`${formatClock(currentTimeSec * 1000)} / ${formatClock(totalDurationSec * 1000)}`}
          onPointerMove={(e) => handleSeekPointer(e, e.buttons === 1)}
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            handleSeekPointer(e, true);
          }}
          onPointerLeave={() => setHoverSeek(null)}
          onKeyDown={(e) => {
            if (e.key === "ArrowLeft") {
              e.preventDefault();
              skipBy(-10);
            } else if (e.key === "ArrowRight") {
              e.preventDefault();
              skipBy(10);
            }
          }}
          className="group/seek relative flex h-6 cursor-pointer items-center outline-none focus-visible:ring-2 focus-visible:ring-white"
        >
          {/* Hover preview tooltip */}
          {hoverSeek && totalDurationSec > 0 ? (
            <div
              className="pointer-events-none absolute bottom-7 -translate-x-1/2 rounded-control border border-white/15 bg-neutral-950/92 px-2.5 py-1 text-center shadow-raised backdrop-blur-xs"
              style={{ left: `${Math.max(5, Math.min(95, hoverSeek.ratio * 100))}%` }}
            >
              <div data-num className="text-xs font-strong text-white tabular-nums">
                {formatClock(hoverSeek.timeSec * 1000)}
              </div>
              {hoverSeek.chapter ? (
                <div className="max-w-44 truncate text-[0.6875rem] text-white/75">{hoverSeek.chapter}</div>
              ) : null}
            </div>
          ) : null}

          {/* Track */}
          <div className="relative h-1.5 w-full overflow-hidden rounded-full bg-white/25 transition-all group-hover/seek:h-2">
            {/* Buffered */}
            <div
              className="absolute inset-y-0 left-0 bg-white/35"
              style={{ width: `${bufferedPct}%` }}
            />
            {/* Played */}
            <div
              className="absolute inset-y-0 left-0 bg-white"
              style={{ width: `${progressPct}%` }}
            />
            {/* Chapter ticks */}
            {totalDurationSec > 0
              ? chapters
                  .filter((ch) => ch.startMs > 0 && ch.startMs / 1000 < totalDurationSec)
                  .map((ch) => (
                    <span
                      key={ch.startMs}
                      aria-hidden
                      className="absolute inset-y-0 w-0.5 bg-black/75"
                      style={{ left: `${(ch.startMs / 1000 / totalDurationSec) * 100}%` }}
                    />
                  ))
              : null}
          </div>

          {/* Thumb */}
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 size-3.5 -translate-x-1/2 -translate-y-1/2 scale-0 rounded-full bg-white shadow-sm transition-transform group-focus-visible/seek:scale-100 group-hover/seek:scale-100"
            style={{ left: `${progressPct}%` }}
          />
        </div>

        {/* Control Buttons Row */}
        <div className="relative flex items-center justify-between gap-1 sm:gap-2">
          {/* Left group */}
          <div className="flex items-center gap-0.5 sm:gap-1.5">
            <button
              type="button"
              onClick={togglePlay}
              aria-label={t(playing ? "videoApp.player.pause" : "videoApp.player.play")}
              className="grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9"
            >
              {playing ? <Pause className="size-5 fill-current" aria-hidden /> : <Play className="ms-0.5 size-5 fill-current" aria-hidden />}
            </button>

            <button
              type="button"
              onClick={() => skipBy(-10)}
              aria-label={t("videoApp.player.rewind")}
              className="grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9"
            >
              <RotateCcw className="size-4" aria-hidden />
            </button>

            <button
              type="button"
              onClick={() => skipBy(10)}
              aria-label={t("videoApp.player.forward")}
              className="grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9"
            >
              <RotateCw className="size-4" aria-hidden />
            </button>

            {nextEpisodeTarget ? (
              <button
                type="button"
                onClick={goNextEpisode}
                aria-label={t("videoApp.player.nextEpisode")}
                className="grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9"
              >
                <SkipForward className="size-4" aria-hidden />
              </button>
            ) : null}

            {/* Volume + slider */}
            <div className="hidden items-center gap-1.5 sm:flex">
              <button
                type="button"
                onClick={toggleMute}
                aria-label={t(muted || volume === 0 ? "videoApp.player.unmute" : "videoApp.player.mute")}
                className="grid size-9 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white"
              >
                {muted || volume === 0 ? <VolumeX className="size-4" aria-hidden /> : <Volume2 className="size-4" aria-hidden />}
              </button>
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={muted ? 0 : volume}
                aria-label={t("videoApp.player.volume")}
                onChange={(e) => updateVolume(Number(e.target.value))}
                className="h-1 w-20 cursor-pointer accent-white"
              />
            </div>

            <span data-num className="ms-1 hidden text-sm font-mid text-white/90 tabular-nums sm:inline">
              {formatClock(currentTimeSec * 1000)}
              <span className="mx-1 text-white/50">/</span>
              {formatClock(totalDurationSec * 1000)}
            </span>
          </div>

          {/* Right group */}
          <div className="relative flex items-center gap-0.5 sm:gap-1.5">
            {/* Subtitles */}
            {subtitleTracks.length > 0 ? (
              <button
                type="button"
                aria-expanded={activeMenu === "subtitles"}
                aria-label={t("videoApp.player.subtitles")}
                onClick={() => setActiveMenu((m) => (m === "subtitles" ? null : "subtitles"))}
                className={cn(
                  "grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9",
                  selectedSubId && "bg-white/20",
                )}
              >
                <Subtitles className="size-4" aria-hidden />
              </button>
            ) : null}

            {/* Audio Track */}
            {audioTracks.length > 1 ? (
              <button
                type="button"
                aria-expanded={activeMenu === "audio"}
                aria-label={t("videoApp.player.audioTrack")}
                onClick={() => setActiveMenu((m) => (m === "audio" ? null : "audio"))}
                className={cn(
                  "grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9",
                  activeMenu === "audio" && "bg-white/20",
                )}
              >
                <Languages className="size-4" aria-hidden />
              </button>
            ) : null}

            {/* Quality */}
            <button
              type="button"
              aria-expanded={activeMenu === "quality"}
              aria-label={t("videoApp.player.quality")}
              onClick={() => setActiveMenu((m) => (m === "quality" ? null : "quality"))}
              className={cn(
                "grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9",
                activeMenu === "quality" && "bg-white/20",
              )}
            >
              <Settings className="size-4" aria-hidden />
            </button>

            {/* Playback Speed */}
            <button
              type="button"
              aria-expanded={activeMenu === "speed"}
              aria-label={t("videoApp.player.speed")}
              onClick={() => setActiveMenu((m) => (m === "speed" ? null : "speed"))}
              className={cn(
                "inline-flex h-8 items-center gap-1 rounded-control px-1.5 text-xs font-strong text-white tabular-nums hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:h-9 sm:px-2",
                activeMenu === "speed" && "bg-white/20",
              )}
            >
              <Gauge className="size-4 max-sm:hidden" aria-hidden />
              {speed === 1 ? "1×" : `${speed}×`}
            </button>

            {/* Picture-in-Picture */}
            {pipSupported ? (
              <button
                type="button"
                onClick={togglePip}
                aria-label={t(isPip ? "videoApp.player.exitPip" : "videoApp.player.pip")}
                className="hidden size-9 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:grid"
              >
                <PictureInPicture2 className="size-4" aria-hidden />
              </button>
            ) : null}

            {/* Fullscreen */}
            <button
              type="button"
              onClick={toggleFullscreen}
              aria-label={t(isFullscreen ? "videoApp.player.exitFullscreen" : "videoApp.player.fullscreen")}
              className="grid size-8 place-items-center rounded-control text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white sm:size-9"
            >
              {isFullscreen ? <Minimize className="size-4" aria-hidden /> : <Maximize className="size-4" aria-hidden />}
            </button>

            {/* Floating Menu Popovers */}
            {activeMenu ? (
              <div
                role="menu"
                aria-label={t(
                  activeMenu === "subtitles"
                    ? "videoApp.player.subtitles"
                    : activeMenu === "audio"
                      ? "videoApp.player.audioTrack"
                      : activeMenu === "quality"
                        ? "videoApp.player.quality"
                        : "videoApp.player.speed",
                )}
                className="absolute right-0 bottom-12 z-30 min-w-52 rounded-panel border border-white/15 bg-neutral-950/95 p-1.5 shadow-raised backdrop-blur-md"
              >
                {activeMenu === "subtitles" ? (
                  <>
                    <MenuOption
                      selected={!selectedSubId}
                      label={t("videoApp.player.subtitlesOff")}
                      onSelect={() => {
                        setSelectedSubId("");
                        setActiveMenu(null);
                      }}
                    />
                    {subtitleTracks.map((track: SubtitleTrack) => (
                      <MenuOption
                        key={track.id}
                        selected={selectedSubId === track.id}
                        label={subtitleTrackLabel(track, locale, t)}
                        onSelect={() => {
                          setSelectedSubId(track.id);
                          lastSubIdRef.current = track.id;
                          setActiveMenu(null);
                        }}
                      />
                    ))}
                  </>
                ) : null}

                {activeMenu === "audio"
                  ? audioTracks.map((track) => {
                      const activeIdx = audioIndex ?? audioTracks.find((a) => a.default)?.index ?? audioTracks[0]?.index;
                      const label = [
                        track.title ?? track.language?.toUpperCase() ?? t("videoApp.player.unspecifiedLang"),
                        codecLabel(track.codec),
                        track.channelLayout ?? (track.channels ? t("videoApp.player.channels", { count: track.channels }) : null),
                      ]
                        .filter(Boolean)
                        .join(" · ");
                      return (
                        <MenuOption
                          key={track.index}
                          selected={activeIdx === track.index}
                          label={label}
                          onSelect={() => handleAudioChange(track.index)}
                        />
                      );
                    })
                  : null}

                {activeMenu === "quality"
                  ? QUALITY_OPTIONS.map((opt) => (
                      <MenuOption
                        key={opt.id}
                        selected={quality === opt.id}
                        label={opt.labelKey ? t(opt.labelKey as "videoApp.player.qualityAuto") : (opt.label ?? opt.id)}
                        onSelect={() => handleQualityChange(opt.id)}
                      />
                    ))
                  : null}

                {activeMenu === "speed"
                  ? SPEED_OPTIONS.map((s) => (
                      <MenuOption
                        key={s}
                        selected={speed === s}
                        label={`${s}×`}
                        onSelect={() => handleSpeedChange(s)}
                      />
                    ))
                  : null}
              </div>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}

function MenuOption({
  label,
  selected,
  onSelect,
}: {
  label: string;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitemradio"
      aria-checked={selected}
      onClick={onSelect}
      className={cn(
        "flex w-full items-center justify-between gap-3 rounded-control px-3 py-2 text-start text-sm transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white",
        selected ? "font-strong text-white" : "text-white/80",
      )}
    >
      <span className="truncate">{label}</span>
      {selected ? <Check className="size-4 shrink-0 text-white" aria-hidden /> : null}
    </button>
  );
}
