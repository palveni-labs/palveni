import { Check, Loader2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { AssistantRun } from "@/provisional/types";
import { useSlot } from "@/skins/scope";
import { Lamp, Seal } from "@/ui/marks";

function WaitingMark() {
  const attention = useSlot("attention");
  if (attention === "seal") return <Seal size={14} />;
  if (attention === "lamp") return <Lamp tone="attention" className="size-2.5" />;
  return <span className="grid size-[1.375rem] place-items-center rounded-full bg-attention-mark" aria-hidden><span className="size-1.5 rounded-full bg-(--pv-attention-mark-fg)" /></span>;
}

/**
 * The assistant's work as short human-readable steps — never tool calls or raw output. While a
 * run waits for approval, the last step says so in words.
 */
export function RunSteps({ run, className }: { run: AssistantRun; className?: string }) {
  const { t } = useTranslation();
  const waiting = run.state === "awaiting_approval";
  if (run.steps.length === 0 && !waiting) return null;
  const steps = [
    ...run.steps.map((step) => ({ ...step, waiting: false })),
    ...(waiting ? [{ id: "waiting", text: t("assistant.waitingTitle"), detail: t("assistant.waitingDetail"), state: "running" as const, waiting: true }] : []),
  ];
  return (
    <ol className={cn("grid", className)}>
      {steps.map((step, index) => (
        <li key={step.id} className="relative grid grid-cols-[1.375rem_minmax(0,1fr)] gap-x-2.5 pb-3 last:pb-0">
          {index < steps.length - 1 ? <span aria-hidden className="absolute top-6 bottom-0.5 left-[0.68rem] w-px bg-line-strong" /> : null}
          <span className="grid size-[1.375rem] place-items-center">
            {step.waiting ? (
              <WaitingMark />
            ) : step.state === "done" ? (
              <Check className="size-4 text-ok" strokeWidth={2.4} aria-hidden />
            ) : step.state === "failed" ? (
              <X className="size-4 text-failed" strokeWidth={2.4} aria-hidden />
            ) : (
              <Loader2 className="pv-spin size-4 text-active-mark" aria-hidden />
            )}
          </span>
          <div className="min-w-0">
            <p className={cn("text-base", step.waiting || step.state !== "done" ? "font-strong text-fg" : "font-strong text-fg")}>{step.text}</p>
            {step.detail ? <p className="mt-0.5 text-sm leading-relaxed text-fg-muted">{step.detail}</p> : null}
            <span className="sr-only">{step.waiting ? t("assistant.runStates.awaiting_approval") : t(`assistant.stepStates.${step.state}`)}</span>
          </div>
        </li>
      ))}
    </ol>
  );
}
