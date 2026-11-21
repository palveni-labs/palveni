import { ListChecks, ListTodo } from "lucide-react";
import { lazy } from "react";
import type { ModuleDefinition } from "../types";
import { useFailureCount, useTaskActivity, useTaskInbox } from "./use-task-inbox";
import { isActiveTask, useTaskFeed } from "./use-task-feed";

function useRunningCount(): number | undefined {
  const { tasks, isPending } = useTaskFeed();
  return isPending ? undefined : tasks.filter(isActiveTask).length;
}

const tasks: ModuleDefinition = {
  id: "tasks",
  order: 40,
  titleKey: "modules.tasks.title",
  icon: ListChecks,
  home: "/tasks",
  feature: "jobs",
  badge: useFailureCount,
  inbox: useTaskInbox,
  activity: useTaskActivity,
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
};

export default tasks;
