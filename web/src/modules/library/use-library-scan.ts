import { useIsAdmin } from "@/api/queries/auth";
import { isActiveJob, useJobs } from "@/api/queries/jobs";
import { useLibraryScanJobId } from "@/api/queries/libraries";
import type { Job, Library } from "@/api/types";
import { useFeature } from "@/modules/access";
import { useScanningLibraries } from "./use-scanning";

export interface LibraryScan {
  /** The newest scan job of this library (admins only: members cannot read jobs). */
  job: Job | null;
  /** That job while it is queued or running. */
  active: Job | null;
  /** A scan is queued or running, wherever it was started. */
  running: boolean;
}

/**
 * The scan state of one library. Scan jobs name their library in `subject`, so a scan started
 * anywhere shows here; the job this browser just started is matched by id as well, so the row
 * reacts before the job list has caught up.
 */
export function useLibraryScan(library: Library): LibraryScan {
  const isAdmin = useIsAdmin();
  const hasJobs = useFeature("jobs");
  const startedId = useLibraryScanJobId(library.id);
  const { data } = useJobs({ enabled: isAdmin && hasJobs });
  const scanning = useScanningLibraries().has(library.id);
  const job =
    data?.jobs.find(
      (entry) => entry.id === startedId || (entry.kind === "library.scan" && entry.subject?.type === "library" && entry.subject.id === library.id),
    ) ?? null;
  const active = job && isActiveJob(job) ? job : null;
  return { job, active, running: active !== null || scanning };
}
