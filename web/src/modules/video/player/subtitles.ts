/**
 * WebVTT cue timestamp offset utility for HLS transcode sessions that begin at `startMs > 0`.
 * Shifting cues in the VTT source works uniformly across Chromium, Firefox and Safari.
 */
import type { TFunction } from "i18next";
import type { SubtitleTrack } from "../api";

const CUE_TIMING =
  /^((?:\d{2,}:)?\d{2}:\d{2}\.\d{3})\s*-->\s*((?:\d{2,}:)?\d{2}:\d{2}\.\d{3})(.*)$/;

export function parseVttTimestamp(raw: string): number {
  const parts = raw.trim().split(":");
  let hours = 0;
  let minutes = 0;
  let secPart = "0";
  if (parts.length === 3) {
    hours = Number(parts[0]);
    minutes = Number(parts[1]);
    secPart = parts[2]!;
  } else if (parts.length === 2) {
    minutes = Number(parts[0]);
    secPart = parts[1]!;
  }
  const [secStr, msStr = "0"] = secPart.split(".");
  return ((hours * 3600 + minutes * 60 + Number(secStr)) * 1000) + Number(msStr.padEnd(3, "0").slice(0, 3));
}

export function formatVttTimestamp(ms: number): string {
  const clamped = Math.max(0, Math.round(ms));
  const hours = Math.floor(clamped / 3_600_000);
  const minutes = Math.floor((clamped % 3_600_000) / 60_000);
  const seconds = Math.floor((clamped % 60_000) / 1000);
  const millis = clamped % 1000;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}.${String(millis).padStart(3, "0")}`;
}

/**
 * Subtracts `offsetMs` from every cue in a WebVTT file.
 * Cues that end before the offset are dropped; cues spanning the offset are clamped to `00:00:00.000`.
 */
export function offsetWebVtt(vtt: string, offsetMs: number): string {
  if (!Number.isFinite(offsetMs) || offsetMs <= 0) return vtt;
  const blocks = vtt.replace(/\r\n/g, "\n").split(/\n{2,}/);
  const out: string[] = [];

  for (const block of blocks) {
    const lines = block.split("\n");
    const timingIndex = lines.findIndex((line) => CUE_TIMING.test(line.trim()));
    if (timingIndex === -1) {
      out.push(block);
      continue;
    }
    const match = CUE_TIMING.exec(lines[timingIndex]!.trim());
    if (!match) {
      out.push(block);
      continue;
    }
    const start = parseVttTimestamp(match[1]!) - offsetMs;
    const end = parseVttTimestamp(match[2]!) - offsetMs;
    if (end <= 0) continue;

    const shifted = `${formatVttTimestamp(Math.max(0, start))} --> ${formatVttTimestamp(end)}${match[3] ?? ""}`;
    const rebuilt = [...lines];
    rebuilt[timingIndex] = shifted;
    out.push(rebuilt.join("\n"));
  }

  return `${out.join("\n\n")}\n`;
}

export function subtitleTrackLabel(track: SubtitleTrack, locale: string, t: TFunction): string {
  let lang: string | null = null;
  if (track.language) {
    try {
      const names = new Intl.DisplayNames(locale, { type: "language" });
      lang = names.of(track.language) ?? track.language;
    } catch {
      lang = track.language;
    }
  }
  return [
    lang ?? t("videoApp.player.unspecifiedLang"),
    track.forced ? t("videoApp.player.forced") : null,
    track.hearingImpaired ? t("videoApp.player.sdh") : null,
  ]
    .filter(Boolean)
    .join(" · ");
}
