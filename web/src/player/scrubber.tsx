import { useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { clamp } from "@/modules/music/format";

/**
 * Accessible range scrubber for playback progress and volume.
 * Uses a native range input layered over a crisp custom track so keyboard, screen-reader,
 * pointer drag and touch interactions all work natively across every browser and skin.
 */
export function Scrubber({
  value,
  max,
  buffered = 0,
  step = 0.5,
  label,
  valueText,
  dark = false,
  compact = false,
  onChange,
  onCommit,
  className,
}: {
  value: number;
  max: number;
  buffered?: number;
  step?: number;
  label: string;
  valueText?: string;
  dark?: boolean;
  compact?: boolean;
  onChange?: (next: number) => void;
  onCommit: (next: number) => void;
  className?: string;
}) {
  const [dragging, setDragging] = useState<number | null>(null);
  const draggingRef = useRef<number | null>(null);
  const safeMax = max > 0 ? max : 1;
  const shown = dragging ?? value;
  const percent = clamp((shown / safeMax) * 100, 0, 100);
  const bufferedPercent = clamp((buffered / safeMax) * 100, 0, 100);

  return (
    <div
      className={cn(
        "group relative flex w-full items-center select-none",
        compact ? "h-4" : "h-5",
        className,
      )}
    >
      <div
        aria-hidden
        className={cn(
          "relative w-full overflow-hidden rounded-full transition-[height] duration-150",
          compact ? "h-1 group-hover:h-1.5" : "h-1.5 group-hover:h-2",
          dark ? "bg-white/20" : "bg-progress-rest",
        )}
      >
        {buffered > 0 ? (
          <div
            className={cn("absolute inset-y-0 start-0 rounded-full", dark ? "bg-white/25" : "bg-fg/15")}
            style={{ width: `${bufferedPercent}%` }}
          />
        ) : null}
        <div
          className={cn("absolute inset-y-0 start-0 rounded-full", dark ? "bg-white" : "bg-action")}
          style={{ width: `${percent}%` }}
        />
      </div>
      <span
        aria-hidden
        className={cn(
          "pointer-events-none absolute top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full shadow-raised transition-opacity duration-150",
          compact ? "size-2.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100" : "size-3.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100",
          dragging !== null && "opacity-100 scale-110",
          dark ? "bg-white" : "bg-surface ring-2 ring-action",
        )}
        style={{ left: `${percent}%` }}
      />
      <input
        type="range"
        min={0}
        max={safeMax}
        step={step}
        value={clamp(shown, 0, safeMax)}
        aria-label={label}
        aria-valuetext={valueText}
        onChange={(event) => {
          const next = Number(event.target.value);
          draggingRef.current = next;
          setDragging(next);
          onChange?.(next);
        }}
        onPointerUp={() => {
          if (draggingRef.current !== null) {
            onCommit(draggingRef.current);
            draggingRef.current = null;
            setDragging(null);
          }
        }}
        onKeyUp={() => {
          if (draggingRef.current !== null) {
            onCommit(draggingRef.current);
            draggingRef.current = null;
            setDragging(null);
          }
        }}
        onBlur={() => {
          if (draggingRef.current !== null) {
            onCommit(draggingRef.current);
            draggingRef.current = null;
            setDragging(null);
          }
        }}
        className="absolute inset-0 size-full cursor-pointer opacity-0"
      />
    </div>
  );
}
