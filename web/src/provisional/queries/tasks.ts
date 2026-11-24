import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { provisionalApi } from "../api";
import { provisionalKeys } from "../keys";
import type { Task, TaskList } from "../types";

export function useProvisionalTasks(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.tasks,
    queryFn: ({ signal }) => provisionalApi.tasks.list(signal),
    enabled,
    staleTime: 15_000,
  });
}

/** Applies a `task.updated` event (newest first). */
export function upsertTask(queryClient: QueryClient, task: Task) {
  queryClient.setQueryData<TaskList>(provisionalKeys.tasks, (list) => {
    if (!list) return list;
    const others = list.tasks.filter((entry) => entry.id !== task.id);
    return { tasks: [task, ...others].sort((a, b) => b.createdAt.localeCompare(a.createdAt)) };
  });
}

export function useApproveTask() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, selectedRowIds }: { id: string; selectedRowIds: string[] }) =>
      provisionalApi.tasks.approve(id, { selectedRowIds }),
    onSuccess: (task) => upsertTask(queryClient, task),
  });
}

export function useCancelTask() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.tasks.cancel(id),
    onSuccess: (task) => upsertTask(queryClient, task),
  });
}
