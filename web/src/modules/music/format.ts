import type { MusicTrack } from "./api";

/** Formats milliseconds or seconds into classic music time "3:45" or "1:04:12". */
export function formatTrackTime(ms: number | null | undefined): string {
  if (ms == null || !Number.isFinite(ms) || ms < 0) return "—";
  const totalSec = Math.floor(ms / 1000);
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  const seconds = totalSec % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function formatSeconds(sec: number): string {
  if (!Number.isFinite(sec) || sec < 0) return "0:00";
  return formatTrackTime(sec * 1000);
}

/** Human-readable album/playlist total length, e.g. "48 分钟" / "1 小时 12 分钟" or "48 min". */
export function formatCollectionDuration(ms: number, locale: string): string {
  if (!Number.isFinite(ms) || ms <= 0) return "";
  const totalMin = Math.max(1, Math.round(ms / 60_000));
  const hours = Math.floor(totalMin / 60);
  const minutes = totalMin % 60;
  const zh = locale.startsWith("zh");
  if (hours === 0) return zh ? `${minutes} 分钟` : `${minutes} min`;
  if (minutes === 0) return zh ? `${hours} 小时` : `${hours} hr`;
  return zh ? `${hours} 小时 ${minutes} 分钟` : `${hours} hr ${minutes} min`;
}

/** Concise audio quality spec, e.g. "FLAC · 24-bit / 96 kHz" or "MP3 · 320 kbps". */
export function formatAudioSpecs(track: Pick<MusicTrack, "codec" | "bitrateKbps" | "sampleRateHz" | "bitDepth">): string | null {
  const parts: string[] = [];
  if (track.codec) parts.push(track.codec.toUpperCase());
  const lossless = track.codec === "flac" || track.codec === "alac" || track.codec === "wav" || track.codec === "aiff";
  if (lossless && (track.bitDepth || track.sampleRateHz)) {
    const rate = track.sampleRateHz ? `${Number((track.sampleRateHz / 1000).toFixed(1))} kHz` : null;
    const depth = track.bitDepth ? `${track.bitDepth}-bit` : null;
    parts.push([depth, rate].filter(Boolean).join(" / "));
  } else if (track.bitrateKbps) {
    parts.push(`${track.bitrateKbps} kbps`);
  }
  return parts.length > 0 ? parts.join(" · ") : null;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
