import { Button, Skeleton } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ArrowDown, ArrowUp, ChevronRight, Eye, EyeOff, Film, House, Layers, Music, RotateCcw, SlidersHorizontal, type LucideIcon } from "lucide-react";
import { Suspense, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import { formatNumber } from "@/lib/format";
import { useAccess, useFeature } from "@/modules/access";
import { allWidgets, isEnabled } from "@/modules/registry";
import type { AppPath, WidgetDefinition } from "@/modules/types";
import { useHome } from "@/provisional/queries/home";
import { useActiveSkin, useSlot } from "@/skins/scope";
import { PageHeader } from "@/ui/page";
import { Section } from "@/ui/section";
import { arrange, useOverviewLayout } from "./layout-store";
import { PluginWidgetsRow } from "./plugin-widgets";
import { useOverviewSummary } from "./summary";

const useNoCount = (): number | undefined => undefined;

function WidgetSection({
  widget,
  editing,
  first,
  last,
  onMove,
  onHide,
}: {
  widget: WidgetDefinition;
  editing: boolean;
  first: boolean;
  last: boolean;
  onMove: (delta: -1 | 1) => void;
  onHide: () => void;
}) {
  const { t } = useTranslation();
  const attention = useSlot("attention");
  const useCount = widget.count ?? useNoCount;
  const count = useCount();
  const Body = widget.component;
  const isAdmin = useIsAdmin();
  const more = widget.more && (!widget.more.adminOnly || isAdmin) ? { label: t(widget.more.labelKey), to: widget.more.to, search: widget.more.search } : undefined;
  const editTools = editing ? (
    <div className="flex items-center gap-0.5">
      <Button size="sm" variant="ghost" isIconOnly isDisabled={first} onPress={() => onMove(-1)} aria-label={t("overview.moveEarlier")}>
        <ArrowUp aria-hidden />
      </Button>
      <Button size="sm" variant="ghost" isIconOnly isDisabled={last} onPress={() => onMove(1)} aria-label={t("overview.moveLater")}>
        <ArrowDown aria-hidden />
      </Button>
      <Button size="sm" variant="ghost" isIconOnly onPress={onHide} aria-label={t("overview.hide")}>
        <EyeOff aria-hidden />
      </Button>
    </div>
  ) : widget.moreAction ? (
    <button type="button" onClick={widget.moreAction.run} className="-my-1 inline-flex items-center gap-0.5 rounded-control py-1 ps-2 pe-1 text-sm text-link outline-none hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus">
      {t(widget.moreAction.labelKey)}
      <ChevronRight className="size-3.5" aria-hidden />
    </button>
  ) : null;
  return (
    <Section
      data-widget={widget.id}
      title={t(widget.titleKey)}
      count={count}
      more={editing ? undefined : more}
      actions={editTools}
      raised={widget.needsYou === true && attention === "lamp"}
      unframed={widget.tiles === true}
      className={cn(editing && "outline-2 outline-offset-4 outline-line-strong outline-dashed")}
    >
      <div className={cn(editing && "pointer-events-none opacity-60")}>
        <Suspense fallback={<Skeleton className="m-4 h-24 rounded-tile" />}>
          <Body />
        </Suspense>
      </div>
    </Section>
  );
}

function Region({ widgets, editing, onMove, onHide }: { widgets: WidgetDefinition[]; editing: boolean; onMove: (id: string, delta: -1 | 1) => void; onHide: (id: string) => void }) {
  return (
    <>
      {widgets.map((widget, index) => (
        <WidgetSection
          key={widget.id}
          widget={widget}
          editing={editing}
          first={index === 0}
          last={index === widgets.length - 1}
          onMove={(delta) => onMove(widget.id, delta)}
          onHide={() => onHide(widget.id)}
        />
      ))}
    </>
  );
}

function KpiTile({
  to,
  icon: Icon,
  label,
  value,
  sub,
  glowClass,
  iconClass,
}: {
  to: AppPath;
  icon: LucideIcon;
  label: string;
  value: string;
  sub?: string;
  glowClass: string;
  iconClass: string;
}) {
  return (
    <Link
      to={to}
      className={cn(
        "pv-glass-panel group relative flex items-center gap-3.5 overflow-hidden rounded-panel border px-4 py-3.5 outline-none transition-all duration-200 hover:-translate-y-0.5 hover:border-line-strong/80 hover:shadow-raised focus-visible:ring-2 focus-visible:ring-focus",
        glowClass,
      )}
    >
      <span className={cn("grid size-11 shrink-0 place-items-center rounded-xl shadow-panel transition-transform duration-200 group-hover:scale-105", iconClass)}>
        <Icon className="size-5" strokeWidth={1.9} aria-hidden />
      </span>
      <div className="grid min-w-0 flex-1 gap-0.5">
        <span className="truncate text-xs font-mid text-fg-muted">{label}</span>
        <span data-num className="truncate font-heading text-2xl leading-tight font-title tracking-tight text-heading tabular-nums">
          {value}
        </span>
        {sub ? <span className="truncate text-xs text-fg-subtle">{sub}</span> : null}
      </div>
      <ChevronRight className="size-4 shrink-0 text-fg-subtle opacity-0 transition-all duration-150 group-hover:translate-x-0.5 group-hover:opacity-100" aria-hidden />
    </Link>
  );
}

function OverviewMetricsStrip() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "zh-CN";
  const hasHome = useFeature("home");
  const { data: libData } = useLibraries();
  const { data: home } = useHome(hasHome);
  const libraries = libData?.libraries ?? [];
  if (libraries.length === 0 && !home) return null;

  const videoCount = libraries.filter((lib) => lib.kind === "video").reduce((sum, lib) => sum + lib.itemCount, 0);
  const audioCount = libraries
    .filter((lib) => lib.kind === "music" || lib.kind === "audiobook" || lib.kind === "podcast")
    .reduce((sum, lib) => sum + lib.itemCount, 0);
  const kindsCount = new Set(libraries.map((lib) => lib.kind)).size;
  const totalItems = libraries.reduce((sum, lib) => sum + lib.itemCount, 0);
  const devicesOn =
    home?.devices.filter((device) =>
      device.state.type === "sensor"
        ? false
        : device.state.type === "media_player"
          ? device.state.status === "playing"
          : device.state.on,
    ).length ?? 0;
  const activeScene = home?.scenes.find((scene) => scene.active)?.name;

  return (
    <div className="grid grid-cols-2 gap-3.5 xl:grid-cols-4">
      <KpiTile
        to="/video"
        icon={Film}
        label={t("kinds.video")}
        value={formatNumber(videoCount, locale)}
        sub={`${formatNumber(libraries.filter((lib) => lib.kind === "video").length, locale)} ${t("overview.stats.libraries")}`}
        glowClass="pv-glow-blue"
        iconClass="bg-[oklch(0.54_0.16_250/0.18)] text-[oklch(0.52_0.16_250)] dark:bg-[oklch(0.56_0.16_250/0.24)] dark:text-[oklch(0.78_0.14_248)]"
      />
      <KpiTile
        to="/music"
        icon={Music}
        label={t("musicApp.title")}
        value={formatNumber(audioCount, locale)}
        sub={`${formatNumber(libraries.filter((lib) => lib.kind === "music" || lib.kind === "audiobook" || lib.kind === "podcast").length, locale)} ${t("overview.stats.libraries")}`}
        glowClass="pv-glow-sky"
        iconClass="bg-[oklch(0.56_0.13_212/0.18)] text-[oklch(0.48_0.13_212)] dark:bg-[oklch(0.62_0.13_210/0.24)] dark:text-[oklch(0.80_0.12_208)]"
      />
      <KpiTile
        to="/library"
        icon={Layers}
        label={t("overview.stats.libraries")}
        value={formatNumber(libraries.length, locale)}
        sub={t("overview.stats.kindsInUse", { count: kindsCount })}
        glowClass="pv-glow-emerald"
        iconClass="bg-[oklch(0.54_0.13_156/0.16)] text-[oklch(0.46_0.13_156)] dark:bg-[oklch(0.66_0.14_156/0.22)] dark:text-[oklch(0.80_0.13_156)]"
      />
      {home ? (
        <KpiTile
          to="/home"
          icon={House}
          label={t("modules.home.title")}
          value={`${devicesOn} / ${home.devices.length}`}
          sub={activeScene}
          glowClass="pv-glow-amber"
          iconClass="bg-[oklch(0.64_0.15_70/0.18)] text-[oklch(0.52_0.15_64)] dark:bg-[oklch(0.76_0.16_74/0.22)] dark:text-[oklch(0.85_0.14_76)]"
        />
      ) : (
        <KpiTile
          to="/library"
          icon={Layers}
          label={t("overview.stats.items")}
          value={formatNumber(totalItems, locale)}
          sub={t("overview.stats.itemsHint")}
          glowClass="pv-glow-amber"
          iconClass="bg-[oklch(0.64_0.15_70/0.18)] text-[oklch(0.52_0.15_64)] dark:bg-[oklch(0.76_0.16_74/0.22)] dark:text-[oklch(0.85_0.14_76)]"
        />
      )}
    </div>
  );
}

/**
 * 概览. The order is fixed by "what needs me first": the main column holds the
 * work (needs you → running → libraries → recently added), the side column the state of the
 * house (server → home → assistant suggestions). Modules contribute the sections; the user may
 * hide them or move them within their column.
 */
export function OverviewPage() {
  const { t } = useTranslation();
  const access = useAccess();
  const { skin, mode } = useActiveSkin();
  const [editing, setEditing] = useState(false);
  const { layout, move, hide, show, reset } = useOverviewLayout();
  const summary = useOverviewSummary();
  const available = allWidgets()
    .filter((widget) => isEnabled(widget, access))
    .sort((a, b) => a.rank - b.rank);
  const { shown, hidden } = arrange(available, layout);
  const main = shown.filter((widget) => widget.region === "main");
  const aside = shown.filter((widget) => widget.region === "aside");
  const colors = skin.colors[mode];
  // A side column with its own ground reads as a separate pane (an inspector).
  const pane = colors.aside !== colors.bg;
  const moveWithin = (region: WidgetDefinition[]) => (id: string, delta: -1 | 1) => move(region.map((widget) => widget.id), id, delta);

  const actions: ReactNode = (
    <div className="flex flex-wrap items-center gap-1.5">
      {editing && hidden.length > 0
        ? hidden.map((widget) => (
            <Button key={widget.id} size="sm" variant="secondary" onPress={() => show(widget.id)}>
              <Eye aria-hidden />
              {t(widget.titleKey)}
            </Button>
          ))
        : null}
      {editing ? (
        <Button size="sm" variant="ghost" onPress={reset}>
          <RotateCcw aria-hidden />
          {t("overview.resetLayout")}
        </Button>
      ) : null}
      <Button size="sm" variant={editing ? "primary" : "ghost"} onPress={() => setEditing((value) => !value)} aria-pressed={editing}>
        <SlidersHorizontal aria-hidden />
        {editing ? t("overview.doneEditing") : t("overview.editLayout")}
      </Button>
    </div>
  );

  if (!pane) {
    return (
      <div className="pv-enter mx-auto flex min-h-full w-full max-w-[77.5rem] flex-col gap-(--pv-gap-section) px-(--pv-gutter) pt-6 pb-14 max-md:px-4 max-md:pt-5">
        <PageHeader title={t("modules.overview.title")} summary={summary} actions={actions} />
        <OverviewMetricsStrip />
        <PluginWidgetsRow />
        <div className="grid min-w-0 items-start gap-(--pv-gap-section) xl:grid-cols-[minmax(0,1fr)_var(--pv-aside-w)]">
          <div className="flex min-w-0 flex-col gap-(--pv-gap-section)">
            <Region widgets={main} editing={editing} onMove={moveWithin(main)} onHide={hide} />
          </div>
          {aside.length > 0 ? (
            <aside aria-label={t("overview.aside")} className="grid content-start gap-(--pv-gap-section) md:grid-cols-2 xl:grid-cols-1">
              <Region widgets={aside} editing={editing} onMove={moveWithin(aside)} onHide={hide} />
            </aside>
          ) : null}
        </div>
      </div>
    );
  }

  return (
    <div className="pv-enter grid min-h-full w-full xl:grid-cols-[minmax(0,1fr)_var(--pv-aside-w)]">
      <div className="mx-auto flex w-full max-w-[60rem] min-w-0 flex-col gap-(--pv-gap-section) px-(--pv-gutter) pt-7 pb-14 max-md:px-4 max-md:pt-5 xl:pe-(--pv-gap-section)">
        <PageHeader title={t("modules.overview.title")} summary={summary} actions={actions} />
        <PluginWidgetsRow />
        <Region widgets={main} editing={editing} onMove={moveWithin(main)} onHide={hide} />
      </div>
      {aside.length > 0 ? (
        <aside
          aria-label={t("overview.aside")}
          className="grid content-start gap-(--pv-gap-section) border-line bg-aside px-(--pv-gutter) pb-14 max-xl:border-t max-md:px-4 md:grid-cols-2 xl:grid-cols-1 xl:border-s xl:px-5 xl:pt-7"
        >
          <Region widgets={aside} editing={editing} onMove={moveWithin(aside)} onHide={hide} />
        </aside>
      ) : null}
    </div>
  );
}
