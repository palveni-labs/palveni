import {
  Download,
  Film,
  ScanSearch,
  AudioLines,
  Sparkles,
  Wand2,
  Workflow,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Task, TaskState } from "@/provisional/types";
import type { StatusTone } from "@/ui/marks";

/** How each state is marked (StatusLabel / Lamp tone). */
export const STATE_TONE: Record<TaskState, StatusTone> = {
  queued: "idle",
  running: "active",
  awaiting_approval: "attention",
  succeeded: "ok",
  failed: "failed",
  cancelled: "idle",
};

export const KIND_ICONS: Record<string, LucideIcon> = {
  "music.loudness": AudioLines,
  "library.scan": ScanSearch,
  download: Download,
  transcode: Film,
  ai: Wand2,
  "assistant.run": Sparkles,
  automation: Workflow,
};

export const TASK_KINDS = ["library.scan", "music.loudness", "download", "transcode", "ai", "assistant.run", "automation"] as const;

const KIND_LABEL_KEYS = {
  "music.loudness": "music.loudness",
  "library.scan": "tasks.kinds.libraryScan",
  download: "tasks.kinds.download",
  transcode: "tasks.kinds.transcode",
  ai: "tasks.kinds.ai",
  "assistant.run": "tasks.kinds.assistantRun",
  automation: "tasks.kinds.automation",
} as const;

export function useTaskLabels() {
  const { t } = useTranslation();
  const kindLabel = (kind: string) =>
    kind in KIND_LABEL_KEYS ? t(KIND_LABEL_KEYS[kind as keyof typeof KIND_LABEL_KEYS]) : kind;
  /** M0 jobs have no title; their kind stands in. */
  const title = (task: Task) => task.title || kindLabel(task.kind);
  return { kindLabel, title };
}

/** "剩余约 6 分钟" from an estimate in seconds; null when there is no estimate. */
export function useRemainingLabel() {
  const { t } = useTranslation();
  return (seconds: number | null | undefined): string | null => {
    if (seconds === null || seconds === undefined) return null;
    return seconds < 90 ? t("tasks.remainingSeconds") : t("tasks.remainingMinutes", { count: Math.round(seconds / 60) });
  };
}
