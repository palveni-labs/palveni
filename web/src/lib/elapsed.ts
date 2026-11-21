type DurationParts = { hours?: number; minutes?: number; seconds?: number };
type DurationFormatter = new (locale: string, options: { style: "short" }) => { format: (value: DurationParts) => string };

/**
 * A span of time in the reader's language: "14分钟", "1小时12分钟", "1 hr, 12 min". Seconds are
 * shown only under a minute; hours drop the seconds.
 */
export function formatElapsed(ms: number, locale: string): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  const total = Math.max(1, Math.round(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const parts: DurationParts = hours > 0 ? { hours, minutes } : minutes > 0 ? { minutes } : { seconds: total };
  const DurationFormat = (Intl as unknown as { DurationFormat?: DurationFormatter }).DurationFormat;
  if (DurationFormat) return new DurationFormat(locale, { style: "short" }).format(parts);
  const unit = (value: number, name: "hour" | "minute" | "second") =>
    new Intl.NumberFormat(locale, { style: "unit", unit: name, unitDisplay: "short" }).format(value);
  return [
    parts.hours ? unit(parts.hours, "hour") : null,
    parts.minutes ? unit(parts.minutes, "minute") : null,
    parts.seconds ? unit(parts.seconds, "second") : null,
  ]
    .filter(Boolean)
    .join(" ");
}
