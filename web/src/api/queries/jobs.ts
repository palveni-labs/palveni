import { queryOptions, useQuery, type QueryClient } from "@tanstack/react-query";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";
import type { Job, JobList, JobState } from "../types";

export const JOBS_LIMIT = 50;

export const jobsQuery = (limit = JOBS_LIMIT) =>
  queryOptions({
    queryKey: queryKeys.jobs.list(limit),
    queryFn: ({ signal }) => api.jobs.list(limit, signal),
    staleTime: 15_000,
  });

export function isActiveJob(job: Pick<Job, "state">): boolean {
  return job.state === "queued" || job.state === "running";
}

export const TERMINAL_STATES: readonly JobState[] = ["succeeded", "failed", "cancelled"];

export function useJobs(options: { enabled?: boolean; refetchInterval?: number | false } = {}) {
  return useQuery({
    ...jobsQuery(),
    enabled: options.enabled ?? true,
    refetchInterval: options.refetchInterval ?? false,
  });
}

/** Applies a `job.updated` event to every cached job list (newest first, trimmed to its limit). */
export function upsertJob(queryClient: QueryClient, job: Job) {
  const lists = queryClient.getQueriesData<JobList>({ queryKey: queryKeys.jobs.all });
  for (const [key, list] of lists) {
    if (!list) continue;
    const limit = typeof key[2] === "number" ? key[2] : JOBS_LIMIT;
    const others = list.jobs.filter((entry) => entry.id !== job.id);
    const jobs = [job, ...others].sort((a, b) => b.createdAt.localeCompare(a.createdAt)).slice(0, limit);
    queryClient.setQueryData<JobList>(key, { jobs });
  }
}

export function useJob(id: string | undefined, options: { enabled?: boolean } = {}): Job | undefined {
  const { data } = useJobs({ enabled: (options.enabled ?? true) && Boolean(id) });
  return id ? data?.jobs.find((job) => job.id === id) : undefined;
}
