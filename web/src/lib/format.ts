const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

/** Binary-based sizes with the familiar short units (1 GB = 1024³ bytes), as NAS UIs show them. */
export function formatBytes(bytes: number, locale: string): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < BYTE_UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = unit === 0 || value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value)} ${BYTE_UNITS[unit]}`;
}

export function formatNumber(value: number, locale: string): string {
  return new Intl.NumberFormat(locale).format(value);
}

export function formatPercent(fraction: number, locale: string): string {
  return new Intl.NumberFormat(locale, { style: "percent", maximumFractionDigits: 0 }).format(
    Math.min(1, Math.max(0, fraction)),
  );
}

export function formatDateTime(iso: string, locale: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(date);
}

const RELATIVE_STEPS: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ["second", 60],
  ["minute", 60],
  ["hour", 24],
  ["day", 7],
  ["week", 4.345],
  ["month", 12],
  ["year", Number.POSITIVE_INFINITY],
];

/** Chinese typesetting puts a space between digits and Han characters ("14 分钟前"). */
export function spaceDigits(text: string): string {
  return text.replace(/(\d)(\p{Script=Han})/gu, "$1 $2").replace(/(\p{Script=Han})(\d)/gu, "$1 $2");
}

/**
 * "3 小时前", "in 2 days". Under 45 seconds the moment reads as `justNow` when given ("刚刚"):
 * Intl's own word for it ("现在", "now") suits a clock, not an event that already happened.
 */
export function formatRelativeTime(iso: string, locale: string, now = Date.now(), justNow?: string): string {
  const time = Date.parse(iso);
  if (Number.isNaN(time)) return iso;
  let delta = (time - now) / 1000;
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  if (Math.abs(delta) < 45) return justNow ?? formatter.format(0, "second");
  for (const [unit, size] of RELATIVE_STEPS) {
    if (Math.abs(delta) < size) return spaceDigits(formatter.format(Math.round(delta), unit));
    delta /= size;
  }
  return spaceDigits(formatter.format(Math.round(delta), "year"));
}

/** Compact duration such as "4.2s", "3m 05s" or "1h 12m". */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds < 10 ? seconds.toFixed(1) : Math.round(seconds)}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${String(Math.round(seconds % 60)).padStart(2, "0")}s`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${String(minutes % 60).padStart(2, "0")}m`;
}

/** Stable 32-bit hash for deterministic per-item variation (placeholder artwork). */
export function hashString(input: string): number {
  let hash = 2166136261;
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}
