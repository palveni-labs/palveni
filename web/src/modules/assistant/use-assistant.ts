import { useTranslation } from "react-i18next";
import { toast } from "@heroui/react";
import type { BlockAction } from "@/blocks/types";
import { errorText } from "@/lib/errors";
import { useConversation, useRunActions, useSendMessage } from "@/provisional/queries/assistant";
import type { AssistantContext, AssistantRun } from "@/provisional/types";

/** Conversation state and actions shared by the full page and the ⌘J panel. */
export function useAssistant() {
  const { t } = useTranslation();
  const conversation = useConversation(true);
  const send = useSendMessage();
  const { approve, cancel, background } = useRunActions();
  const runs = new Map((conversation.data?.runs ?? []).map((run) => [run.id, run]));

  const sendMessage = (text: string, context?: AssistantContext) =>
    send.mutate({ text, context }, { onError: (error) => toast.danger(errorText(t, error)) });

  /** Maps a result block's intent to the run's (provisional) API. */
  const onRunAction = (run: AssistantRun, action: BlockAction) => {
    if (action.type === "approve") {
      approve.mutate({ runId: run.id, selectedRowIds: action.selectedRowIds }, { onError: (error) => toast.danger(errorText(t, error)) });
    } else if (action.type === "cancel") {
      cancel.mutate(run.id, { onError: (error) => toast.danger(errorText(t, error)) });
    }
  };

  const toBackground = (run: AssistantRun) =>
    background.mutate(run.id, {
      onSuccess: () => toast.success(t("assistant.backgrounded")),
      onError: (error) => toast.danger(errorText(t, error)),
    });

  return {
    conversation: conversation.data,
    isPending: conversation.isPending,
    runs,
    sendMessage,
    sending: send.isPending,
    onRunAction,
    toBackground,
    deciding: approve.isPending || cancel.isPending,
  };
}

export function isRunActive(run: AssistantRun | undefined): boolean {
  return run?.state === "running" || run?.state === "awaiting_approval";
}
