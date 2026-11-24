/**
 * Plain blocks: text, a collection of works, a table and a progress reading. Every value goes
 * through the skin's tokens; tones are status colours with an icon, never colour alone.
 */
import { AlertTriangle, CheckCircle2, XCircle, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { formatPercent } from "@/lib/format";
import { Measure, StatusLabel } from "@/ui/marks";
import { ProgressBar } from "@/ui/progress";
import { Cover } from "@/ui/work";
import { formatCell, isNumericFormat } from "../format";
import { BLOCK_ICONS } from "../icons";
import type { BlockTone, CollectionBlock, CollectionItem, ProgressBlock, TableBlock, TextBlock } from "../types";

/** The heading every titled block uses. */
export function BlockTitle({ children, className }: { children: ReactNode; className?: string }) {
  return <h3 className={cn("font-heading text-md font-title text-heading", className)}>{children}</h3>;
}

const TONES: Record<BlockTone, { text: string; icon: LucideIcon | null }> = {
  default: { text: "text-fg", icon: null },
  muted: { text: "text-fg-muted", icon: null },
  success: { text: "text-ok", icon: CheckCircle2 },
  warning: { text: "text-degraded", icon: AlertTriangle },
  danger: { text: "text-failed", icon: XCircle },
};

export function TextBlockView({ block }: { block: TextBlock }) {
  const tone = TONES[block.tone ?? "default"];
  const Icon = tone.icon;
  return (
    <div className="grid gap-1.5">
      {block.title ? <BlockTitle>{block.title}</BlockTitle> : null}
      <p className={cn("flex items-start gap-2 text-base whitespace-pre-line", tone.text, Icon && "font-mid")}>
        {Icon ? <Icon className="mt-[0.2em] size-[1.1em] shrink-0" aria-hidden /> : null}
        <span className="min-w-0">{block.text}</span>
      </p>
    </div>
  );
}

/* ---------------------------------------------------------------- collection */

const TALL: ReadonlySet<CollectionItem["kind"]> = new Set(["video", "book", "comic"]);

function ItemCover({ item }: { item: CollectionItem }) {
  const artworkId = item.artworkId || null;
  const icon = BLOCK_ICONS[item.kind];
  const shape = cn("w-full rounded-tile", TALL.has(item.kind) ? "aspect-[2/3]" : "aspect-square");
  if (artworkId) {
    return <Cover artworkId={artworkId} title={item.title} icon={icon} decorative className={cn(shape, "shadow-art")} />;
  }
  const Icon = icon;
  return (
    <span aria-hidden className={cn(shape, "grid place-items-center bg-sunken text-fg-subtle")}>
      <Icon className="size-6" strokeWidth={1.6} />
    </span>
  );
}

export function CollectionBlockView({ block, onOpen }: { block: CollectionBlock; onOpen?: (id: string) => void }) {
  const { t } = useTranslation();
  const row = block.layout === "row";
  return (
    <div className="grid min-w-0 gap-3">
      {block.title ? <BlockTitle>{block.title}</BlockTitle> : null}
      <ul className={cn(row ? "no-scrollbar -mx-1 flex gap-3 overflow-x-auto px-1 pb-1" : "grid grid-cols-[repeat(auto-fill,minmax(7rem,1fr))] gap-x-3 gap-y-4")}>
        {block.items.map((item) => (
          <li key={item.id} className={cn("min-w-0", row && "w-28 shrink-0")}>
            <button
              type="button"
              onClick={() => onOpen?.(item.id)}
              className="group grid w-full min-w-0 content-start gap-2 rounded-tile text-start"
            >
              <span className="block transition-transform duration-200 ease-skin group-hover:-translate-y-0.5">
                <ItemCover item={item} />
              </span>
              <span className="grid min-w-0 gap-0.5">
                <span className="truncate text-sm font-mid text-fg group-hover:underline">{item.title}</span>
                {item.subtitle || item.badge ? (
                  <span className="truncate text-xs text-fg-muted">
                    {[item.subtitle, item.badge].filter(Boolean).join(" · ")}
                  </span>
                ) : null}
              </span>
            </button>
          </li>
        ))}
      </ul>
      {block.more ? <p className="text-sm text-fg-muted">{t("blocks.more", { count: block.more })}</p> : null}
    </div>
  );
}

/* ---------------------------------------------------------------- table */

export function TableBlockView({ block }: { block: TableBlock }) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const alignEnd = (column: TableBlock["columns"][number]) => (column.align ? column.align === "end" : isNumericFormat(column.format));
  return (
    <div className="grid min-w-0 gap-2">
      {block.title ? <BlockTitle>{block.title}</BlockTitle> : null}
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-line-strong">
              {block.columns.map((column) => (
                <th
                  key={column.key}
                  scope="col"
                  className={cn("py-2 pe-4 text-xs font-mid whitespace-nowrap text-fg-muted last:pe-0", alignEnd(column) ? "text-end" : "text-start")}
                >
                  {column.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {block.rows.length === 0 ? (
              <tr>
                <td colSpan={block.columns.length} className="py-4 text-center text-fg-muted">
                  {t("blocks.tableEmpty")}
                </td>
              </tr>
            ) : (
              block.rows.map((row, index) => (
                <tr key={index} className="border-b border-line last:border-b-0">
                  {block.columns.map((column) => (
                    <td
                      key={column.key}
                      className={cn("py-2.5 pe-4 align-top text-fg tabular-nums last:pe-0", alignEnd(column) ? "text-end whitespace-nowrap" : "text-start")}
                    >
                      {formatCell(row[column.key] ?? null, column.format, locale)}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/* ---------------------------------------------------------------- progress */

export function ProgressBlockView({ block }: { block: ProgressBlock }) {
  const { t, i18n } = useTranslation();
  const state = block.state ?? "running";
  const reading =
    state === "succeeded" ? (
      <StatusLabel tone="ok" className="text-sm">
        {t("blocks.progress.succeeded")}
      </StatusLabel>
    ) : state === "failed" ? (
      <StatusLabel tone="failed" className="text-sm">
        {t("blocks.progress.failed")}
      </StatusLabel>
    ) : block.value !== null ? (
      <Measure value={formatPercent(block.value, i18n.resolvedLanguage ?? "en")} className="text-sm font-strong text-fg" />
    ) : (
      <span className="text-sm text-fg-muted">{t("blocks.progress.unknown")}</span>
    );
  return (
    <div className="grid gap-2">
      {block.title ? <BlockTitle>{block.title}</BlockTitle> : null}
      <div className="flex items-baseline justify-between gap-3">
        <span className="min-w-0 truncate text-base font-mid text-fg">{block.label}</span>
        {reading}
      </div>
      <ProgressBar
        value={state === "succeeded" ? 1 : block.value}
        label={block.label}
        tone={state === "failed" ? "failed" : state === "succeeded" ? "neutral" : "active"}
      />
      {block.detail ? <p className="text-sm text-fg-muted">{block.detail}</p> : null}
    </div>
  );
}
