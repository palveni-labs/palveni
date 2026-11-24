import { Button, buttonVariants } from "@heroui/react";
import { Link, useRouterState } from "@tanstack/react-router";
import { Maximize2, MessageSquareText, X } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSuggestions } from "@/provisional/queries/assistant";
import { shell, useShellState, type AssistantHandoff } from "@/shell/shell-store";
import { Composer, Conversation, SuggestionList } from "./conversation";
import { usePageContextLabel } from "./page-context";
import { useAssistant } from "./use-assistant";

/** Handoffs already sent (survives StrictMode's double effect run). */
const consumed = new WeakSet<AssistantHandoff>();

/** Compact assistant (⌘J) next to any page; knows the current page and any handed-over items. */
export default function AssistantPanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const assistant = useAssistant();
  const suggestions = useSuggestions(true);
  const { handoff } = useShellState();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const pageLabel = usePageContextLabel();
  const contextLabel = handoff?.label || (pathname === "/assistant" ? "" : t("assistant.onPage", { page: pageLabel }));
  const { sendMessage } = assistant;

  const send = (text: string) => {
    sendMessage(text, { page: pathname, label: handoff?.label || pageLabel, itemIds: handoff?.itemIds });
    shell.clearHandoff();
  };

  // Suggestions and ⌘K "ask" hand over a prompt that should go out immediately.
  useEffect(() => {
    if (!handoff?.autoSend || !handoff.prompt || consumed.has(handoff)) return;
    consumed.add(handoff);
    sendMessage(handoff.prompt, { page: pathname, label: pageLabel });
    shell.clearHandoff();
  }, [handoff, sendMessage, pathname, pageLabel]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex items-center gap-2.5 border-b border-line px-4 py-3">
        <span className="grid size-8 place-items-center rounded-full bg-sunken text-fg">
          <MessageSquareText className="size-4" aria-hidden />
        </span>
        <div className="min-w-0 flex-1">
          <h2 className="text-base font-strong text-fg">{t("assistant.title")}</h2>
          <p className="truncate text-xs text-fg-muted">{pageLabel}</p>
        </div>
        <Link to="/assistant" aria-label={t("assistant.openFull")} onClick={onClose} className={buttonVariants({ variant: "ghost", size: "sm", isIconOnly: true })}>
          <Maximize2 aria-hidden />
        </Link>
        <Button variant="ghost" size="sm" isIconOnly onPress={onClose} aria-label={t("a11y.close")}>
          <X aria-hidden />
        </Button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        <Conversation
          messages={assistant.conversation?.messages ?? []}
          runs={assistant.runs}
          inlineResults
          onRunAction={assistant.onRunAction}
          onBackground={assistant.toBackground}
          deciding={assistant.deciding}
          empty={
            <div className="grid gap-3 pt-2">
              <p className="font-heading text-lg font-title text-heading">{t("assistant.greeting")}</p>
              <SuggestionList suggestions={(suggestions.data?.suggestions ?? []).slice(0, 3)} onPick={(suggestion) => send(suggestion.prompt)} />
            </div>
          }
        />
      </div>
      <div className="border-t border-line p-3">
        <Composer
          key={handoff?.prompt ?? "panel-composer"}
          initial={handoff?.autoSend ? undefined : handoff?.prompt}
          onSend={send}
          disabled={assistant.sending}
          contextLabel={contextLabel || null}
          onClearContext={handoff ? () => shell.clearHandoff() : undefined}
          autoFocus
        />
      </div>
    </div>
  );
}
