/** PROVISIONAL endpoints (mock server only). See ./types.ts. */
import { request, requestBlob } from "@/api/client";
import type { DevicePatch } from "@/blocks/types";
import type {
  ApproveRequest,
  AssistantConfigResponse,
  AssistantMemory,
  AssistantRun,
  AutomationRule,
  Catalog,
  Conversation,
  CreateDownloaderClientRequest,
  CreateDownloadTaskRequest,
  CreateDownloadTaskResponse,
  CreateHomeDeviceRequest,
  CreateHomeRuleRequest,
  CreateSubscriptionRequest,
  DownloaderClientSummary,
  DownloadsOverviewResponse,
  HomeBridgeSummary,
  HomeBridgeSyncResult,
  HomeDevice,
  HomeSnapshot,
  McpInfoResponse,
  MediaSubscription,
  OrganizeApplyRequest,
  OrganizeBatchList,
  OrganizeBatchSummary,
  OrganizePreviewRequest,
  OrganizePreviewResponse,
  SaveHomeBridgeRequest,
  SendMessageRequest,
  SendMessageResponse,
  StorageMount,
  StorageReport,
  SubscriptionCheckResult,
  Suggestion,
  SystemMetrics,
  Task,
  TaskList,
  UpdateAssistantConfigRequest,
} from "./types";

const enc = encodeURIComponent;

export const provisionalApi = {
  tasks: {
    list: (signal?: AbortSignal) => request<TaskList>("/tasks", { signal, query: { limit: 100 } }),
    approve: (id: string, body: ApproveRequest) => request<Task>(`/tasks/${enc(id)}/approve`, { method: "POST", body }),
    cancel: (id: string) => request<Task>(`/tasks/${enc(id)}/cancel`, { method: "POST" }),
  },
  assistant: {
    conversation: (signal?: AbortSignal) => request<Conversation>("/assistant/conversation", { signal }),
    send: (body: SendMessageRequest) => request<SendMessageResponse>("/assistant/messages", { method: "POST", body }),
    approve: (runId: string, body: ApproveRequest) =>
      request<AssistantRun>(`/assistant/runs/${enc(runId)}/approve`, { method: "POST", body }),
    cancel: (runId: string) => request<AssistantRun>(`/assistant/runs/${enc(runId)}/cancel`, { method: "POST" }),
    background: (runId: string) =>
      request<{ run: AssistantRun; task: Task }>(`/assistant/runs/${enc(runId)}/background`, { method: "POST" }),
    reset: () => request<Conversation>("/assistant/conversation/reset", { method: "POST" }),
    suggestions: (signal?: AbortSignal) => request<{ suggestions: Suggestion[] }>("/assistant/suggestions", { signal }),
    config: (signal?: AbortSignal) => request<AssistantConfigResponse>("/assistant/config", { signal }),
    updateConfig: (body: UpdateAssistantConfigRequest) =>
      request<AssistantConfigResponse>("/assistant/config", { method: "PUT", body }),
    memories: (signal?: AbortSignal) => request<{ memories: AssistantMemory[] }>("/assistant/memories", { signal }),
    addMemory: (content: string) => request<AssistantMemory>("/assistant/memories", { method: "POST", body: { content } }),
    deleteMemory: (id: string) => request<void>(`/assistant/memories/${enc(id)}`, { method: "DELETE" }),
  },
  home: {
    snapshot: (signal?: AbortSignal) => request<HomeSnapshot>("/home", { signal }),
    updateDevice: (id: string, patch: DevicePatch) => request<HomeDevice>(`/home/devices/${enc(id)}`, { method: "POST", body: patch }),
    createDevice: (body: CreateHomeDeviceRequest) => request<HomeDevice>("/home/devices", { method: "POST", body }),
    deleteDevice: (id: string) => request<void>(`/home/devices/${enc(id)}`, { method: "DELETE" }),
    activateScene: (id: string) => request<HomeSnapshot>(`/home/scenes/${enc(id)}/activate`, { method: "POST" }),
    setRule: (id: string, enabled: boolean) =>
      request<AutomationRule>(`/home/rules/${enc(id)}`, { method: "POST", body: { enabled } }),
    createRule: (body: CreateHomeRuleRequest) => request<AutomationRule>("/home/rules", { method: "POST", body }),
    deleteRule: (id: string) => request<void>(`/home/rules/${enc(id)}`, { method: "DELETE" }),
    bridges: (signal?: AbortSignal) => request<{ bridges: HomeBridgeSummary[] }>("/home/bridges", { signal }),
    saveBridge: (body: SaveHomeBridgeRequest) => request<HomeBridgeSummary>("/home/bridges", { method: "POST", body }),
    deleteBridge: (id: string) => request<void>(`/home/bridges/${enc(id)}`, { method: "DELETE" }),
    syncBridge: (id: string) => request<HomeBridgeSyncResult>(`/home/bridges/${enc(id)}/sync`, { method: "POST" }),
  },
  storage: {
    report: (signal?: AbortSignal) => request<StorageReport>("/system/storage", { signal }),
    reconnect: (mountId: string) => request<StorageMount>(`/system/storage/mounts/${enc(mountId)}/reconnect`, { method: "POST" }),
  },
  artwork: {
    catalog: (signal?: AbortSignal) => request<Catalog>("/catalog", { signal }),
    image: (artworkId: string, signal?: AbortSignal) => requestBlob(`/artwork/${enc(artworkId)}`, { signal }),
  },
  metrics: {
    current: (signal?: AbortSignal) => request<SystemMetrics>("/system/metrics", { signal }),
  },
  mcp: {
    info: (signal?: AbortSignal) => request<McpInfoResponse>("/mcp/info", { signal }),
  },
  downloads: {
    overview: (signal?: AbortSignal) => request<DownloadsOverviewResponse>("/downloads", { signal }),
    saveClient: (body: CreateDownloaderClientRequest) =>
      request<DownloaderClientSummary>("/downloads/clients", { method: "POST", body }),
    deleteClient: (id: string) => request<void>(`/downloads/clients/${enc(id)}`, { method: "DELETE" }),
    createTask: (body: CreateDownloadTaskRequest) =>
      request<CreateDownloadTaskResponse>("/downloads/tasks", { method: "POST", body }),
    createSubscription: (body: CreateSubscriptionRequest) =>
      request<MediaSubscription>("/downloads/subscriptions", { method: "POST", body }),
    deleteSubscription: (id: string) => request<void>(`/downloads/subscriptions/${enc(id)}`, { method: "DELETE" }),
    checkSubscription: (id: string) =>
      request<SubscriptionCheckResult>(`/downloads/subscriptions/${enc(id)}/check`, { method: "POST" }),
  },
  organizer: {
    preview: (body: OrganizePreviewRequest) =>
      request<OrganizePreviewResponse>("/organizer/preview", { method: "POST", body }),
    apply: (body: OrganizeApplyRequest) =>
      request<OrganizeBatchSummary>("/organizer/apply", { method: "POST", body }),
    batches: (signal?: AbortSignal) => request<OrganizeBatchList>("/organizer/batches", { signal, query: { limit: 50 } }),
    undo: (id: string) => request<OrganizeBatchSummary>(`/organizer/batches/${enc(id)}/undo`, { method: "POST" }),
  },
};
