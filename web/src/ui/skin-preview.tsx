/**
 * Live previews of a skin. A miniature of the workspace is drawn by the host's own components
 * inside <SkinScope>, so the skin's tokens and slot choices show exactly as they would apply:
 * the navigation in the skin's `nav` form, a section separated per `section`, a row that waits
 * for the user marked per `attention`, and a running row whose progress is drawn per `progress`.
 *
 * The miniature is rendered at a fixed workspace size and scaled to fit its frame. It is a
 * picture (aria-hidden, inert): the skin's name and actions sit next to it.
 */
import { buttonVariants } from "@heroui/react";
import { ChevronDown, FolderOpen, LayoutDashboard, ListChecks, Palette, Settings2 } from "lucide-react";
import { useLayoutEffect, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { SkinEntry } from "@/skins/registry";
import { MODES, type Mode } from "@/skins/schema";
import { SkinScope, useSlot } from "@/skins/scope";
import { AttentionMark, Lamp, Measure } from "@/ui/marks";
import { ProgressBar, RunProgress } from "@/ui/progress";
import { Section, useRowInset, useSectionVariant } from "@/ui/section";
import { Cover } from "@/ui/work";

/** The workspace size the miniature is laid out at, before scaling. */
const BASE_W = 680;
const BASE_H = 380;
const PROGRESS = 0.42;
/** Placeholder covers for the `thumbs` attention mark (no real work is shown in a preview). */
const BLANK_COVERS: Array<string | null> = Array.from({ length: 8 }, () => null);

function useFitScale(ref: RefObject<HTMLElement | null>, base: number): number {
  const [scale, setScale] = useState(0);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    const update = () => setScale(element.clientWidth / base);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref, base]);
  return scale;
}

/* ---------------------------------------------------------------- the miniature */

function useNavItems() {
  const { t } = useTranslation();
  return [
    { label: t("modules.overview.title"), icon: LayoutDashboard },
    { label: t("modules.tasks.title"), icon: ListChecks },
    { label: t("modules.library.title"), icon: FolderOpen },
  ];
}

function MiniTopBar() {
  const { t } = useTranslation();
  const items = useNavItems();
  return (
    <div className="flex h-(--pv-shell-h) shrink-0 items-center justify-between gap-4 border-b border-shell-line bg-shell px-5 text-shell-fg">
      <span className="font-heading text-[1.125rem] leading-none font-title tracking-[-0.015em]">{t("app.name")}</span>
      <ul className="flex items-center gap-0.5 rounded-control bg-nav-track p-[3px]">
        {items.map((item, index) => (
          <li
            key={item.label}
            className={cn(
              "relative flex h-(--pv-control-h) items-center gap-1 rounded-control px-3 text-sm whitespace-nowrap",
              index === 0 ? "bg-nav-active font-strong text-nav-active-fg" : "font-mid text-shell-fg-muted",
            )}
          >
            {item.label}
            {index === 2 ? <ChevronDown className="size-3 opacity-65" /> : null}
            {index === 0 ? (
              <span className="absolute inset-x-2.5 h-0.5 bg-nav-indicator" style={{ bottom: "calc((var(--pv-control-h) + 6px - var(--pv-shell-h)) / 2)" }} />
            ) : null}
          </li>
        ))}
        <li className="flex h-(--pv-control-h) items-center gap-1 rounded-control px-3 text-sm font-mid whitespace-nowrap text-shell-fg-muted">
          {t("modules.system.title")}
          <ChevronDown className="size-3 opacity-65" />
        </li>
      </ul>
      <span className="size-6 rounded-full bg-shell-field" />
    </div>
  );
}

function MiniSidebar() {
  const { t } = useTranslation();
  const items = useNavItems();
  return (
    <div className="flex w-[12.5rem] shrink-0 flex-col border-e border-shell-line bg-shell px-2.5 text-shell-fg">
      <span className="flex h-(--pv-shell-h) items-center px-2 font-heading text-[1.125rem] leading-none font-title text-heading">{t("app.name")}</span>
      <ul className="grid gap-0.5">
        {items.map(({ label, icon: Icon }, index) => (
          <li
            key={label}
            className={cn("flex h-8 items-center gap-2 rounded-tile px-2.5 text-sm", index === 0 ? "bg-nav-active font-strong text-nav-active-fg shadow-panel" : "font-mid text-shell-fg")}
          >
            <Icon className={cn("size-3.5 shrink-0", index === 0 ? "text-nav-active-fg" : "text-icon")} />
            {label}
          </li>
        ))}
      </ul>
      <div className="mt-4 grid gap-0.5">
        <span className="px-2.5 pb-0.5 text-[0.6875rem] font-mid text-shell-fg-muted">{t("nav.manage")}</span>
        <div className="flex h-7 items-center gap-2 rounded-tile px-2.5 text-sm font-mid text-shell-fg">
          <Settings2 className="size-3.5 shrink-0 text-icon" />
          <span className="flex-1">{t("modules.system.title")}</span>
          <ChevronDown className="size-3 text-shell-fg-muted" />
        </div>
        <div className="ms-4 grid gap-0.5 border-s border-shell-line ps-2 text-xs text-shell-fg-muted">
          <span className="py-0.5">{t("nav.manageLibraries")}</span>
          <span className="py-0.5">{t("nav.settings")}</span>
        </div>
      </div>
    </div>
  );
}

function WaitingRow() {
  const { t } = useTranslation();
  const attention = useSlot("attention");
  const inset = useRowInset();
  const variant = useSectionVariant();
  const tint = attention === "tint";
  return (
    <li
      className={cn(
        "flex items-center gap-3.5 py-3",
        tint ? "rounded-tile border border-attention-line bg-attention-soft px-4" : inset,
        variant === "plain" && !tint && "rounded-tile",
      )}
    >
      <AttentionMark previews={BLANK_COVERS} title="" />
      <div className="grid min-w-0 flex-1 gap-0.5">
        <span className="truncate text-md font-mid text-fg">{t("appearance.preview.waitingTitle")}</span>
        <span className="text-sm font-mid text-attention">{t("attention.waiting")}</span>
      </div>
      <span className={cn(buttonVariants({ variant: "primary", size: "sm" }), "button--confirm shrink-0")}>{t("attention.confirm")}</span>
    </li>
  );
}

function RunningRow() {
  const { t } = useTranslation();
  const progress = useSlot("progress");
  const itemColor = useSlot("itemColor");
  const attention = useSlot("attention");
  const inset = useRowInset();
  const kind = t("appearance.preview.runningKind");
  const subject = t("appearance.preview.runningSubject");
  const lead =
    itemColor === "on" ? (
      <Cover artworkId={null} title="" decorative icon={FolderOpen} className="h-[3.375rem] w-9 shrink-0 rounded-[3px] shadow-art" />
    ) : (
      <span className="flex w-[4.5rem] shrink-0 items-center gap-2.5 text-sm text-fg-muted">
        {attention === "lamp" ? <Lamp tone="active" /> : null}
        {kind}
      </span>
    );
  return (
    <li>
      <RunProgress value={PROGRESS} label={`${kind} ${subject}`}>
        <div className={cn("flex min-h-(--pv-row-h) items-center gap-3.5 py-3", inset)}>
          {lead}
          <div className="grid min-w-0 flex-1 gap-0.5">
            <span className="truncate text-md font-strong text-fg">
              {itemColor === "on" ? <span className="me-2 font-mid text-fg-muted">{kind}</span> : null}
              {subject}
            </span>
            <span className="truncate text-sm text-fg-muted">{t("appearance.preview.runningDetail")}</span>
          </div>
          {progress === "bar" ? <ProgressBar value={PROGRESS} label={subject} className="w-24 shrink-0" /> : null}
          <div className="shrink-0 text-end">
            <Measure value="42%" className="block text-md font-title text-fg" />
            <span className="block text-sm text-fg-muted">{t("appearance.preview.runningLeft")}</span>
          </div>
        </div>
      </RunProgress>
    </li>
  );
}

function Miniature() {
  const { t } = useTranslation();
  const nav = useSlot("nav");
  const pageTitle = useSlot("pageTitle");
  const attention = useSlot("attention");
  const section = useSlot("section");
  const title = t("modules.overview.title");
  return (
    <div className="flex size-full bg-page font-ui text-base text-fg">
      {nav === "sidebar" ? <MiniSidebar /> : null}
      <div className="flex min-w-0 flex-1 flex-col">
        {nav === "topbar" ? (
          <MiniTopBar />
        ) : (
          <div className="flex h-(--pv-shell-h) shrink-0 items-center border-b border-line bg-page px-5 text-[1.0625rem] font-strong text-heading">
            {pageTitle === "toolbar" ? title : null}
          </div>
        )}
        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden px-(--pv-gutter) pt-5">
          {pageTitle === "content" ? <span className="font-heading text-headline leading-none font-title text-heading">{title}</span> : null}
          <Section title={t("appearance.preview.section")} count={2}>
            <ul className={cn(attention === "tint" ? "grid gap-2" : "divide-y divide-line", section === "panel" && attention !== "tint" && "border-t border-line")}>
              <WaitingRow />
              <RunningRow />
            </ul>
          </Section>
        </div>
      </div>
    </div>
  );
}

/* ---------------------------------------------------------------- frames */

/** One live preview of `entry` in `mode`, scaled to the frame's width. */
export function SkinPreview({ entry, mode, className }: { entry: SkinEntry; mode: Mode; className?: string }) {
  const frame = useRef<HTMLDivElement>(null);
  const scale = useFitScale(frame, BASE_W);
  return (
    <div
      ref={frame}
      aria-hidden
      inert
      data-skin-preview={mode}
      className={cn("relative overflow-hidden rounded-tile border border-line bg-sunken", className)}
      style={{ aspectRatio: `${BASE_W} / ${BASE_H}` }}
    >
      <div className="absolute start-0 top-0 origin-top-left" style={{ width: BASE_W, height: BASE_H, transform: `scale(${scale})` }}>
        <SkinScope entry={entry} mode={mode} className="size-full">
          <Miniature />
        </SkinScope>
      </div>
    </div>
  );
}

/** Light and dark side by side, each captioned. */
export function SkinPreviewPair({ entry, className }: { entry: SkinEntry; className?: string }) {
  const { t } = useTranslation();
  return (
    <div className={cn("grid grid-cols-2 gap-2", className)}>
      {MODES.map((mode) => (
        <figure key={mode} className="grid min-w-0 gap-1.5">
          <SkinPreview entry={entry} mode={mode} />
          <figcaption className="text-xs text-fg-muted">{t(`theme.${mode}`)}</figcaption>
        </figure>
      ))}
    </div>
  );
}

/**
 * Stands in for a pair of previews when the skin cannot be drawn live (not installed yet, or its
 * manifest did not load): the package's preview image, or a calm note.
 */
export function PreviewStandIn({ src, alt, note, icon: Icon = Palette, className }: { src?: string | null; alt: string; note: string; icon?: typeof Palette; className?: string }) {
  const style = { aspectRatio: `${BASE_W * 2 + 12} / ${BASE_H}` };
  if (src) {
    return <img src={src} alt={alt} loading="lazy" draggable={false} className={cn("block w-full rounded-tile border border-line bg-sunken object-cover object-top", className)} style={style} />;
  }
  return (
    <div className={cn("grid place-items-center rounded-tile border border-dashed border-line-strong bg-sunken px-4 text-center", className)} style={style}>
      <span className="flex flex-col items-center gap-2 text-sm text-fg-muted">
        <Icon className="size-5" strokeWidth={1.7} aria-hidden />
        {note}
      </span>
    </div>
  );
}
