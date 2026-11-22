import type { TFunction } from "i18next";
import type { MediaInfo, VideoEntry } from "./api";

/** Formats milliseconds as `H:MM:SS` or `M:SS` for player clocks and resume timestamps. */
export function formatClock(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "0:00";
  const totalSec = Math.floor(ms / 1000);
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  const seconds = totalSec % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

/** Human-friendly runtime such as "2 小时 14 分钟" / "48 分钟" / "2h 14m". */
export function formatRuntime(ms: number | null | undefined, t: TFunction, compact = false): string | null {
  if (ms == null || !Number.isFinite(ms) || ms <= 0) return null;
  const totalMin = Math.max(1, Math.round(ms / 60_000));
  const hours = Math.floor(totalMin / 60);
  const minutes = totalMin % 60;
  if (hours > 0) {
    return compact
      ? t("videoApp.time.shortHoursMinutes", { hours, minutes })
      : t("videoApp.time.hoursMinutes", { hours, minutes });
  }
  return compact
    ? t("videoApp.time.shortMinutes", { minutes: totalMin })
    : t("videoApp.time.minutes", { minutes: totalMin });
}

/** Formats S01E03 or E03 cleanly. */
export function formatEpisodeCode(season: number | null | undefined, episode: number | null | undefined): string | null {
  if (episode == null) return null;
  const ep = `E${String(episode).padStart(2, "0")}`;
  if (season == null) return ep;
  return `S${String(season).padStart(2, "0")}${ep}`;
}

/** Progress fraction 0–1 for partially watched items (null when unstarted or completed). */
export function watchProgress(entry: Pick<VideoEntry, "runtimeMs" | "state">): number | null {
  const pos = entry.state?.positionMs ?? 0;
  if (!entry.state || entry.state.played || pos <= 0) return null;
  if (!entry.runtimeMs || entry.runtimeMs <= 0) return 0.15;
  return Math.min(0.98, Math.max(0.03, pos / entry.runtimeMs));
}

export function remainingMs(entry: Pick<VideoEntry, "runtimeMs" | "state">): number | null {
  const pos = entry.state?.positionMs ?? 0;
  if (!entry.runtimeMs || entry.runtimeMs <= 0 || pos <= 0) return null;
  return Math.max(60_000, entry.runtimeMs - pos);
}

/** Resolution label from video pixel dimensions (4K / 1080p / 720p / 480p / SD). */
export function resolutionLabel(video: NonNullable<MediaInfo["video"]>): string {
  if (video.width >= 3800 || video.height >= 2100) return "4K";
  if (video.width >= 1900 || video.height >= 1060) return "1080p";
  if (video.width >= 1260 || video.height >= 700) return "720p";
  if (video.height >= 470) return "480p";
  return `${video.height}p`;
}

export function hdrLabel(hdr: NonNullable<MediaInfo["video"]>["hdr"]): string | null {
  switch (hdr) {
    case "dolby-vision":
      return "Dolby Vision";
    case "hdr10":
      return "HDR10";
    case "hlg":
      return "HLG";
    default:
      return null;
  }
}

export function codecLabel(codec: string): string {
  switch (codec.toLowerCase()) {
    case "hevc":
    case "h265":
      return "HEVC";
    case "h264":
    case "avc":
      return "H.264";
    case "av1":
      return "AV1";
    case "vp9":
      return "VP9";
    case "eac3":
      return "E-AC-3";
    case "ac3":
      return "AC-3";
    case "truehd":
      return "TrueHD";
    case "dts":
      return "DTS";
    case "aac":
      return "AAC";
    case "flac":
      return "FLAC";
    case "opus":
      return "Opus";
    default:
      return codec.toUpperCase();
  }
}

export function formatBitrate(kbps: number | null | undefined): string | null {
  if (kbps == null || !Number.isFinite(kbps) || kbps <= 0) return null;
  if (kbps >= 1000) return `${(kbps / 1000).toFixed(1)} Mbps`;
  return `${Math.round(kbps)} kbps`;
}

/** Up to 2 initials for cast members without a portrait image. */
export function personInitials(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return "?";
  const parts = trimmed.split(/\s+/);
  if (parts.length >= 2) {
    return `${parts[0]!.charAt(0)}${parts[parts.length - 1]!.charAt(0)}`.toUpperCase();
  }
  return [...trimmed].slice(0, 2).join("").toUpperCase();
}
