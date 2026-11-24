import type { TFunction } from "i18next";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { jobDetail, stepText, subjectTitle } from "@/api/job-text";
import { useIsAdmin } from "@/api/queries/auth";
import { useJobs } from "@/api/queries/jobs";
import type { Job } from "@/api/types";
import { useAccess } from "@/modules/access";
import { useProvisionalTasks } from "@/provisional/queries/tasks";
import type { Task } from "@/provisional/types";

export type TaskSource = "tasks" | "jobs" | "none";

/** M0 jobs carry no steps or results; they become minimal tasks with localised text. */
export function jobToTask(job: Job, t: TFunction): Task {
  return {
    id: job.id,
    kind: job.kind,
    title: subjectTitle(t, job.kind, job.subject) ?? "",
    subject: job.subject?.name ? { title: job.subject.name, detail: null, artworkId: null } : null,
    scope: job.subject?.type === "library" ? job.subject.name : null,
    libraryId: job.subject?.type === "library" ? job.subject.id : null,
    step: job.step,
    state: job.state,
    progress: job.progress,
    message: jobDetail(t, job),
    error: job.error,
    createdAt: job.createdAt,
    startedAt: job.startedAt,
    finishedAt: job.finishedAt,
    steps: [],
    result: [],
    origin: null,
  };
}

const EMPTY: Task[] = [];

/**
 * Every kind of background work in one list. With the provisional "tasks" feature the server
 * provides rich tasks (steps, results, approvals); on an M0 server this is the jobs API.
 */
export function useTaskFeed(): { tasks: Task[]; source: TaskSource; isPending: boolean } {
  const { features } = useAccess();
  const isAdmin = useIsAdmin();
  const rich = features.includes("tasks");
  const jobsAvailable = !rich && isAdmin && features.includes("jobs");
  const { t } = useTranslation();
  const provisional = useProvisionalTasks(rich);
  const jobs = useJobs({ enabled: jobsAvailable });
  const mapped = useMemo(() => jobs.data?.jobs.map((job) => jobToTask(job, t)) ?? EMPTY, [jobs.data, t]);
  // Unified tasks that mirror a job carry its step: localise it the same way.
  const richTasks = useMemo(
    () => provisional.data?.tasks.map((task) => (task.step ? { ...task, message: stepText(t, task.step, task.message) } : task)) ?? EMPTY,
    [provisional.data, t],
  );

  if (rich) return { tasks: richTasks, source: "tasks", isPending: provisional.isPending };
  if (jobsAvailable) return { tasks: mapped, source: "jobs", isPending: jobs.isPending };
  return { tasks: EMPTY, source: "none", isPending: false };
}

export function isActiveTask(task: Pick<Task, "state">): boolean {
  return task.state === "queued" || task.state === "running";
}
