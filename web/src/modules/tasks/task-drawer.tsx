import { Button, buttonVariants, Drawer, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ScanSearch, SearchX, Settings2 } from "lucide-react";
import { useState, useSyncExternalStore, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { scanGaps, scanGapsText, type ScanGaps } from "@/api/job-text";
import { useIsAdmin } from "@/api/queries/auth";
import { useScanLibrary } from "@/api/queries/libraries";
import { formatElapsed } from "@/lib/elapsed";
import { RelativeTime } from "@/components/relative-time";
import { errorText } from "@/lib/errors";
import { formatPercent } from "@/lib/format";
import { useNow } from "@/lib/use-now";
import type { Task } from "./task";
import { useSlot } from "@/skins/scope";
import { Measure, StatusLabel } from "@/ui/marks";
import { ProgressBar, RunProgress } from "@/ui/progress";
import { EmptyState, LoadingRows } from "@/ui/states";
import { useTaskLabels } from "./task-kinds";
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
  return (
    <div className="grid gap-2">
      <RunProgress value={task.progress} label={label} wash={wash} edge={edge}>
        <div className="flex items-baseline justify-between gap-4 px-1 py-2.5">
          <span className="min-w-0 truncate text-sm text-fg-muted">
            {task.state === "queued" ? t("tasks.states.queued") : (task.message ?? t("tasks.inProgress"))}
          </span>
          <span className="shrink-0 text-end">
            {task.progress !== null ? (
              <Measure value={formatPercent(task.progress, i18n.resolvedLanguage ?? "en")} className="text-md font-strong text-fg" />
            ) : (
              <span className="text-sm text-fg-muted">{t("tasks.totalUnknown")}</span>
            )}
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

function TaskDetail({ task, phone }: { task: Task; phone: boolean }) {
  const { t, i18n } = useTranslation();
  const { title, kindLabel } = useTaskLabels();
  const now = useNow(30_000);
  const locale = i18n.resolvedLanguage ?? "en";
  const name = title(task);
  const active = isActiveTask(task);
  const gaps = task.state === "succeeded" ? scanGaps(task.step) : null;

  const started = task.startedAt ?? task.createdAt;
  const took = task.finishedAt ? formatElapsed(Date.parse(task.finishedAt) - Date.parse(started), locale) : null;
  const running = active && task.startedAt ? formatElapsed(now - Date.parse(task.startedAt), locale) : null;
  const meta = [kindLabel(task.kind), task.scope].filter(Boolean).join(" · ");

  return (
    <>
      <Drawer.CloseTrigger aria-label={t("a11y.close")} className={cn("absolute end-3 z-10", phone ? "top-5" : "top-3")} />
      <Drawer.Header className={cn("flex-row items-start gap-4 px-(--pv-pad-panel) pt-5 pb-4", "border-b border-line")}>
        <div className="grid min-w-0 flex-1 gap-1.5 pe-9">
          <Drawer.Heading className={cn("font-heading text-xl font-title break-words", "text-heading")}>{name}</Drawer.Heading>
          <p className={cn("flex flex-wrap items-center gap-x-2 gap-y-1 text-sm", "text-fg-muted")}>
            {gaps ? (
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
        {active ? <RunningProgress task={task} /> : null}
        {task.state === "failed" ? <Failure task={task} /> : null}
        {gaps ? <Incomplete task={task} gaps={gaps} /> : null}

        <dl className="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-6 gap-y-2">
          <Fact label={t("tasks.created")}>
            <RelativeTime value={started} />
          </Fact>
          {took ? <Fact label={t("tasks.duration")}>{took}</Fact> : running ? <Fact label={t("tasks.elapsed")}>{running}</Fact> : null}
          <Fact label={t("tasks.id")}>
            <span className="break-all text-fg-muted select-all">{task.id}</span>
          </Fact>
        </dl>

          <p className="text-sm text-fg-muted">{t("tasks.noSteps")}</p>
        <span aria-hidden className="block h-0" />
      </Drawer.Body>
    </>
  );
}

export function TaskDrawer({
  open,
  task,
  loading,
  onClose,
}: {
  open: boolean;
  task: Task | undefined;
  loading: boolean;
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
            <TaskDetail task={shown} phone={phone} />
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
