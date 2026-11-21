/**
 * Localised text for M0 jobs: a title from `subject`, the current step from `step.code` with its
 * `params`, and the server's English `message` only for codes this client does not know.
 */
import type { TFunction } from "i18next";
import type { Job, JobStep, JobSubject } from "./types";

const STEP_KEYS = {
  "scan.discovering": "jobSteps.scanDiscovering",
  "scan.indexing": "jobSteps.scanIndexing",
  "scan.summary": "jobSteps.scanSummary",
} as const;

export function stepText(t: TFunction, step: JobStep | null | undefined, fallback: string | null): string | null {
  if (!step) return fallback;
  const key = STEP_KEYS[step.code as keyof typeof STEP_KEYS];
  return key ? t(key, step.params) : fallback;
}

/** "扫描「家庭相册」" for a scan of a library; null when the job is about no known object. */
export function subjectTitle(t: TFunction, kind: string, subject: JobSubject | null | undefined): string | null {
  if (!subject?.name) return null;
  if (kind === "library.scan" && subject.type === "library") return t("jobSteps.scanTitle", { name: subject.name });
  return null;
}

/** What a finished scan could not read. Items under these places were kept, never removed. */
export interface ScanGaps {
  unavailablePaths: number;
  unreadableFolders: number;
}

function count(value: string | number | undefined): number {
  const n = typeof value === "number" ? value : Number(value ?? 0);
  return Number.isFinite(n) && n > 0 ? Math.floor(n) : 0;
}

/** The gaps a scan summary reports, or null when the scan read everything it was given. */
export function scanGaps(step: JobStep | null | undefined): ScanGaps | null {
  if (step?.code !== "scan.summary") return null;
  const gaps = { unavailablePaths: count(step.params.unavailablePaths), unreadableFolders: count(step.params.unreadableFolders) };
  return gaps.unavailablePaths > 0 || gaps.unreadableFolders > 0 ? gaps : null;
}

/** "1 个路径不可用，其中的条目保持不变": what was skipped and that nothing under it was removed. */
export function scanGapsText(t: TFunction, gaps: ScanGaps): string {
  const parts = [
    gaps.unavailablePaths > 0 ? t("jobSteps.gapPaths", { count: gaps.unavailablePaths }) : null,
    gaps.unreadableFolders > 0 ? t("jobSteps.gapFolders", { count: gaps.unreadableFolders }) : null,
  ].filter((part): part is string => part !== null);
  return t("jobSteps.gapKept", { gaps: parts.join(t("jobSteps.gapJoin")) });
}

/** The line describing where a job stands: step, or the failure reason. */
export function jobDetail(t: TFunction, job: Pick<Job, "state" | "step" | "message" | "error">): string | null {
  if (job.state === "failed") return job.error;
  if (job.state === "cancelled") return null;
  return stepText(t, job.step, job.message);
}
