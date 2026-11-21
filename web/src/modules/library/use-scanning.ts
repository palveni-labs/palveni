import { useJobs } from "@/api/queries/jobs";
import { useIsAdmin } from "@/api/queries/auth";
import { useFeature } from "@/modules/access";

const NONE: ReadonlySet<string> = new Set();
const ACTIVE = new Set(["queued", "running"]);

export function useScanningLibraries(): ReadonlySet<string> {
  const hasJobs = useFeature("jobs");
  const isAdmin = useIsAdmin();
  const jobsReadable = isAdmin && hasJobs;
  const jobs = useJobs({ enabled: jobsReadable });
  const ids = (jobs.data?.jobs ?? []).flatMap((job) => (job.kind === "library.scan" && ACTIVE.has(job.state) && job.subject?.type === "library" ? [job.subject.id] : []));
  return ids.length > 0 ? new Set(ids) : NONE;
}
