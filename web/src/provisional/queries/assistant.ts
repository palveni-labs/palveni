import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { provisionalApi } from "../api";
import { provisionalKeys } from "../keys";
import type {
  AssistantConfigResponse,
  AssistantContext,
  AssistantMemory,
  AssistantRun,
  Conversation,
  UpdateAssistantConfigRequest,
} from "../types";
import { upsertTask } from "./tasks";

export function useConversation(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.conversation,
    queryFn: ({ signal }) => provisionalApi.assistant.conversation(signal),
    enabled,
    staleTime: Infinity,
  });
}

export function useSuggestions(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.suggestions,
    queryFn: ({ signal }) => provisionalApi.assistant.suggestions(signal),
    enabled,
    staleTime: 60_000,
  });
}

/** Applies an `assistant.run.updated` event to the cached conversation. */
export function upsertRun(queryClient: QueryClient, run: AssistantRun) {
  queryClient.setQueryData<Conversation>(provisionalKeys.conversation, (conversation) => {
    if (!conversation) return conversation;
    const exists = conversation.runs.some((entry) => entry.id === run.id);
    return {
      ...conversation,
      runs: exists ? conversation.runs.map((entry) => (entry.id === run.id ? run : entry)) : [...conversation.runs, run],
    };
  });
}

export function useSendMessage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ text, context }: { text: string; context?: AssistantContext }) =>
      provisionalApi.assistant.send({ text, context }),
    onSuccess: ({ message, reply, run }) => {
      queryClient.setQueryData<Conversation>(provisionalKeys.conversation, (conversation) =>
        conversation
          ? {
              ...conversation,
              messages: [...conversation.messages, message, reply],
              runs: conversation.runs.some((entry) => entry.id === run.id) ? conversation.runs : [...conversation.runs, run],
            }
          : conversation,
      );
    },
  });
}

export function useRunActions() {
  const queryClient = useQueryClient();
  const approve = useMutation({
    mutationFn: ({ runId, selectedRowIds }: { runId: string; selectedRowIds: string[] }) =>
      provisionalApi.assistant.approve(runId, { selectedRowIds }),
    onSuccess: (run) => upsertRun(queryClient, run),
  });
  const cancel = useMutation({
    mutationFn: (runId: string) => provisionalApi.assistant.cancel(runId),
    onSuccess: (run) => upsertRun(queryClient, run),
  });
  const background = useMutation({
    mutationFn: (runId: string) => provisionalApi.assistant.background(runId),
    onSuccess: ({ run, task }) => {
      upsertRun(queryClient, run);
      upsertTask(queryClient, task);
    },
  });
  return { approve, cancel, background };
}

export function useResetConversation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => provisionalApi.assistant.reset(),
    onSuccess: (conversation) => queryClient.setQueryData(provisionalKeys.conversation, conversation),
  });
}

export function useAssistantConfig(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.assistantConfig,
    queryFn: ({ signal }) => provisionalApi.assistant.config(signal),
    enabled,
    staleTime: 60_000,
  });
}

export function useUpdateAssistantConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: UpdateAssistantConfigRequest) => provisionalApi.assistant.updateConfig(body),
    onSuccess: (config: AssistantConfigResponse) => queryClient.setQueryData(provisionalKeys.assistantConfig, config),
  });
}

export function useAssistantMemories(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.assistantMemories,
    queryFn: ({ signal }) => provisionalApi.assistant.memories(signal),
    enabled,
    staleTime: 30_000,
  });
}

export function useAddAssistantMemory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (content: string) => provisionalApi.assistant.addMemory(content),
    onSuccess: (memory: AssistantMemory) =>
      queryClient.setQueryData<{ memories: AssistantMemory[] }>(provisionalKeys.assistantMemories, (prev) => ({
        memories: [memory, ...(prev?.memories ?? [])],
      })),
  });
}

export function useDeleteAssistantMemory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.assistant.deleteMemory(id),
    onSuccess: (_res, id) =>
      queryClient.setQueryData<{ memories: AssistantMemory[] }>(provisionalKeys.assistantMemories, (prev) =>
        prev ? { memories: prev.memories.filter((entry) => entry.id !== id) } : prev,
      ),
  });
}

export function useMcpInfo(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.mcpInfo,
    queryFn: ({ signal }) => provisionalApi.mcp.info(signal),
    enabled,
    staleTime: 60_000,
  });
}
