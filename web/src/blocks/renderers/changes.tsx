/**
 * The change preview and the confirmation request: the heart of "改动之前先让人看到改动".
 *
 * The preview lists every row that will change, before → after, each individually selectable,
 * grouped (identical copies / look-alikes kept), long groups folded behind a disclosure. The
 * request states what confirming does, in plain language, and confirms exactly the rows left
 * selected. Its look follows the `attention` slot; its legibility is guarded by SecureSurface.
 */
import { Button, Checkbox, Spinner, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { ArrowRight, Check, ChevronDown, Equal, X } from "lucide-react";
import { Fragment, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { formatBytes } from "@/lib/format";
import { SecureSurface, useSecureSurface } from "@/skins/secure";
import { useSlot } from "@/skins/scope";
import { Lamp, Measure, Seal } from "@/ui/marks";
import { Cover } from "@/ui/work";
import { BLOCK_ICONS } from "../icons";
import type { ApprovalBlock, ApprovalEffect, ChangePreviewBlock, ChangeRow } from "../types";

const FILE_NAME = /([\w\-() ]+\.(?:jpe?g|png|heic|webp|gif|mkv|mp4|avi|mov|flac|m4a|mp3|epub|pdf|cbz|srt))/gi;

/** Plain text with file names set as data (monospace). */
function WithPaths({ text }: { text: string }) {
  const parts = text.split(FILE_NAME);
  return (
    <>
      {parts.map((part, index) =>
        index % 2 === 1 ? (
          <span key={index} className="font-mono text-[0.92em]">
            {part}
          </span>
        ) : (
          <Fragment key={index}>{part}</Fragment>
        ),
      )}
    </>
  );
}

/** A path with its folder quiet and its file name readable. */
function PathText({ path, muted }: { path: string; muted: boolean }) {
  const slash = path.lastIndexOf("/");
  const dir = slash >= 0 ? path.slice(0, slash + 1) : "";
  const file = slash >= 0 ? path.slice(slash + 1) : path;
  return (
    <span className="min-w-0 truncate font-mono text-[0.8125rem] tracking-[-0.01em]" title={path}>
      <span className="text-fg-muted">{dir}</span>
      <span className={muted ? "text-fg-muted" : "text-fg"}>{file}</span>
    </span>
  );
}

function Thumbs({ row }: { row: ChangeRow }) {
  const [a, b] = row.thumbs ?? [];
  if (!a) return null;
  return (
    <span className="flex items-center gap-1.5" aria-hidden>
      <Cover artworkId={a} title="" decorative className="h-8 w-[2.75rem] rounded-[3px]" />
      {b ? (
        <>
          {row.relation === "similar" ? <span className="w-3.5 text-center text-sm text-fg-muted">≈</span> : <Equal className="size-3.5 text-fg-muted" />}
          <Cover artworkId={b} title="" decorative className="h-8 w-[2.75rem] rounded-[3px]" />
        </>
      ) : null}
    </span>
  );
}

/** "2.1 MB" as a measured value (number and unit set apart where the skin uses mono readings). */
function Size({ text }: { text: string }) {
  const match = /^([\d.,]+)\s*(.*)$/.exec(text);
  return match ? <Measure value={match[1]} unit={match[2] ? ` ${match[2]}` : undefined} /> : <>{text}</>;
}

function sumBytes(rows: ChangeRow[]): number {
  return rows.reduce((sum, row) => sum + (row.sizeBytes ?? 0), 0);
}

export function ChangePreviewView({
  block,
  selected,
  onToggle,
  onToggleAll,
  readOnly,
}: {
  block: ChangePreviewBlock;
  selected: ReadonlySet<string>;
  onToggle: (rowId: string) => void;
  onToggleAll: (all: boolean) => void;
  readOnly: boolean;
}) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const attention = useSlot("attention");
  const [filter, setFilter] = useState<string>("all");
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const secure = useSecureSurface();
  const groups = block.groups ?? [];
  const groupOf = (row: ChangeRow) => row.group ?? "";
  const ungrouped = block.rows.filter((row) => !groups.some((group) => group.id === groupOf(row)));
  const sections = [
    ...(ungrouped.length > 0 ? [{ id: "", label: "", collapseAfter: undefined as number | undefined, rows: ungrouped }] : []),
    ...groups.map((group) => ({ ...group, rows: block.rows.filter((row) => groupOf(row) === group.id) })),
  ].filter((section) => section.rows.length > 0 && (filter === "all" || section.id === filter));
  const all = block.rows.length > 0 && block.rows.every((row) => selected.has(row.id));
  const some = !all && block.rows.some((row) => selected.has(row.id));
  const count = block.rows.filter((row) => selected.has(row.id)).length;
  const hasThumbs = block.rows.some((row) => row.thumbs?.length);
  const hasSize = block.rows.some((row) => row.sizeBytes !== undefined);
  const columns = block.columns ?? {};
  const span = 2 + (hasThumbs ? 1 : 0) + 1 + (hasSize ? 1 : 0);

  return (
    <section
      {...(readOnly ? {} : secure)}
      id={`block-${block.id}`}
      aria-label={block.title}
      className="grid min-w-0 gap-2"
    >
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        {block.title ? <h3 className="font-heading text-md font-title text-heading">{block.title}</h3> : null}
        <span className="text-sm text-fg-muted" data-num>
          {t("blocks.selectedCount", { selected: count, total: block.rows.length })}
        </span>
        {groups.length > 1 ? (
          <ToggleButtonGroup
            aria-label={t("blocks.filter")}
            selectionMode="single"
            disallowEmptySelection
            selectedKeys={[filter]}
            onSelectionChange={(keys) => setFilter(String([...keys][0] ?? "all"))}
            className="pv-segmented ms-auto"
          >
            <ToggleButton id="all">
              {t("blocks.filterAll")}
              <span className="text-fg-muted tabular-nums">{block.rows.length}</span>
            </ToggleButton>
            {groups.map((group) => (
              <ToggleButton key={group.id} id={group.id}>
                {group.label}
                <span className="text-fg-muted tabular-nums">{block.rows.filter((row) => groupOf(row) === group.id).length}</span>
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        ) : null}
      </div>
      {block.summary ? <p className="text-sm text-fg-muted">{block.summary}</p> : null}
      {/* Column visibility follows the space the list is given (drawer, panel, result pane), not the window. */}
      <div className="@container overflow-x-auto">
        <table className="w-full min-w-[34rem] table-fixed border-collapse text-sm">
          <colgroup>
            <col className="w-9" />
            {hasThumbs ? <col className="w-[8.25rem]" /> : null}
            <col />
            <col className="w-[36%] @max-3xl:hidden" />
            {hasSize ? <col className="w-[5.5rem]" /> : null}
          </colgroup>
          <thead>
            <tr className="border-b border-line text-start text-xs text-fg-muted">
              <th scope="col" className="w-9 py-2 ps-1 text-start font-mid">
                {!readOnly ? (
                  <Checkbox isSelected={all} isIndeterminate={some} onChange={(value) => onToggleAll(value)} aria-label={t("blocks.selectAll")}>
                    <Checkbox.Content>
                      <Checkbox.Control>
                        <Checkbox.Indicator />
                      </Checkbox.Control>
                    </Checkbox.Content>
                  </Checkbox>
                ) : (
                  <span className="sr-only">{t("blocks.selected")}</span>
                )}
              </th>
              {hasThumbs ? (
                <th scope="col" className="w-28 py-2 text-start font-mid">
                  {columns.item ?? t("blocks.columns.item")}
                </th>
              ) : null}
              <th scope="col" className="py-2 text-start font-mid">
                {columns.change ?? t("blocks.columns.change")}
              </th>
              <th scope="col" className="py-2 text-start font-mid @max-3xl:hidden">
                {columns.reason ?? t("blocks.columns.reason")}
              </th>
              {hasSize ? (
                <th scope="col" className="py-2 pe-1 text-end font-mid">
                  {t("blocks.columns.size")}
                </th>
              ) : null}
            </tr>
          </thead>
          {sections.map((section, sectionIndex) => {
            const limit = section.collapseAfter;
            const folded = limit !== undefined && !expanded[section.id] && section.rows.length > limit + 1;
            const rows = folded ? section.rows.slice(0, limit) : section.rows;
            const hidden = folded ? section.rows.slice(limit) : [];
            return (
              <tbody key={section.id || "rows"}>
                {sectionIndex > 0 ? (
                  <tr>
                    <th scope="rowgroup" colSpan={span} className="pt-4 pb-1.5 ps-1 text-start text-xs font-mid text-fg-muted">
                      {section.label} · {section.rows.length}
                    </th>
                  </tr>
                ) : null}
                {rows.map((row) => {
                  const checked = selected.has(row.id);
                  const kept = row.group !== undefined && groups.findIndex((group) => group.id === row.group) > 0;
                  return (
                    <tr
                      key={row.id}
                      className={cn(
                        "border-b border-line align-middle transition-colors",
                        checked && attention === "tint" && "bg-attention-soft",
                        !checked && kept && attention !== "thumbs" && "bg-sunken/70",
                      )}
                    >
                      <td className="py-2.5 ps-1">
                        {!readOnly ? (
                          <Checkbox isSelected={checked} onChange={() => onToggle(row.id)} aria-label={row.before}>
                            <Checkbox.Content>
                              <Checkbox.Control>
                                <Checkbox.Indicator />
                              </Checkbox.Control>
                            </Checkbox.Content>
                          </Checkbox>
                        ) : checked ? (
                          <Check className="size-4 text-ok" aria-label={t("blocks.selected")} />
                        ) : (
                          <X className="size-4 text-fg-subtle" aria-label={t("blocks.notSelected")} />
                        )}
                      </td>
                      {hasThumbs ? (
                        <td className="py-2.5 pe-3">
                          <Thumbs row={row} />
                        </td>
                      ) : null}
                      <td className="py-2.5 pe-4">
                        <span className="flex min-w-0 items-center gap-2">
                          <PathText path={row.before} muted={!checked} />
                          {checked ? <ArrowRight className="size-3.5 shrink-0 text-fg-subtle" aria-hidden /> : null}
                          <span className={cn("shrink-0 whitespace-nowrap", checked ? "font-strong text-fg" : "text-fg-muted")}>{row.after}</span>
                        </span>
                        <span className="mt-0.5 block truncate text-xs text-fg-muted @3xl:hidden">
                          {row.note ? `${row.note} · ` : null}
                          {row.detail}
                        </span>
                      </td>
                      <td className="py-2.5 pe-4 text-fg-muted @max-3xl:hidden">
                        {row.note ? <span className="me-2 inline-flex rounded-chip border border-line-strong px-2 py-px text-xs font-mid text-fg">{row.note}</span> : null}
                        {row.detail ? (
                          <span className={row.note ? "mt-1 block" : undefined}>
                            <WithPaths text={row.detail} />
                          </span>
                        ) : null}
                      </td>
                      {hasSize ? (
                        <td className="py-2.5 pe-1 text-end font-strong whitespace-nowrap text-fg">{row.sizeBytes !== undefined ? <Size text={formatBytes(row.sizeBytes, locale)} /> : null}</td>
                      ) : null}
                    </tr>
                  );
                })}
                {hidden.length > 0 ? (
                  <tr className="border-b border-line">
                    <td colSpan={span} className="py-1.5">
                      <button
                        type="button"
                        onClick={() => setExpanded((current) => ({ ...current, [section.id]: true }))}
                        className="inline-flex items-center gap-1.5 rounded-control px-2 py-1.5 text-sm font-mid text-link hover:bg-hover"
                      >
                        <ChevronDown className="size-4" aria-hidden />
                        {t("blocks.showRest", { count: hidden.length, size: formatBytes(sumBytes(hidden), locale) })}
                      </button>
                    </td>
                  </tr>
                ) : null}
              </tbody>
            );
          })}
        </table>
      </div>
    </section>
  );
}

function effectOf(effect: string | ApprovalEffect): ApprovalEffect {
  return typeof effect === "string" ? { text: effect } : effect;
}

function Effect({ effect }: { effect: ApprovalEffect }) {
  const Icon = effect.icon ? BLOCK_ICONS[effect.icon] : null;
  return (
    <li className="flex items-center gap-2 text-sm text-fg-muted">
      {Icon ? <Icon className="size-4 shrink-0 text-fg-subtle" aria-hidden /> : <span className="size-1 shrink-0 rounded-full bg-current" aria-hidden />}
      <span>
        {effect.text}
        {effect.path ? <span className="ms-1.5 font-mono text-[0.8125rem] text-fg">{effect.path}</span> : null}
      </span>
    </li>
  );
}

function RequestMark() {
  const attention = useSlot("attention");
  if (attention === "seal") return <Seal size={18} className="mt-1" />;
  if (attention === "lamp") return <Lamp tone="attention" className="mt-[0.55em] size-2.5" />;
  return null;
}

export function ApprovalView({
  block,
  selectedCount,
  onConfirm,
  onEdit,
  onCancel,
  busy,
  pending,
  pinned = false,
  className,
}: {
  block: ApprovalBlock;
  /** Rows selected in the linked change preview, if any. */
  selectedCount: number | null;
  onConfirm: () => void;
  onEdit: () => void;
  onCancel: () => void;
  busy: boolean;
  /** The host is carrying out a decision. */
  pending?: boolean;
  /** Kept in view at the bottom of the scrolling result. */
  pinned?: boolean;
  className?: string;
}) {
  const { t } = useTranslation();
  const attention = useSlot("attention");
  const status = block.status ?? "pending";
  const title = block.title.replace("{count}", String(selectedCount ?? ""));
  const effects = block.effects.map(effectOf);
  const ConfirmIcon = block.confirmIcon ? BLOCK_ICONS[block.confirmIcon] : null;
  const confirmText: ReactNode = (
    <span className="tabular-nums">
      {block.confirmLabel ?? t("blocks.approval.confirm")}
      {selectedCount !== null ? t("blocks.approval.count", { count: selectedCount }) : null}
    </span>
  );
  return (
    <SecureSurface
      role="group"
      aria-label={title}
      className={cn(
        "grid gap-x-5 gap-y-3 px-(--pv-pad-panel) py-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end",
        attention === "tint" && "border-t border-attention-line bg-attention-soft",
        attention === "thumbs" && "border-t border-line bg-sunken",
        attention === "seal" && "border-t border-line-strong bg-page",
        attention === "lamp" && "rounded-panel border border-line bg-raised shadow-raised",
        pinned && "sticky z-10 mt-2",
        pinned && (attention === "lamp" ? "bottom-3 mb-3" : "bottom-0 -mx-(--pv-pad-panel)"),
        className,
      )}
    >
      <div className="flex min-w-0 items-start gap-3">
        {status === "pending" ? <RequestMark /> : null}
        <div className="grid min-w-0 gap-1.5">
          <h3 className={cn("font-heading font-title text-heading", attention === "seal" ? "text-xl" : "text-lg")}>{title}</h3>
          <ul className="grid gap-x-5 gap-y-1 sm:flex sm:flex-wrap">
            {effects.map((effect, index) => (
              <Effect key={index} effect={effect} />
            ))}
          </ul>
        </div>
      </div>
      {status === "pending" ? (
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button variant="ghost" onPress={onCancel} isDisabled={busy}>
            {t("blocks.approval.cancel")}
          </Button>
          {block.previewId ? (
            <Button variant="outline" onPress={onEdit} isDisabled={busy}>
              {t("blocks.approval.edit")}
            </Button>
          ) : null}
          <Button
            size="lg"
            variant={block.risk === "high" ? "danger" : "primary"}
            className={cn(block.risk !== "high" && "button--confirm")}
            onPress={onConfirm}
            isDisabled={busy || selectedCount === 0}
            isPending={pending}
          >
            {pending ? <Spinner size="sm" color="current" /> : ConfirmIcon ? <ConfirmIcon aria-hidden /> : null}
            {confirmText}
          </Button>
        </div>
      ) : (
        <p className={cn("flex items-center gap-1.5 text-sm font-mid", status === "approved" ? "text-ok" : "text-fg-muted")}>
          {status === "approved" ? <Check className="size-4" aria-hidden /> : <X className="size-4" aria-hidden />}
          {status === "approved" ? t("blocks.approval.approved") : t("blocks.approval.cancelled")}
        </p>
      )}
    </SecureSurface>
  );
}
