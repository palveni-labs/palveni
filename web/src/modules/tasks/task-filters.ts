import type { Task } from "./task";

export type TaskFilter = "all" | "active" | "done" | "failed";
export const TASK_FILTERS: TaskFilter[] = ["all", "active", "done", "failed"];

export function isTaskFilter(value: unknown): value is TaskFilter {
  return typeof value === "string" && (TASK_FILTERS as string[]).includes(value);
}

/** The group a task is listed in: waiting for the user first, then running, then finished. */
export type TaskGroup = "active" | "finished";

export function groupOf(task: Pick<Task, "state">): TaskGroup {
  if (task.state === "queued" || task.state === "running") return "active";
  return "finished";
}

export function matchesFilter(task: Pick<Task, "state">, filter: TaskFilter): boolean {
  switch (filter) {
    case "active":
      return task.state === "queued" || task.state === "running";
    case "done":
      return task.state === "succeeded";
    case "failed":
      return task.state === "failed" || task.state === "cancelled";
    case "all":
      return true;
  }
}
