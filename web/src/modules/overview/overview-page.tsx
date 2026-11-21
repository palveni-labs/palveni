import { Button, Skeleton } from "@heroui/react";
import { ArrowDown, ArrowUp, ChevronRight, Eye, EyeOff, RotateCcw, SlidersHorizontal } from "lucide-react";
import { Suspense, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { useAccess } from "@/modules/access";
import { allWidgets, isEnabled } from "@/modules/registry";
import type { WidgetDefinition } from "@/modules/types";
import { useActiveSkin, useSlot } from "@/skins/scope";
import { PageHeader } from "@/ui/page";
import { Section } from "@/ui/section";
import { arrange, useOverviewLayout } from "./layout-store";
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

export function OverviewPage() {
  const { t } = useTranslation();
  const access = useAccess();
  const { skin, mode } = useActiveSkin();
  const pageTitle = useSlot("pageTitle");
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

  return (
    <div className="pv-enter grid min-h-full xl:grid-cols-[minmax(0,1fr)_var(--pv-aside-w)]">
      <div className="flex min-w-0 flex-col gap-(--pv-gap-section) px-(--pv-gutter) pt-7 pb-14 max-md:px-4 max-md:pt-5 xl:pe-(--pv-gap-section)">
        <PageHeader title={t("modules.overview.title")} summary={summary} actions={actions} />
        <Region widgets={main} editing={editing} onMove={moveWithin(main)} onHide={hide} />
      </div>
      {aside.length > 0 ? (
        <aside
          aria-label={t("overview.aside")}
          className={cn(
            "grid content-start gap-(--pv-gap-section) px-(--pv-gutter) pb-14 max-md:px-4 md:grid-cols-2 xl:grid-cols-1 xl:ps-0",
            pane ? "border-line bg-aside max-xl:border-t xl:border-s xl:px-5 xl:pt-7" : "xl:pt-7",
            !pane && pageTitle === "content" && "xl:mt-[calc(var(--pv-text-page)*1.2+var(--pv-gap-section))]",
          )}
        >
          <Region widgets={aside} editing={editing} onMove={moveWithin(aside)} onHide={hide} />
        </aside>
      ) : null}
    </div>
  );
}
