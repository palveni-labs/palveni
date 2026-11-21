/**
 * Small marks the slots decide: how a measured value is set (`measure`), how a "needs you" item
 * is marked (`attention`), and the status lamp/label vocabulary shared by every skin.
 */
import { AlertTriangle, CheckCircle2, CircleDashed, CircleDot, Loader2, XCircle, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { useSlot } from "@/skins/scope";
import { Cover } from "./work";

/** A measured value with its unit. Monospace only where the skin says readings are mono. */
export function Measure({ value, unit, className }: { value: ReactNode; unit?: ReactNode; className?: string }) {
  const measure = useSlot("measure");
  return (
    <span data-num className={cn("whitespace-nowrap tabular-nums", className)}>
      <span className={measure === "mono" ? "font-mono" : undefined}>{value}</span>
      {unit ? <span className={cn("ms-0.5", measure === "mono" ? "font-sans text-[0.86em] font-normal text-fg-muted" : undefined)}>{unit}</span> : null}
    </span>
  );
}

export type StatusTone = "active" | "attention" | "ok" | "degraded" | "failed" | "idle";

const TONE_TEXT: Record<StatusTone, string> = {
  active: "text-active",
  attention: "text-attention",
  ok: "text-ok",
  degraded: "text-degraded",
  failed: "text-failed",
  idle: "text-idle",
};

const TONE_MARK: Record<StatusTone, string> = {
  active: "bg-active-mark",
  attention: "bg-attention-mark",
  ok: "bg-ok",
  degraded: "bg-degraded",
  failed: "bg-failed",
  idle: "bg-transparent ring-1 ring-inset ring-idle",
};

/** A status lamp: always next to a word, never the only cue. */
export function Lamp({ tone, className }: { tone: StatusTone; className?: string }) {
  return <span aria-hidden className={cn("inline-block size-2 shrink-0 rounded-full", TONE_MARK[tone], className)} />;
}

const TONE_ICON: Record<StatusTone, LucideIcon> = {
  active: Loader2,
  attention: CircleDot,
  ok: CheckCircle2,
  degraded: AlertTriangle,
  failed: XCircle,
  idle: CircleDashed,
};

/** Icon (or lamp) plus words in the state's colour. */
export function StatusLabel({ tone, children, lamp = false, className }: { tone: StatusTone; children: ReactNode; lamp?: boolean; className?: string }) {
  const Icon = TONE_ICON[tone];
  return (
    <span className={cn("inline-flex items-center gap-1.5 font-mid", TONE_TEXT[tone], className)}>
      {lamp ? <Lamp tone={tone} /> : <Icon className={cn("size-[1.05em] shrink-0", tone === "active" && "pv-spin")} aria-hidden />}
      {children}
    </span>
  );
}

/** The seal (朱印): a square stamp with an inner frame. */
export function Seal({ size = 12, children, className }: { size?: number; children?: ReactNode; className?: string }) {
  return (
    <span aria-hidden={children === undefined} className={cn("pv-seal shrink-0", className)} style={{ width: size, height: size, fontSize: size * 0.62 }}>
      {children}
    </span>
  );
}

/**
 * Leading mark of an item that waits for the user, per the `attention` slot. `previews` are the
 * covers of what the item would change (shown by the `thumbs` variant).
 */
export function AttentionMark({ previews = [], title, icon: Icon = CircleDot }: { previews?: Array<string | null>; title: string; icon?: LucideIcon }) {
  const attention = useSlot("attention");
  switch (attention) {
    case "seal":
      return <Seal size={13} className="mt-[0.3em]" />;
    case "lamp":
      return <Lamp tone="attention" className="mt-[0.45em] size-2.5" />;
    case "thumbs":
      return <CoverPeek artworkIds={previews} title={title} />;
    case "tint":
      return <Icon className="mt-[0.15em] size-[1.15rem] shrink-0 text-attention-mark" aria-hidden />;
  }
}

/** Up to eight covers in two rows of four: what a batch change would touch. `null` draws a blank cover. */
export function CoverPeek({ artworkIds, title }: { artworkIds: Array<string | null>; title: string }) {
  const ids = artworkIds.slice(0, 8);
  return (
    <span role="img" aria-label={title} className="grid w-[7rem] shrink-0 grid-cols-4 gap-[3px]">
      {ids.map((id, index) => (
        <Cover key={id ?? `blank-${index}`} artworkId={id} title="" decorative className="aspect-square w-full rounded-[3px]" />
      ))}
    </span>
  );
}
