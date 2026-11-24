/**
 * Task detail (search param `task`): what the task is, how far it got in plain-language steps,
 * its result blocks — including a pending confirmation request, which works here exactly as in
 * the assistant — and what to do when it failed or a scan could not read everything. A sheet from
 * the right on wide screens, from the bottom on phones.
 */
import { Button, buttonVariants, Drawer, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { Check, CircleSlash, Loader2, Minus, ScanSearch, SearchX, Settings2, X } from "lucide-react";
import { useState, useSyncExternalStore, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { scanGaps, scanGapsText, type ScanGaps } from "@/api/job-text";
import { useIsAdmin } from "@/api/queries/auth";
import { useScanLibrary } from "@/api/queries/libraries";
import { BlockRenderer } from "@/blocks/block-renderer";
import { formatElapsed } from "@/blocks/format";
import type { BlockAction } from "@/blocks/types";
import { RelativeTime } from "@/components/relative-time";
import { errorText } from "@/lib/errors";
import { formatPercent } from "@/lib/format";
import { useNow } from "@/lib/use-now";
import { useApproveTask, useCancelTask } from "@/provisional/queries/tasks";
import type { Task, TaskStep } from "@/provisional/types";
import { useSlot } from "@/skins/scope";
import { Lamp, Measure, Seal, StatusLabel } from "@/ui/marks";
import { ProgressBar, RunProgress } from "@/ui/progress";
import { EmptyState, LoadingRows } from "@/ui/states";
import { Cover, useWorkColor } from "@/ui/work";
import { useRemainingLabel, useTaskLabels } from "./task-kinds";
import { TaskStateLabel } from "./task-meta";
import { isActiveTask } from "./use-task-feed";

const PHONE_QUERY = "(max-width: 639px)";

function useIsPhone(): boolean {
  return useSyncExternalStore(
    (onChange) => {
      if (typeof window.matchMedia !== "function") return () => {};
      const list = window.matchMedia(PHONE_QUERY);
      list.addEventListener("change", onChange);
      return () => list.removeEventListener("change", onChange);
    },
    () => typeof window.matchMedia === "function" && window.matchMedia(PHONE_QUERY).matches,
    () => false,
  );
}

const HEADING = "font-heading text-md font-title text-heading";

/* ---------------------------------------------------------------- steps */

/** "Waiting for you" as the `attention` slot marks it. */
function WaitingMark() {
  const attention = useSlot("attention");
  if (attention === "seal") return <Seal size={14} />;
  if (attention === "lamp") return <Lamp tone="attention" className="size-2.5" />;
  return (
    <span className="grid size-[1.125rem] place-items-center rounded-full bg-attention-mark" aria-hidden>
      <span className="size-1.5 rounded-full bg-(--pv-attention-mark-fg)" />
    </span>
  );
}

function StepMark({ state, waiting }: { state: TaskStep["state"]; waiting: boolean }) {
  if (waiting) return <WaitingMark />;
  switch (state) {
    case "done":
      return <Check className="size-4 text-ok" strokeWidth={2.4} aria-hidden />;
    case "failed":
      return <X className="size-4 text-failed" strokeWidth={2.4} aria-hidden />;
    case "running":
      return <Loader2 className="pv-spin size-4 text-active-mark" aria-hidden />;
    case "skipped":
      return <Minus className="size-4 text-fg-subtle" strokeWidth={2.4} aria-hidden />;
    case "pending":
      return <span className="size-2.5 rounded-full border-[1.5px] border-line-strong" aria-hidden />;
  }
}

/** The task's steps in order, each with its mark, words and time; a rule joins the marks. */
function StepTimeline({ task }: { task: Task }) {
  const { t } = useTranslation();
  const awaiting = task.state === "awaiting_approval";
  return (
    <ol className="grid">
      {task.steps.map((step, index) => {
        const waiting = awaiting && step.state === "running";
        const quiet = step.state === "pending" || step.state === "skipped";
        return (
          <li key={step.id} className="relative grid grid-cols-[1.375rem_minmax(0,1fr)] gap-x-3 pb-4 last:pb-0">
            {index < task.steps.length - 1 ? <span aria-hidden className="absolute top-6 bottom-1 left-[0.68rem] w-px bg-line-strong" /> : null}
            <span className="grid h-6 place-items-center">
              <StepMark state={step.state} waiting={waiting} />
            </span>
            <div className="min-w-0">
              <div className="flex items-baseline justify-between gap-3">
                <p className={cn("min-w-0 text-base", quiet ? "text-fg-muted" : "font-strong text-fg", step.state === "skipped" && "line-through decoration-line-strong")}>
                  {step.label}
                  <span className="sr-only">
                    {" "}
                    {waiting ? t("tasks.stepStates.waiting") : t(`tasks.stepStates.${step.state}`)}
                  </span>
                </p>
                {step.at ? <RelativeTime value={step.at} className="shrink-0 text-xs text-fg-muted" /> : null}
              </div>
              {waiting ? (
                <p className="mt-0.5 text-sm font-mid text-attention">{t("tasks.waitingStep")}</p>
              ) : step.detail ? (
                <p className="mt-0.5 text-sm text-fg-muted">{step.detail}</p>
              ) : null}
            </div>
          </li>
        );
      })}
    </ol>
  );
}

/* ---------------------------------------------------------------- parts */

function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="contents">
      <dt className="text-sm text-fg-muted">{label}</dt>
      <dd className="min-w-0 text-sm text-fg">{children}</dd>
    </div>
  );
}

/** Progress of running work, drawn the way the `progress` slot draws every running row. */
function RunningProgress({ task, wash, edge }: { task: Task; wash?: string; edge?: string }) {
  const { t, i18n } = useTranslation();
  const progress = useSlot("progress");
  const label = t("tasks.progressOf", { title: task.title || task.kind });
  const left = useRemainingLabel()(task.etaSeconds);
  return (
    <div className="grid gap-2">
      <RunProgress value={task.progress} label={label} wash={wash} edge={edge}>
        <div className="flex items-baseline justify-between gap-4 px-1 py-2.5">
          <span className="min-w-0 truncate text-sm text-fg-muted">
            {task.state === "queued" ? t("tasks.states.queued") : (task.message ?? t("tasks.inProgress"))}
            {task.rate ? (
              <>
                <span aria-hidden> · </span>
                <Measure value={task.rate} />
              </>
            ) : null}
          </span>
          <span className="shrink-0 text-end">
            {task.progress !== null ? (
              <Measure value={formatPercent(task.progress, i18n.resolvedLanguage ?? "en")} className="text-md font-strong text-fg" />
            ) : (
              <span className="text-sm text-fg-muted">{t("tasks.totalUnknown")}</span>
            )}
            {left ? <span className="ms-2 text-sm text-fg-muted">{left}</span> : null}
          </span>
        </div>
      </RunProgress>
      {progress === "bar" ? <ProgressBar value={task.progress} label={label} /> : null}
    </div>
  );
}

interface Recovery {
  hint: "transcode" | "libraryScan" | "download" | "default";
  link?: { to: "/system/hardware" | "/system/libraries"; label: "openHardware" | "openLibraries" };
}

/** What to try next when a task failed, by kind. */
const RECOVERY: Record<string, Recovery> = {
  transcode: { hint: "transcode", link: { to: "/system/hardware", label: "openHardware" } },
  "library.scan": { hint: "libraryScan", link: { to: "/system/libraries", label: "openLibraries" } },
  download: { hint: "download" },
};

function Failure({ task }: { task: Task }) {
  const { t } = useTranslation();
  const recovery: Recovery = RECOVERY[task.kind] ?? { hint: "default" };
  return (
    <section aria-label={t("tasks.failedTitle")} className="grid gap-2 rounded-tile bg-failed-soft px-4 py-3.5">
      <StatusLabel tone="failed" className="text-sm">
        {t("tasks.failedTitle")}
      </StatusLabel>
      {task.error ? <p className="text-sm break-words text-fg">{task.error}</p> : null}
      <p className="text-sm text-fg-muted">{t(`tasks.recovery.${recovery.hint}`)}</p>
      {recovery.link ? (
        <Link to={recovery.link.to} className={cn(buttonVariants({ variant: "outline", size: "sm" }), "justify-self-start")}>
          <Settings2 aria-hidden />
          {t(`tasks.recovery.${recovery.link.label}`)}
        </Link>
      ) : null}
    </section>
  );
}

/**
 * A scan that finished without reading some of its places. Nothing under them was removed; the
 * user needs to know the library may be out of date there, and why.
 */
function Incomplete({ task, gaps }: { task: Task; gaps: ScanGaps }) {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const scan = useScanLibrary();
  const libraryId = task.libraryId;
  const rescan = () => {
    if (!libraryId) return;
    scan.mutate(libraryId, {
      onSuccess: () => toast.success(t("adminLibraries.scanStarted", { name: task.scope ?? task.subject?.title ?? "" })),
      onError: (error) => toast.danger(errorText(t, error)),
    });
  };
  return (
    <section aria-label={t("jobSteps.gapTitle")} className="grid gap-2 rounded-tile border border-degraded-line bg-degraded-soft px-4 py-3.5">
      <StatusLabel tone="degraded" className="text-sm">
        {t("jobSteps.gapTitle")}
      </StatusLabel>
      <p className="text-sm font-mid text-fg">{scanGapsText(t, gaps)}</p>
      <p className="text-sm text-fg">{t("jobSteps.gapWhy")}</p>
      <div className="flex flex-wrap gap-2">
        {isAdmin && libraryId ? (
          <Button size="sm" variant="outline" isPending={scan.isPending} onPress={rescan}>
            <ScanSearch aria-hidden />
            {t("jobSteps.rescan")}
          </Button>
        ) : null}
        {isAdmin ? (
          <Link to="/system/libraries" className={buttonVariants({ variant: "ghost", size: "sm" })}>
            <Settings2 aria-hidden />
            {t("tasks.recovery.openLibraries")}
          </Link>
        ) : null}
      </div>
    </section>
  );
}

/* ---------------------------------------------------------------- detail */

function TaskDetail({ task, interactive, phone }: { task: Task; interactive: boolean; phone: boolean }) {
  const { t, i18n } = useTranslation();
  const { title, kindLabel } = useTaskLabels();
  const approve = useApproveTask();
  const cancel = useCancelTask();
  const now = useNow(30_000);
  const locale = i18n.resolvedLanguage ?? "en";
  const name = title(task);
  const subject = task.subject;
  const work = useWorkColor(subject?.artworkId, subject?.title);
  const active = isActiveTask(task);
  const gaps = task.state === "succeeded" ? scanGaps(task.step) : null;

  const cancelTask = () =>
    cancel.mutate(task.id, {
      onSuccess: () => toast(t("tasks.cancelled")),
      onError: (error) => toast.danger(errorText(t, error)),
    });

  const onAction = (action: BlockAction) => {
    if (!interactive) return;
    if (action.type === "approve") {
      approve.mutate(
        { id: task.id, selectedRowIds: action.selectedRowIds },
        { onSuccess: () => toast.success(t("tasks.approved")), onError: (error) => toast.danger(errorText(t, error)) },
      );
    } else if (action.type === "cancel") {
      cancelTask();
    }
  };

  const started = task.startedAt ?? task.createdAt;
  const took = task.finishedAt ? formatElapsed(Date.parse(task.finishedAt) - Date.parse(started), locale) : null;
  const running = active && task.startedAt ? formatElapsed(now - Date.parse(task.startedAt), locale) : null;
  const meta = [kindLabel(task.kind), task.scope].filter(Boolean).join(" · ");

  return (
    <>
      <Drawer.CloseTrigger aria-label={t("a11y.close")} className={cn("absolute end-3 z-10", phone ? "top-5" : "top-3", work && "text-(--work-field-fg)")} />
      <Drawer.Header className={cn("flex-row items-start gap-4 px-(--pv-pad-panel) pt-5 pb-4", work ? "work-field" : "border-b border-line")} style={work?.style}>
        {work ? <Cover artworkId={subject?.artworkId} title={subject?.title ?? name} decorative className="h-[4.5rem] w-12 shrink-0 rounded-[4px] shadow-art" /> : null}
        <div className="grid min-w-0 flex-1 gap-1.5 pe-9">
          <Drawer.Heading className={cn("font-heading text-xl font-title break-words", work ? "text-(--work-field-fg)" : "text-heading")}>{name}</Drawer.Heading>
          <p className={cn("flex flex-wrap items-center gap-x-2 gap-y-1 text-sm", work ? "text-(--work-field-sub)" : "text-fg-muted")}>
            {work ? (
              <span className="font-mid text-(--work-field-fg)">{gaps ? t("jobSteps.incomplete") : t(`tasks.states.${task.state}`)}</span>
            ) : gaps ? (
              <StatusLabel tone="degraded" className="text-sm">
                {t("jobSteps.incomplete")}
              </StatusLabel>
            ) : (
              <TaskStateLabel state={task.state} />
            )}
            <span aria-hidden>·</span>
            <span>{meta}</span>
          </p>
        </div>
      </Drawer.Header>
      <Drawer.Body className="mx-0 grid content-start gap-6 px-(--pv-pad-panel) pt-5 pb-0 text-base text-fg">
        {task.message && !active && task.state !== "failed" ? <p className="text-base text-fg">{task.message}</p> : null}
        {active ? <RunningProgress task={task} wash={work?.colors.wash} edge={work?.colors.bar} /> : null}
        {task.state === "failed" ? <Failure task={task} /> : null}
        {gaps ? <Incomplete task={task} gaps={gaps} /> : null}

        <dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-6 gap-y-2">
          <Fact label={t("tasks.created")}>
            <RelativeTime value={started} />
          </Fact>
          {took ? <Fact label={t("tasks.duration")}>{took}</Fact> : running ? <Fact label={t("tasks.elapsed")}>{running}</Fact> : null}
          {task.origin ? <Fact label={t("tasks.origin")}>{task.origin.label}</Fact> : null}
          <Fact label={t("tasks.id")}>
            <span className="break-all text-fg-muted select-all">{task.id}</span>
          </Fact>
        </dl>

        {task.steps.length > 0 ? (
          <section aria-label={t("tasks.steps")} className="grid gap-3">
            <h3 className={HEADING}>{t("tasks.steps")}</h3>
            <StepTimeline task={task} />
          </section>
        ) : !interactive ? (
          <p className="text-sm text-fg-muted">{t("tasks.noSteps")}</p>
        ) : null}

        {task.result.length > 0 ? (
          <section aria-label={t("tasks.result")} className="grid gap-3">
            <h3 className={HEADING}>{t("tasks.result")}</h3>
            <BlockRenderer
              key={`${task.id}-${task.state}`}
              blocks={task.result}
              readOnly={!interactive}
              busy={approve.isPending || cancel.isPending}
              pinApprovals
              onAction={onAction}
            />
          </section>
        ) : null}
        {/* Space at the end without padding the scroller, so a pinned request sits flush at the bottom. */}
        <span aria-hidden className="block h-0" />
      </Drawer.Body>
      {interactive && active ? (
        <Drawer.Footer className="border-t border-line px-(--pv-pad-panel) py-3.5">
          <Button variant="outline" onPress={cancelTask} isPending={cancel.isPending}>
            <CircleSlash aria-hidden />
            {t("tasks.cancel")}
          </Button>
        </Drawer.Footer>
      ) : null}
    </>
  );
}

export function TaskDrawer({
  open,
  task,
  loading,
  interactive,
  onClose,
}: {
  open: boolean;
  task: Task | undefined;
  loading: boolean;
  interactive: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const { title } = useTaskLabels();
  const phone = useIsPhone();
  // Keep showing the last task while the sheet slides away.
  const [kept, setKept] = useState(task);
  if (task && task !== kept) setKept(task);
  const shown = task ?? (open ? undefined : kept);

  return (
    <Drawer.Backdrop isOpen={open} onOpenChange={(next) => (next ? undefined : onClose())}>
      <Drawer.Content placement={phone ? "bottom" : "right"}>
        <Drawer.Dialog
          // Named up front: the heading inside mounts a moment later, while the sheet opens.
          aria-label={shown ? title(shown) : t("tasks.detail")}
          className={cn("gap-0 overflow-hidden p-0", phone ? "max-h-[90svh]" : "w-[min(48rem,94vw)] max-w-none")}
        >
          {phone ? <Drawer.Handle className="pt-2.5" /> : null}
          {shown ? (
            <TaskDetail task={shown} interactive={interactive} phone={phone} />
          ) : loading ? (
            <LoadingRows rows={4} className="px-(--pv-pad-panel) py-6" />
          ) : (
            <>
              <Drawer.CloseTrigger aria-label={t("a11y.close")} className="absolute end-3 top-3 z-10" />
              <EmptyState icon={SearchX} title={t("tasks.notFound")} detail={t("tasks.notFoundDetail")} className="px-6" />
            </>
          )}
        </Drawer.Dialog>
      </Drawer.Content>
    </Drawer.Backdrop>
  );
}
