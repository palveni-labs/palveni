import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useFeature } from "@/modules/access";
import type { InboxItem } from "@/modules/types";
import { useConversation, useRunActions } from "@/provisional/queries/assistant";
import type { AssistantRun } from "@/provisional/types";

export function runSelection(run: AssistantRun): string[] {
  return run.blocks.flatMap((block) => (block.type === "change-preview" ? block.rows.filter((row) => row.selected).map((row) => row.id) : []));
}

/** PROVISIONAL: assistant runs waiting for a decision (not yet moved to the background). */
export function useAssistantInbox(): InboxItem[] {
  const { t } = useTranslation();
  const enabled = useFeature("assistant");
  const { data } = useConversation(enabled);
  const { approve, cancel } = useRunActions();
  const busy = approve.isPending ? approve.variables.runId : cancel.isPending ? cancel.variables : null;
  return useMemo(
    () =>
      (data?.runs ?? [])
        .filter((run) => run.state === "awaiting_approval" && !run.taskId)
        .map(
          (run): InboxItem => ({
            id: `run:${run.id}`,
            kind: "approval",
            title: run.request ?? run.title ?? t("assistant.title"),
            createdAt: run.createdAt,
            tone: "warning",
            to: "/assistant",
            scope: run.subject?.title,
            previews: run.previews,
            actions: [
              { id: "dismiss", label: t("attention.dismiss"), variant: "ghost", run: () => cancel.mutate(run.id), busy: busy === run.id },
              { id: "review", label: t("attention.review"), variant: "outline", to: "/assistant" },
              { id: "confirm", label: t("attention.confirm"), variant: "confirm", run: () => approve.mutate({ runId: run.id, selectedRowIds: runSelection(run) }), busy: busy === run.id },
            ],
          }),
        ),
    [data, t, approve, cancel, busy],
  );
}
