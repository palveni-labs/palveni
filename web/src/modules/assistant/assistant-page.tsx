import { Button, Skeleton } from "@heroui/react";
import { Settings2, SquarePen } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { formatDateTime } from "@/lib/format";
import { useResetConversation, useSuggestions } from "@/provisional/queries/assistant";
import { shell, useShellState } from "@/shell/shell-store";
import { useSlot } from "@/skins/scope";
import { Composer, Conversation, SuggestionList } from "./conversation";
import { RunResult } from "./result";
import { AssistantSettingsDrawer } from "./settings-drawer";
import { useAssistant } from "./use-assistant";

/**
 * 助手: the conversation on the left (steps in plain words, the composer at the bottom), the
 * result of the run on the right (summary, change preview, the confirmation request pinned in
 * view). Below the large breakpoint they stack in that order.
 */
export function AssistantPage() {
  const { t, i18n } = useTranslation();
  const assistant = useAssistant();
  const suggestions = useSuggestions(true);
  const reset = useResetConversation();
  const section = useSlot("section");
  const { handoff } = useShellState();
  const [selected, setSelected] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const messages = assistant.conversation?.messages ?? [];
  // Show the chosen run, else the newest run that produced something.
  const latestWithResult = [...messages]
    .reverse()
    .map((message) => (message.runId ? assistant.runs.get(message.runId) : undefined))
    .find((run) => run && (run.blocks.length > 0 || run.state === "running"));
  const run = (selected ? assistant.runs.get(selected) : undefined) ?? latestWithResult;
  const startedAt = messages[0]?.createdAt;

  const locale = i18n.resolvedLanguage ?? "en";
  const when = (iso: string) =>
    new Date(iso).toDateString() === new Date().toDateString()
      ? t("time.todayAt", { time: new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" }).format(new Date(iso)) })
      : formatDateTime(iso, locale);

  const send = (text: string) => {
    assistant.sendMessage(text, { page: "/assistant", label: handoff?.label || undefined, itemIds: handoff?.itemIds });
    shell.clearHandoff();
    setSelected(null);
  };

  return (
    <div className="pv-enter pv-page-frame mx-auto flex min-h-0 w-full max-w-(--pv-layout-max) flex-1 flex-col px-(--pv-layout-gutter) pt-(--pv-layout-top) pb-5 lg:h-[calc(100svh-var(--pv-shell-h)-1.25rem)] lg:flex-none">
      <div className="pv-glass-panel grid min-h-0 flex-1 overflow-hidden rounded-panel border lg:grid-cols-[clamp(22rem,36%,30rem)_minmax(0,1fr)] lg:grid-rows-[minmax(0,1fr)]">
      <section aria-label={t("assistant.views.chat")} className="flex min-h-0 flex-col border-line lg:border-e">
        <header className="flex items-center gap-2 border-b border-line/60 px-5 pt-4 pb-3 max-md:px-4">
          <div className="min-w-0 flex-1">
            <h1 className="truncate text-lg font-title text-heading">{run?.title ?? t("assistant.title")}</h1>
            {startedAt ? <p className="text-xs text-fg-muted">{when(startedAt)}</p> : null}
          </div>
          <Button size="sm" variant="ghost" onPress={() => setSettingsOpen(true)}>
            <Settings2 className="size-4" aria-hidden />
            {t("assistant.settings.open")}
          </Button>
          {messages.length > 0 ? (
            <Button isIconOnly variant="ghost" onPress={() => reset.mutate()} isDisabled={reset.isPending} aria-label={t("assistant.newConversation")}>
              <SquarePen aria-hidden />
            </Button>
          ) : null}
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 pt-3 pb-4 max-md:px-4">
          {assistant.isPending ? (
            <div className="grid gap-3">
              <Skeleton className="h-12 rounded-tile" />
              <Skeleton className="h-28 rounded-tile" />
            </div>
          ) : (
            <Conversation
              messages={messages}
              runs={assistant.runs}
              selectedRunId={run?.id ?? null}
              onSelectRun={(id) => {
                setSelected(id);
                document.getElementById("assistant-result")?.scrollIntoView({ behavior: "smooth", block: "start" });
              }}
              onBackground={assistant.toBackground}
              deciding={assistant.deciding}
              empty={
                <div className="grid gap-4 pt-2">
                  <p className="font-heading text-xl font-title text-heading">{t("assistant.greeting")}</p>
                  <p className="text-sm text-fg-muted">{t("assistant.subtitle")}</p>
                  <SuggestionList suggestions={suggestions.data?.suggestions ?? []} onPick={(suggestion) => send(suggestion.prompt)} />
                </div>
              }
            />
          )}
        </div>
        <div className="px-5 pb-4 max-lg:hidden">
          <Composer
            key={handoff?.prompt ?? "composer"}
            initial={handoff?.prompt}
            onSend={send}
            disabled={assistant.sending}
            contextLabel={handoff?.label || null}
            onClearContext={() => shell.clearHandoff()}
          />
        </div>
      </section>
      <section
        id="assistant-result"
        aria-label={t("assistant.views.canvas")}
        className={cn("min-h-0 bg-sunken/30 max-lg:border-t max-lg:border-line", section === "panel" ? "p-4 max-md:p-3 lg:p-5" : "")}
      >
        <RunResult run={run} onAction={assistant.onRunAction} onBackground={assistant.toBackground} deciding={assistant.deciding} />
      </section>
      </div>
      <div className="sticky bottom-0 z-10 mt-3 border-t border-line bg-page p-3 lg:hidden">
        <Composer key={handoff?.prompt ?? "composer-phone"} initial={handoff?.prompt} onSend={send} disabled={assistant.sending} contextLabel={handoff?.label || null} onClearContext={() => shell.clearHandoff()} />
      </div>
      <AssistantSettingsDrawer open={settingsOpen} onOpenChange={setSettingsOpen} />
    </div>
  );
}
