import { ScanSearch } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { scanGaps, scanGapsText } from "@/api/job-text";
import { useIsAdmin } from "@/api/queries/auth";
import { useScanLibrary } from "@/api/queries/libraries";
import { useNow } from "@/lib/use-now";
import type { ActivitySummary, InboxItem } from "@/modules/types";
import { useInboxItems } from "@/shell/inbox-store";
import { useApproveTask, useCancelTask } from "@/provisional/queries/tasks";
import type { Task } from "@/provisional/types";
import { useTaskLabels } from "./task-kinds";
import { isActiveTask, useTaskFeed } from "./use-task-feed";

const RECENT_MS = 24 * 3600_000;

/** Rows a task's change preview selects by default (what "确认" approves without opening it). */
export function defaultSelection(task: Task): string[] {
  return task.result.flatMap((block) => (block.type === "change-preview" ? block.rows.filter((row) => row.selected).map((row) => row.id) : []));
}

/**
 * Scans whose newest run could not read some of the library's places: the library may be out of
 * date there until the storage is back and it is scanned again.
 */
function incompleteScans(tasks: Task[]): Set<string> {
  const latest = new Map<string, Task>();
  for (const task of tasks) {
    if (task.kind !== "library.scan" || !task.libraryId) continue;
    const seen = latest.get(task.libraryId);
    if (!seen || Date.parse(task.createdAt) > Date.parse(seen.createdAt)) latest.set(task.libraryId, task);
  }
  return new Set([...latest.values()].filter((task) => task.state === "succeeded" && scanGaps(task.step)).map((task) => task.id));
}

/**
 * Approvals waiting for the user, incomplete scans (as alerts, until the library is scanned
 * again), and tasks that finished or failed in the last day.
 */
export function useTaskInbox(): InboxItem[] {
  const { t } = useTranslation();
  const { title } = useTaskLabels();
  const { tasks } = useTaskFeed();
  const isAdmin = useIsAdmin();
  const approve = useApproveTask();
  const cancel = useCancelTask();
  const scan = useScanLibrary();
  const now = useNow(60_000);
  const busyId = approve.isPending ? approve.variables.id : cancel.isPending ? cancel.variables : null;
  const scanningId = scan.isPending ? scan.variables : null;
  return useMemo(() => {
    const incomplete = incompleteScans(tasks);
    return tasks.flatMap((task): InboxItem[] => {
      const gaps = incomplete.has(task.id) ? scanGaps(task.step) : null;
      if (gaps && task.finishedAt) {
        const libraryId = task.libraryId ?? "";
        return [
          {
            id: `scan-gaps:${task.id}`,
            kind: "alert",
            title: title(task),
            detail: scanGapsText(t, gaps),
            state: t("jobSteps.incomplete"),
            createdAt: task.finishedAt,
            tone: "warning",
            to: "/tasks",
            search: { task: task.id },
            actions: [
              { id: "review", label: t("attention.details"), variant: "ghost", to: "/tasks", search: { task: task.id } },
              ...(isAdmin && libraryId
                ? [{ id: "rescan", label: t("jobSteps.rescan"), variant: "outline" as const, icon: ScanSearch, run: () => scan.mutate(libraryId), busy: scanningId === libraryId }]
                : []),
            ],
          },
        ];
      }
      if (task.state === "awaiting_approval") {
        return [
          {
            id: task.id,
            kind: "approval",
            title: title(task),
            detail: task.message ?? undefined,
            createdAt: task.createdAt,
            tone: "warning",
            to: "/tasks",
            search: { task: task.id },
            scope: task.scope ?? undefined,
            previews: task.previews,
            actions: [
              { id: "dismiss", label: t("attention.dismiss"), variant: "ghost", run: () => cancel.mutate(task.id), busy: busyId === task.id },
              { id: "review", label: t("attention.review"), variant: "outline", to: "/tasks", search: { task: task.id } },
              {
                id: "confirm",
                label: t("attention.confirm"),
                variant: "confirm",
                run: () => approve.mutate({ id: task.id, selectedRowIds: defaultSelection(task) }),
                busy: busyId === task.id,
              },
            ],
          },
        ];
      }
      const finished = task.finishedAt ? Date.parse(task.finishedAt) : 0;
      if ((task.state === "succeeded" || task.state === "failed") && now - finished < RECENT_MS && task.finishedAt) {
        return [
          {
            id: task.id,
            kind: "notification",
            title: title(task),
            detail: task.state === "failed" ? (task.error ?? t("tasks.states.failed")) : (task.message ?? t("tasks.states.succeeded")),
            createdAt: task.finishedAt,
            tone: task.state === "failed" ? "danger" : "success",
            to: "/tasks",
            search: { task: task.id },
          },
        ];
      }
      return [];
    });
  }, [tasks, title, t, now, approve, cancel, busyId, isAdmin, scan, scanningId]);
}

/**
 * Navigation badge: work that needs the user — every request waiting for confirmation (task or
 * assistant run, as collected in the inbox) plus tasks that failed in the last day.
 */
export function useAwaitingCount(): number {
  const { tasks } = useTaskFeed();
  const now = useNow(60_000);
  const approvals = useInboxItems().filter((item) => item.kind === "approval").length;
  const failed = tasks.filter((task) => task.state === "failed" && task.finishedAt !== null && now - Date.parse(task.finishedAt) < RECENT_MS).length;
  return approvals + failed;
}

/** The most important running work, for the shell's activity display. */
export function useTaskActivity(): ActivitySummary | null {
  const { kindLabel } = useTaskLabels();
  const { tasks } = useTaskFeed();
  const running = tasks.filter(isActiveTask);
  const lead = running.find((task) => task.progress !== null) ?? running[0];
  if (!lead) return null;
  return {
    label: kindLabel(lead.kind),
    title: lead.subject?.title ?? lead.title ?? kindLabel(lead.kind),
    progress: lead.progress,
    more: running.length - 1,
    to: "/tasks",
  };
}
