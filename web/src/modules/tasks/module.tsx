import { Download, FolderSync, ListChecks, ListTodo } from "lucide-react";
import { lazy } from "react";
import { upsertTask } from "@/provisional/queries/tasks";
import type { Task } from "@/provisional/types";
import type { ModuleDefinition } from "../types";
import { useAwaitingCount, useTaskActivity, useTaskInbox } from "./use-task-inbox";
import { isActiveTask, useTaskFeed } from "./use-task-feed";

function useRunningCount(): number | undefined {
  const { tasks, isPending } = useTaskFeed();
  return isPending ? undefined : tasks.filter(isActiveTask).length;
}

/** 任务: every background job in one place (M0 jobs; richer tasks, external downloads & media organizer). */
const tasks: ModuleDefinition = {
  id: "tasks",
  order: 45,
  group: "space",
  titleKey: "modules.tasks.title",
  icon: ListChecks,
  home: "/tasks",
  feature: "jobs",
  badge: useAwaitingCount,
  inbox: useTaskInbox,
  activity: useTaskActivity,
  nav: [
    { to: "/tasks", titleKey: "tasks.nav.queue", icon: ListChecks },
    { to: "/tasks/downloads", titleKey: "tasks.nav.downloads", icon: Download, feature: "downloads" },
    { to: "/tasks/organizer", titleKey: "tasks.nav.organizer", icon: FolderSync, adminOnly: true, feature: "organizer" },
  ],
  widgets: [
    {
      id: "tasks.active",
      titleKey: "widgets.activeTasks",
      icon: ListTodo,
      region: "main",
      rank: 20,
      count: useRunningCount,
      more: { labelKey: "widgets.allTasks", to: "/tasks" },
      component: lazy(() => import("./widgets/active-tasks-widget")),
    },
  ],
  events: {
    "task.updated": (queryClient, data) => upsertTask(queryClient, data as Task),
  },
};

export default tasks;
