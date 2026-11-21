import { cn } from "@/lib/cn";
import { useSlot } from "@/skins/scope";

/**
 * A count of things waiting for the user (navigation badge, inbox), marked the same way the
 * `attention` slot marks the items themselves: a seal, a signal-coloured block, a tint, or a
 * solid pill.
 */
export function CountMark({ count, className }: { count: number; className?: string }) {
  const attention = useSlot("attention");
  if (count <= 0) return null;
  const text = count > 99 ? "99+" : String(count);
  return (
    <span
      data-num
      className={cn(
        "inline-grid h-[1.125rem] min-w-[1.125rem] place-items-center px-1 text-[0.6875rem] leading-none font-strong tabular-nums",
        attention === "seal" && "pv-seal rounded-[2px]",
        attention === "lamp" && "rounded-[3px] bg-attention-mark text-(--pv-attention-mark-fg)",
        attention === "tint" && "rounded-full bg-attention-soft text-attention ring-1 ring-attention-line ring-inset",
        attention === "thumbs" && "rounded-full bg-action text-action-fg",
        className,
      )}
    >
      {text}
    </span>
  );
}
