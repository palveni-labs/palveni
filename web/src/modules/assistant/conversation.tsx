import { Button, Kbd } from "@heroui/react";
import { ArrowUp, Copy, HardDrive, ListMusic, MessageSquareText, Moon, PanelRightOpen, Subtitles, X } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { BlockRenderer } from "@/blocks/block-renderer";
import type { BlockAction } from "@/blocks/types";
import type { AssistantMessage, AssistantRun, Suggestion } from "@/provisional/types";
import { RunSteps } from "./run-steps";
import { isRunActive } from "./use-assistant";

const SUGGESTION_ICONS = { duplicate: Copy, storage: HardDrive, music: ListMusic, subtitle: Subtitles, scene: Moon } as const;

export function SuggestionList({ suggestions, onPick, className }: { suggestions: Suggestion[]; onPick: (suggestion: Suggestion) => void; className?: string }) {
  return (
    <ul className={cn("grid gap-1.5", className)}>
      {suggestions.map((suggestion) => {
        const Icon = SUGGESTION_ICONS[suggestion.icon];
        return (
          <li key={suggestion.id}>
            <button
              type="button"
              onClick={() => onPick(suggestion)}
              className="flex w-full items-center gap-3 rounded-tile border border-line bg-surface px-3.5 py-3 text-start text-base text-fg outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
            >
              <Icon className="size-4 shrink-0 text-icon" aria-hidden />
              <span className="min-w-0 flex-1">{suggestion.text}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function Typing() {
  return (
    <span className="inline-flex items-center gap-1 py-1" aria-hidden>
      {[0, 1, 2].map((dot) => (
        <span key={dot} className="size-1.5 animate-pulse rounded-full bg-fg-subtle" style={{ animationDelay: `${dot * 150}ms` }} />
      ))}
    </span>
  );
}

interface ConversationProps {
  messages: AssistantMessage[];
  runs: Map<string, AssistantRun>;
  /** Full page: results live in the result pane; selecting a run shows it there. */
  selectedRunId?: string | null;
  onSelectRun?: (runId: string) => void;
  /** Panel: results are shown inline under the reply. */
  inlineResults?: boolean;
  onRunAction?: (run: AssistantRun, action: BlockAction) => void;
  onBackground?: (run: AssistantRun) => void;
  deciding?: boolean;
  empty?: ReactNode;
}

export function Conversation({ messages, runs, selectedRunId, onSelectRun, inlineResults, onRunAction, onBackground, deciding, empty }: ConversationProps) {
  const { t } = useTranslation();
  const endRef = useRef<HTMLDivElement>(null);
  const lastRun = messages.at(-1)?.runId ? runs.get(messages.at(-1)?.runId ?? "") : undefined;

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end", behavior: "smooth" });
  }, [messages.length, lastRun?.steps.length, lastRun?.state]);

  if (messages.length === 0) return <>{empty}</>;

  return (
    <div className="grid gap-[1.375rem]">
      {messages.map((message) => {
        if (message.role === "user") {
          return (
            <p key={message.id} className="max-w-[85%] justify-self-end rounded-[1.125rem_1.125rem_0.375rem_1.125rem] bg-sunken px-3.5 py-2.5 text-base leading-relaxed text-fg">
              {message.text}
            </p>
          );
        }
        const run = message.runId ? runs.get(message.runId) : undefined;
        const hasResult = run ? run.blocks.length > 0 : false;
        const selected = run && selectedRunId === run.id;
        return (
          <div key={message.id} className="grid gap-3">
            <p className="flex items-center gap-2 text-sm font-strong text-fg-muted">
              <span className="grid size-6 place-items-center rounded-full bg-sunken text-fg">
                <MessageSquareText className="size-3.5" aria-hidden />
              </span>
              {t("assistant.title")}
            </p>
            <p className="text-base leading-relaxed text-fg">{message.text}</p>
            {run ? <RunSteps run={run} /> : null}
            {run?.state === "running" && run.steps.every((step) => step.state !== "running") ? <Typing /> : null}
            {run && hasResult && inlineResults ? (
              <div className="rounded-tile border border-line p-3">
                <BlockRenderer
                  key={`${run.id}-${run.state}`}
                  blocks={run.blocks}
                  readOnly={!isRunActive(run)}
                  busy={deciding}
                  onAction={(action) => onRunAction?.(run, action)}
                />
              </div>
            ) : null}
            {run && (hasResult || isRunActive(run)) ? (
              <div className="flex flex-wrap items-center gap-2">
                {hasResult && onSelectRun ? (
                  <Button size="sm" variant={selected ? "secondary" : "outline"} onPress={() => onSelectRun(run.id)} className="lg:hidden">
                    <PanelRightOpen aria-hidden />
                    {run.state === "awaiting_approval" ? t("assistant.reviewChanges") : t("assistant.showResult")}
                  </Button>
                ) : null}
                {isRunActive(run) && !run.taskId && onBackground && inlineResults ? (
                  <Button size="sm" variant="secondary" onPress={() => onBackground(run)}>
                    {t("assistant.toBackground")}
                  </Button>
                ) : null}
                {run.state === "awaiting_approval" && !run.taskId ? <span className="text-xs text-fg-muted">{t("assistant.staysInTasks")}</span> : null}
                {run.taskId ? <span className="text-xs text-fg-muted">{t("assistant.runningAsTask")}</span> : null}
              </div>
            ) : null}
          </div>
        );
      })}
      <div ref={endRef} />
    </div>
  );
}

export function Composer({
  onSend,
  disabled,
  contextLabel,
  onClearContext,
  initial,
  autoFocus,
  className,
}: {
  onSend: (text: string) => void;
  disabled?: boolean;
  contextLabel?: string | null;
  onClearContext?: () => void;
  initial?: string;
  autoFocus?: boolean;
  className?: string;
}) {
  const { t } = useTranslation();
  const [text, setText] = useState(initial ?? "");
  const submit = () => {
    const value = text.trim();
    if (!value || disabled) return;
    onSend(value);
    setText("");
  };
  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      submit();
    }
  };
  return (
    <div className={cn("grid gap-1.5 rounded-panel border border-line-control bg-field px-4 pt-3 pb-2.5 focus-within:border-focus", className)}>
      {contextLabel ? (
        <div className="flex">
          <span className="flex max-w-full items-center gap-1.5 rounded-chip bg-sunken py-0.5 ps-2.5 pe-1 text-xs text-fg">
            <span className="truncate">{contextLabel}</span>
            {onClearContext ? (
              <button type="button" onClick={onClearContext} className="rounded-full p-0.5 hover:bg-hover" aria-label={t("assistant.clearContext")}>
                <X className="size-3" aria-hidden />
              </button>
            ) : null}
          </span>
        </div>
      ) : null}
      <textarea
        value={text}
        autoFocus={autoFocus}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={onKeyDown}
        rows={2}
        placeholder={t("assistant.placeholder")}
        aria-label={t("assistant.placeholder")}
        className="field-sizing-content max-h-40 min-h-11 w-full resize-none bg-transparent text-base text-fg outline-none placeholder:text-fg-subtle max-md:min-h-6"
      />
      <div className="flex items-center gap-2">
        <span className="flex flex-1 items-center gap-1.5 text-xs text-fg-muted max-md:invisible">
          <Kbd>↵</Kbd>
          {t("assistant.sendHint")}
        </span>
        <Button isIconOnly size="sm" variant="primary" onPress={submit} isDisabled={disabled || !text.trim()} aria-label={t("assistant.send")}>
          <ArrowUp aria-hidden />
        </Button>
      </div>
    </div>
  );
}
