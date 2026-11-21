import type { TFunction } from "i18next";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { jobDetail, subjectTitle } from "@/api/job-text";
import { useIsAdmin } from "@/api/queries/auth";
import { useJobs } from "@/api/queries/jobs";
import type { Job } from "@/api/types";
import { useAccess } from "@/modules/access";
import type { Task } from "./task";

export type TaskSource = "jobs" | "none";

/** M0 jobs carry no steps or results; they become minimal tasks with localised text. */
export function jobToTask(job: Job, t: TFunction): Task {
  return {
    id: job.id,
    kind: job.kind,
    title: subjectTitle(t, job.kind, job.subject) ?? "",
    subject: job.subject?.name ? { title: job.subject.name } : null,
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
  };
}

const EMPTY: Task[] = [];

export function useTaskFeed(): { tasks: Task[]; source: TaskSource; isPending: boolean } {
  const { features } = useAccess();
  const isAdmin = useIsAdmin();
  const jobsAvailable = isAdmin && features.includes("jobs");
  const { t } = useTranslation();
  const jobs = useJobs({ enabled: jobsAvailable });
  const mapped = useMemo(() => jobs.data?.jobs.map((job) => jobToTask(job, t)) ?? EMPTY, [jobs.data, t]);
  if (jobsAvailable) return { tasks: mapped, source: "jobs", isPending: jobs.isPending };
  return { tasks: EMPTY, source: "none", isPending: false };
}

export function isActiveTask(task: Pick<Task, "state">): boolean {
  return task.state === "queued" || task.state === "running";
}
