/**
 * One piece of background work as a row: what it is, what it is working on, and its progress
 * drawn per the `progress` slot. Used by the overview's 正在进行 and the task list. With
 * `onOpen` the title opens the task (the whole row is the target); `detailed` adds how long a
 * finished task took and why a failed one failed. A scan that could not read some of its places
 * finishes in a warning state of its own, wherever the row is shown.
 */
import { TriangleAlert, Workflow } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { scanGaps, scanGapsText } from "@/api/job-text";
import { RelativeTime } from "@/components/relative-time";
import { formatElapsed } from "@/blocks/format";
import { formatPercent } from "@/lib/format";
import type { Task } from "@/provisional/types";
import { useSlot } from "@/skins/scope";
import { Lamp, Measure, StatusLabel } from "@/ui/marks";
import { ProgressBar, RunProgress } from "@/ui/progress";
import { useRowInset } from "@/ui/section";
import { Cover, useWorkColor } from "@/ui/work";
import { KIND_ICONS, STATE_TONE, useRemainingLabel, useTaskLabels } from "./task-kinds";
import { isActiveTask } from "./use-task-feed";

export function RunRow({ task, trailing, onOpen, detailed = false }: { task: Task; trailing?: ReactNode; onOpen?: () => void; detailed?: boolean }) {
  const { t, i18n } = useTranslation();
  const { kindLabel, title } = useTaskLabels();
  const remaining = useRemainingLabel();
  const progress = useSlot("progress");
  const itemColor = useSlot("itemColor");
  const attention = useSlot("attention");
  const inset = useRowInset();
  const subjectTitle = task.subject?.title ?? title(task);
  const work = useWorkColor(task.subject?.artworkId, subjectTitle);
  const running = isActiveTask(task);
  const done = task.state === "succeeded";
  const gaps = done ? scanGaps(task.step) : null;
  const Icon = KIND_ICONS[task.kind] ?? Workflow;
  const percent = task.progress === null ? null : formatPercent(task.progress, i18n.resolvedLanguage ?? "en");
  const titleClass = "truncate text-md font-strong text-fg";
  const label = `${kindLabel(task.kind)} ${subjectTitle}`;
  const left = remaining(task.etaSeconds);

  const lead =
    itemColor === "on" ? (
      <Cover
        artworkId={task.subject?.artworkId}
        title={subjectTitle}
        icon={Icon}
        decorative
        className={cn("shrink-0 rounded-[3px] shadow-art", task.subject?.artworkId?.startsWith("ph-") ? "size-10" : "h-[3.375rem] w-9")}
      />
    ) : (
      <span className="flex w-[4.5rem] shrink-0 items-center gap-2.5 text-sm text-fg-muted max-sm:w-auto">
        {attention === "lamp" ? <Lamp tone={running ? "active" : gaps ? "degraded" : done ? "idle" : STATE_TONE[task.state]} /> : null}
        <span className="max-sm:sr-only">{kindLabel(task.kind)}</span>
      </span>
    );

  const locale = i18n.resolvedLanguage ?? "en";
  const failure = detailed && task.state === "failed" ? task.error : null;
  const warning = gaps ? scanGapsText(t, gaps) : null;
  const message = failure ?? warning ?? task.message;
  const detail = [message, task.rate].filter(Boolean);
  const took = detailed && task.startedAt && task.finishedAt ? formatElapsed(Date.parse(task.finishedAt) - Date.parse(task.startedAt), locale) : null;
  const right = running ? (
    task.progress !== null ? (
      <div className="min-w-[6.5rem] shrink-0 text-end">
        <Measure value={percent} className="block text-md font-title text-fg" />
        {left ? <span className="block text-sm text-fg-muted">{left}</span> : null}
      </div>
    ) : (
      <div className="min-w-[6.5rem] shrink-0 text-end">
        <span className="block text-md font-title text-fg">{t("tasks.inProgress")}</span>
        <span className="block text-sm text-fg-muted">{t("tasks.totalUnknown")}</span>
      </div>
    )
  ) : (
    <div className="text-end">
      <StatusLabel tone={gaps ? "degraded" : STATE_TONE[task.state]} className="text-sm">
        {gaps ? t("jobSteps.incomplete") : t(`tasks.states.${task.state}`)}
      </StatusLabel>
      <span className="block text-sm whitespace-nowrap text-fg-muted">
        <RelativeTime value={task.finishedAt ?? task.createdAt} />
        {took ? <span> · {t("tasks.took", { duration: took })}</span> : null}
      </span>
    </div>
  );

  // What is being done, before the work's name; skipped when the title already says it ("扫描「电影」").
  const prefix = task.subject?.verb ?? kindLabel(task.kind);
  const showPrefix = Boolean(task.subject?.verb) || (itemColor === "on" && (Boolean(task.subject) || !title(task).startsWith(prefix)));
  const heading = (
    <>
      {showPrefix ? <span className="me-2 font-mid text-fg-muted">{prefix}</span> : null}
      {task.subject ? (
        <>
          {task.subject.title}
          {task.subject.detail ? (
            <span className="font-mid">
              {task.kind === "transcode" ? <span className="mx-1.5 text-fg-muted">→</span> : " "}
              {task.subject.detail}
            </span>
          ) : null}
        </>
      ) : (
        title(task)
      )}
    </>
  );

  return (
    <RunProgress value={task.progress} label={label} running={running} wash={work?.colors.wash} edge={work?.colors.bar}>
      <div className={cn("flex min-h-(--pv-row-h) items-center gap-x-3.5 gap-y-2 py-3 max-sm:flex-wrap", inset, itemColor === "on" && "min-h-[4.75rem]", onOpen && "relative")}>
        {lead}
        <div className="grid min-w-0 flex-1 gap-0.5">
          {onOpen ? (
            <button type="button" onClick={onOpen} className={cn(titleClass, "block w-full text-start after:absolute after:inset-0 hover:underline")}>
              {heading}
            </button>
          ) : (
            <p className={titleClass}>{heading}</p>
          )}
          {detail.length > 0 ? (
            <p className={cn("truncate text-sm", failure ? "text-failed" : warning ? "font-mid text-degraded" : "text-fg-muted")}>
              {warning ? <TriangleAlert className="me-1.5 inline size-[1.05em] align-[-0.15em]" aria-hidden /> : null}
              {message}
              {message && task.rate ? <span aria-hidden> · </span> : null}
              {task.rate ? <Measure value={task.rate} /> : null}
              {task.rate && task.kind === "transcode" ? ` ${t("tasks.realtime")}` : null}
            </p>
          ) : null}
        </div>
        {progress === "bar" && running ? (
          <ProgressBar value={task.progress} label={label} className="w-[clamp(6rem,16vw,15rem)] shrink-0 max-md:order-last max-md:w-full" />
        ) : null}
        {right}
        {trailing}
      </div>
    </RunProgress>
  );
}
