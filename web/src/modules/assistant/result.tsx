import { buttonVariants, Button, Chip } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ArrowDownToLine, LayoutPanelTop } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { BlockRenderer } from "@/blocks/block-renderer";
import type { Block, BlockAction } from "@/blocks/types";
import type { AssistantRun, RunState } from "@/provisional/types";
import { useSlot } from "@/skins/scope";
import { Lamp, Seal, StatusLabel, type StatusTone } from "@/ui/marks";
import { SummaryFacts, type Fact } from "@/ui/page";
import { EmptyState } from "@/ui/states";
import { Cover, useWorkColor } from "@/ui/work";
import { isRunActive } from "./use-assistant";

const RUN_TONE: Record<RunState, StatusTone> = {
  running: "active",
  awaiting_approval: "attention",
  succeeded: "ok",
  failed: "failed",
  cancelled: "idle",
};

function RunStatus({ state, onField }: { state: RunState; onField: boolean }) {
  const { t } = useTranslation();
  const attention = useSlot("attention");
  const label = t(`assistant.runStates.${state}`);
  if (onField) {
    return (
      <Chip size="sm" className="h-6 shrink-0 bg-current/15 px-2.5 text-(--work-field-fg)">
        {label}
      </Chip>
    );
  }
  if (state === "awaiting_approval" && attention === "seal") {
    return (
      <span className="inline-flex items-center gap-1.5 text-sm font-strong text-fg">
        <Seal size={13} />
        {label}
      </span>
    );
  }
  if (state === "awaiting_approval" && attention === "lamp") {
    return (
      <span className="inline-flex items-center gap-1.5 text-sm font-mid text-attention">
        <Lamp tone="attention" />
        {label}
      </span>
    );
  }
  if (state === "awaiting_approval") {
    return (
      <span className="inline-flex shrink-0 items-center gap-1.5 rounded-chip border border-attention-line bg-attention-soft px-2.5 py-0.5 text-sm font-mid text-attention">
        <Lamp tone="attention" className="size-1.5" />
        {label}
      </span>
    );
  }
  return <StatusLabel tone={RUN_TONE[state]} className="text-sm">{label}</StatusLabel>;
}

function factsOf(blocks: Block[]): Fact[] {
  return blocks.flatMap((block) => (block.type === "stat" ? [{ label: block.label, value: block.value, unit: block.unit, note: block.hint }] : []));
}

/** Result of a run: what it is about, the summary, and its blocks (the change preview and the request). */
export function RunResult({
  run,
  onAction,
  onBackground,
  deciding,
}: {
  run: AssistantRun | undefined;
  onAction: (run: AssistantRun, action: BlockAction) => void;
  onBackground: (run: AssistantRun) => void;
  deciding: boolean;
}) {
  const { t } = useTranslation();
  const summary = useSlot("summary");
  const section = useSlot("section");
  const work = useWorkColor(run?.subject?.artworkId, run?.subject?.title);
  if (!run) return <EmptyState icon={LayoutPanelTop} title={t("assistant.resultEmptyTitle")} detail={t("assistant.canvasEmpty")} />;

  const facts = factsOf(run.blocks);
  const body = run.blocks.filter((block) => block.type !== "stat");
  const onField = Boolean(work);
  const title = run.title ?? t("assistant.title");

  const background = run.taskId ? (
    <Link to="/tasks" search={{ task: run.taskId }} className={buttonVariants({ variant: "ghost", size: "sm" })}>
      {t("assistant.openTask")}
    </Link>
  ) : isRunActive(run) ? (
    <Button size="sm" variant="outline" className={cn(onField && "button--on-field")} onPress={() => onBackground(run)}>
      <ArrowDownToLine aria-hidden />
      {t("assistant.toBackground")}
    </Button>
  ) : null;

  return (
    <div className={cn("flex min-h-0 flex-col lg:h-full lg:overflow-y-auto", section === "panel" && "rounded-panel border border-line bg-surface max-lg:overflow-hidden")}>
      <header
        className={cn("grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-4 gap-y-2 px-(--pv-pad-panel) py-4", onField ? "work-field py-[1.125rem]" : "border-b border-line")}
        style={work?.style}
      >
        {onField ? <Cover artworkId={run.subject?.artworkId} title={run.subject?.title ?? title} decorative className="size-[3.25rem] rounded-tile shadow-art" /> : <span />}
        <div className="min-w-0">
          <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
            <h2 className="min-w-0 font-heading text-xl font-title text-balance">{title}</h2>
            {!onField ? <RunStatus state={run.state} onField={false} /> : null}
          </div>
          {summary === "inline" && facts.length > 0 ? <SummaryFacts facts={facts} onField={onField} className={cn("mt-1 text-sm", onField && "[&_dd]:text-(--work-field-fg)")} /> : null}
        </div>
        <div className="flex items-center gap-2">
          {onField ? <RunStatus state={run.state} onField /> : null}
          {background}
        </div>
      </header>
      {summary === "cells" && facts.length > 0 ? <SummaryFacts facts={facts} className="border-t-0" /> : null}
      <div className="flex flex-1 flex-col gap-4 px-(--pv-pad-panel) pt-4">
        {body.length > 0 ? (
          <BlockRenderer
            key={`${run.id}-${run.state}`}
            blocks={body}
            readOnly={!isRunActive(run)}
            busy={deciding}
            pinApprovals
            onAction={(action) => onAction(run, action)}
            className="flex-1 content-start"
          />
        ) : (
          <EmptyState compact icon={LayoutPanelTop} title={t("assistant.working")} />
        )}
      </div>
    </div>
  );
}
