/**
 * Progress, drawn the way the skin's `progress` slot says:
 *   bar       — a thin bar in its own column
 *   separator — the row's bottom rule is the progress line (with a "now" tick)
 *   row-fill  — the whole row fills from the left, with an exact edge along the bottom
 * Unknown progress is never drawn as 0%: each variant has a moving or dashed form.
 */
import type { CSSProperties, ReactNode } from "react";
import { cn } from "@/lib/cn";
import { useSlot } from "@/skins/scope";

export type ProgressTone = "active" | "neutral" | "failed";

function percent(value: number | null): number | null {
  return value === null ? null : Math.round(Math.min(1, Math.max(0, value)) * 1000) / 10;
}

/** Thin bar; also the meter used for readings (CPU, storage) outside the separator variant. */
export function ProgressBar({
  value,
  label,
  tone = "active",
  className,
  style,
}: {
  value: number | null;
  label: string;
  tone?: ProgressTone;
  className?: string;
  style?: CSSProperties;
}) {
  const p = percent(value);
  const fill = tone === "failed" ? "bg-failed" : tone === "neutral" ? "bg-progress-done" : "bg-(--run-bar,var(--pv-active-mark))";
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={p ?? undefined}
      className={cn("relative h-1 w-full overflow-hidden rounded-full bg-progress-rest", className)}
      style={style}
    >
      {p === null ? (
        <div className={cn("pv-sweep absolute inset-y-0 left-0 w-2/5 rounded-full", fill)} />
      ) : (
        <div className={cn("h-full rounded-full transition-[width] duration-500", fill)} style={{ width: `${p}%` }} />
      )}
    </div>
  );
}

/**
 * Wraps one row of running work and draws its progress per the slot. In the `bar` variant the
 * row places a <ProgressBar> itself (it needs a column); this wrapper then draws nothing extra.
 * `wash`/`edge` override the colours (work colour in skins that use it).
 */
export function RunProgress({
  value,
  label,
  running = true,
  wash,
  edge,
  className,
  children,
}: {
  value: number | null;
  label: string;
  running?: boolean;
  wash?: string;
  edge?: string;
  className?: string;
  children: ReactNode;
}) {
  const variant = useSlot("progress");
  const p = percent(value);
  const hidden = (
    <span role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={p ?? undefined} className="sr-only" />
  );
  if (variant === "row-fill" && running) {
    return (
      <div className={cn("relative isolate", className)}>
        {p !== null ? (
          <div aria-hidden className="absolute inset-y-0 left-0 -z-10 transition-[width] duration-700" style={{ width: `${p}%`, background: wash ?? "var(--pv-active-soft)" }} />
        ) : null}
        <div aria-hidden className="absolute inset-x-0 bottom-0 -z-10 h-[3px] overflow-hidden">
          {p !== null ? (
            <div className="h-full transition-[width] duration-700" style={{ width: `${p}%`, background: edge ?? "var(--pv-progress-done)" }} />
          ) : (
            <div className="pv-sweep h-full w-1/4 rounded-full" style={{ background: edge ?? "var(--pv-progress-done)" }} />
          )}
        </div>
        {hidden}
        {children}
      </div>
    );
  }
  if (variant === "separator") {
    return (
      <div className={cn("relative", className)}>
        <div aria-hidden className="absolute inset-x-0 bottom-0 h-px bg-line" />
        {running ? (
          p !== null ? (
            <div aria-hidden className="absolute -bottom-px left-0 h-0.5 bg-progress-done transition-[width] duration-700" style={{ width: `${p}%` }}>
              <span className="absolute -top-[3px] -right-px h-2 w-0.5 bg-progress-done" />
            </div>
          ) : (
            <div aria-hidden className="pv-dashed absolute inset-x-0 -bottom-px h-0.5 text-progress-done" />
          )
        ) : null}
        {running ? hidden : null}
        {children}
      </div>
    );
  }
  return <div className={className}>{children}</div>;
}

/**
 * A reading with a known range (storage, memory): neutral data, not work in progress, so it is
 * drawn in the secondary text colour. `separator` skins draw it as the row's own bottom line;
 * the others as a small bar beside the reading.
 */
export function ReadingMeter({ value, label, warn = false }: { value: number; label: string; warn?: boolean }) {
  const variant = useSlot("progress");
  const p = percent(value) ?? 0;
  if (variant === "separator") {
    return (
      <div role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={p} className="absolute inset-x-0 bottom-0 h-px bg-line">
        <div className={cn("h-full", warn ? "bg-degraded" : "bg-fg-muted")} style={{ width: `${p}%` }} />
      </div>
    );
  }
  return (
    <div role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={p} className="h-1.5 w-full overflow-hidden rounded-full bg-progress-rest">
      <div className={cn("h-full rounded-full", warn ? "bg-degraded" : "bg-fg-muted")} style={{ width: `${p}%` }} />
    </div>
  );
}
