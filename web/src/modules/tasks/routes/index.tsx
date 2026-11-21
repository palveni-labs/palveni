import { createFileRoute } from "@tanstack/react-router";
import { isTaskFilter, type TaskFilter } from "../task-filters";
import { TasksPage } from "../tasks-page";

interface TasksSearch {
  filter?: Exclude<TaskFilter, "all">;
  kind?: string;
  /** Opens the detail drawer. */
  task?: string;
}

export const Route = createFileRoute("/_app/tasks/")({
  validateSearch: (search: Record<string, unknown>): TasksSearch => ({
    filter: isTaskFilter(search.filter) && search.filter !== "all" ? search.filter : undefined,
    kind: typeof search.kind === "string" ? search.kind : undefined,
    task: typeof search.task === "string" ? search.task : undefined,
  }),
  component: TasksPage,
});
