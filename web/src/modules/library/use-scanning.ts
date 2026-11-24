import { useJobs } from "@/api/queries/jobs";
import { useIsAdmin } from "@/api/queries/auth";
import { useFeature } from "@/modules/access";
import { useProvisionalTasks } from "@/provisional/queries/tasks";

const NONE: ReadonlySet<string> = new Set();
const ACTIVE = new Set(["queued", "running"]);

/**
 * Ids of libraries with a scan queued or running, wherever it was started. Each scan job names
 * its library in `subject`; with the provisional unified tasks the same jobs
 * arrive as tasks, so either list answers.
 */
export function useScanningLibraries(): ReadonlySet<string> {
  const rich = useFeature("tasks");
  const hasJobs = useFeature("jobs");
  const isAdmin = useIsAdmin();
  const jobsReadable = !rich && isAdmin && hasJobs;
  const tasks = useProvisionalTasks(rich);
  const jobs = useJobs({ enabled: jobsReadable });
  if (rich) {
    const ids = (tasks.data?.tasks ?? []).flatMap((task) => (task.kind === "library.scan" && ACTIVE.has(task.state) && task.libraryId ? [task.libraryId] : []));
    return ids.length > 0 ? new Set(ids) : NONE;
  }
  const ids = (jobs.data?.jobs ?? []).flatMap((job) => (job.kind === "library.scan" && ACTIVE.has(job.state) && job.subject?.type === "library" ? [job.subject.id] : []));
  return ids.length > 0 ? new Set(ids) : NONE;
}
