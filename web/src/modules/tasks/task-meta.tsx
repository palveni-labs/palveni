import { Workflow } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { TaskState } from "@/provisional/types";
import { StatusLabel } from "@/ui/marks";
import { KIND_ICONS, STATE_TONE } from "./task-kinds";

/** The state in words with its icon (never colour alone). */
export function TaskStateLabel({ state, className }: { state: TaskState; className?: string }) {
  const { t } = useTranslation();
  return (
    <StatusLabel tone={STATE_TONE[state]} className={cn("text-sm", className)}>
      {t(`tasks.states.${state}`)}
    </StatusLabel>
  );
}

export function KindIcon({ kind, className }: { kind: string; className?: string }) {
  const Icon = KIND_ICONS[kind] ?? Workflow;
  return (
    <span className={cn("flex size-9 shrink-0 items-center justify-center rounded-tile bg-sunken text-fg-muted", className)}>
      <Icon className="size-[1.05rem]" aria-hidden />
    </span>
  );
}
