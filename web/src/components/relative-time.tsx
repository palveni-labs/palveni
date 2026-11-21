import { useRelativeTime } from "@/lib/use-relative-time";

/**
 * `<time>` with a relative label and the absolute time as tooltip. `format` wraps the relative
 * label in a sentence ("Scanned 3 hours ago").
 */
export function RelativeTime({
  value,
  className,
  format,
}: {
  value: string;
  className?: string;
  format?: (relative: string) => string;
}) {
  const { label, title } = useRelativeTime(value);
  return (
    <time dateTime={value} title={title} className={className}>
      {format ? format(label) : label}
    </time>
  );
}
