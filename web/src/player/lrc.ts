/**
 * Synced LRC lyrics parser. Supports [mm:ss], [mm:ss.xx], [mm:ss.xxx] and multiple timestamps per
 * line (chorus compression), stripping metadata tags like [ti:] / [ar:].
 */

export interface LrcLine {
  timeMs: number;
  text: string;
}

const TIMESTAMP_RE = /\[(\d{1,3}):(\d{2})(?:[.:](\d{1,3}))?\]/g;

export function parseLrc(raw: string): LrcLine[] {
  const out: LrcLine[] = [];
  for (const rawLine of raw.split(/\r?\n/)) {
    const times: number[] = [];
    TIMESTAMP_RE.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = TIMESTAMP_RE.exec(rawLine)) !== null) {
      const minutes = Number(match[1]);
      const seconds = Number(match[2]);
      const fracRaw = match[3] ?? "";
      const fracMs = fracRaw
        ? fracRaw.length === 1
          ? Number(fracRaw) * 100
          : fracRaw.length === 2
            ? Number(fracRaw) * 10
            : Number(fracRaw)
        : 0;
      times.push((minutes * 60 + seconds) * 1000 + fracMs);
    }
    if (times.length === 0) continue;
    const text = rawLine.replace(TIMESTAMP_RE, "").trim();
    if (!text) continue;
    for (const timeMs of times) out.push({ timeMs, text });
  }
  out.sort((a, b) => a.timeMs - b.timeMs);
  return out;
}

/** Returns the index of the currently active lyric line (or -1 before the first timestamp). */
export function activeLrcIndex(lines: readonly LrcLine[], currentTimeMs: number): number {
  if (lines.length === 0) return -1;
  let lo = 0;
  let hi = lines.length - 1;
  let result = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >>> 1;
    if (lines[mid]!.timeMs <= currentTimeMs) {
      result = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return result;
}
