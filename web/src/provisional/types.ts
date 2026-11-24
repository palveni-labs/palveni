/**
 * PROVISIONAL API — extended API module.
 *
 * These endpoints exist only in the mock server (`npm run dev:mock`) so the assistant, unified
 * tasks, smart-home, storage, cover-art and live-metrics designs can be reviewed. Each group is
 * behind a feature flag the real server does not report yet ("assistant", "tasks", "home",
 * "storage", "artwork", "metrics"), so a real M0 backend never receives these requests. Expect
 * every shape here to change.
 */
import type { JobState, JobStep } from "@/api/types";
import type { Block, DeviceInfo, DevicePatch } from "@/blocks/types";

export const PROVISIONAL_FEATURES = [
  "assistant",
  "tasks",
  "home",
  "storage",
  "artwork",
  "metrics",
  "mcp",
  "downloads",
  "organizer",
] as const;

/* ---------------------------------------------------------------- unified tasks (feature "tasks") */

export type TaskKind = "library.scan" | "download" | "transcode" | "ai" | "assistant.run" | "automation";

export type TaskState = JobState | "awaiting_approval";

export interface TaskStep {
  id: string;
  label: string;
  state: "pending" | "running" | "done" | "failed" | "skipped";
  detail: string | null;
  at: string | null;
}

/** The work a task is about, so lists can show its cover and title. */
export interface TaskSubject {
  title: string;
  /** What is being done to it, when the kind alone does not say ("生成中文字幕"). */
  verb?: string | null;
  /** e.g. "2021", or the library name. */
  detail: string | null;
  artworkId: string | null;
}

export interface Task {
  id: string;
  kind: TaskKind | (string & {});
  title: string;
  state: TaskState;
  progress: number | null;
  message: string | null;
  error: string | null;
  createdAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  steps: TaskStep[];
  /** Result blocks (and, while awaiting approval, the change preview + approval). */
  result: Block[];
  /** Where the task came from, e.g. an assistant conversation or an automation rule. */
  origin: { module: string; label: string } | null;
  subject?: TaskSubject | null;
  /** A measured rate, e.g. "4.2×" or "12.4 MB/s". */
  rate?: string | null;
  /** Estimated seconds left; null when it cannot be estimated. */
  etaSeconds?: number | null;
  /** Covers of the items an approval would change (shown next to it). */
  previews?: string[];
  /** Library the task belongs to, for the second line of attention rows. */
  scope?: string | null;
  /** For tasks that mirror an M0 job: its step, localised by the client. */
  step?: JobStep | null;
  /** The library a scan task scans. */
  libraryId?: string | null;
}

export interface TaskList {
  tasks: Task[];
}

export interface ApproveRequest {
  selectedRowIds: string[];
}

/* ---------------------------------------------------------------- assistant (feature "assistant") */

export interface RunStep {
  id: string;
  /** Plain-language progress, e.g. "正在搜索音乐库… 找到 132 首". Never tool-call JSON. */
  text: string;
  /** A second line in plain language (how it was done, what was found). */
  detail?: string;
  state: "running" | "done" | "failed";
}

export type RunState = "running" | "awaiting_approval" | "succeeded" | "failed" | "cancelled";

export interface AssistantRun {
  id: string;
  /** What the run is doing, in a few words ("整理「家庭相册」的重复照片"). */
  title?: string;
  /** The library or work the run is about (its cover colours the result in work-colour skins). */
  subject?: { title: string; artworkId: string | null } | null;
  /** While awaiting approval: what it asks to do, in one line ("建议移除 21 张重复照片，可释放 186 MB"). */
  request?: string;
  /** Covers of what the approval would change. */
  previews?: string[];
  state: RunState;
  steps: RunStep[];
  blocks: Block[];
  /** Set when the run was sent to the background and continues as a task. */
  taskId: string | null;
  createdAt: string;
}

export interface AssistantMessage {
  id: string;
  role: "user" | "assistant";
  text: string;
  createdAt: string;
  /** Assistant messages produced by a run point at it. */
  runId: string | null;
}

export interface Conversation {
  id: string;
  messages: AssistantMessage[];
  runs: AssistantRun[];
}

export interface AssistantContext {
  /** Current page path, so the assistant knows what "this" refers to. */
  page: string;
  /** Items the user handed over (e.g. a selection in a library). */
  itemIds?: string[];
  label?: string;
}

export interface SendMessageRequest {
  text: string;
  context?: AssistantContext;
}

export interface SendMessageResponse {
  message: AssistantMessage;
  reply: AssistantMessage;
  run: AssistantRun;
}

export interface Suggestion {
  id: string;
  text: string;
  prompt: string;
  /** Names the action, e.g. "生成歌单". */
  actionLabel?: string;
  icon: "duplicate" | "storage" | "music" | "subtitle" | "scene";
}

export interface AssistantPresetInfo {
  id: string;
  label: string;
  defaultBaseUrl: string;
  defaultModel: string;
  requiresApiKey: boolean;
}

export interface AssistantConfigResponse {
  preset: string;
  baseUrl: string;
  model: string;
  apiKeyConfigured: boolean;
  redactCloudPaths: boolean;
  presets: AssistantPresetInfo[];
}

export interface UpdateAssistantConfigRequest {
  preset: string;
  baseUrl: string;
  model: string;
  apiKey?: string;
  clearApiKey?: boolean;
  redactCloudPaths?: boolean;
}

export interface AssistantMemory {
  id: string;
  content: string;
  createdAt: string;
}

/* ---------------------------------------------------------------- smart home (feature "home") */

export interface Room {
  id: string;
  name: string;
}

export interface HomeDevice extends DeviceInfo {
  roomId: string;
}

export interface Scene {
  id: string;
  name: string;
  icon: "movie" | "home" | "night" | "morning" | "away";
  active: boolean;
}

/** "影音联动": playback events driving the home. */
export interface AutomationRule {
  id: string;
  trigger: string;
  action: string;
  enabled: boolean;
  lastRunAt: string | null;
}

export interface HomeSnapshot {
  rooms: Room[];
  devices: HomeDevice[];
  scenes: Scene[];
  rules: AutomationRule[];
}

export type DeviceUpdateRequest = DevicePatch;

export interface HomeBridgeSummary {
  id: string;
  name: string;
  baseUrl: string;
  enabled: boolean;
  tokenConfigured: boolean;
  lastSyncAt: string | null;
}

export interface SaveHomeBridgeRequest {
  id?: string;
  name: string;
  baseUrl: string;
  token?: string;
  enabled?: boolean;
}

export interface HomeBridgeSyncResult {
  bridge: HomeBridgeSummary;
  syncedDevices: number;
  syncedScenes: number;
  snapshot: HomeSnapshot;
}

export type HomeDeviceKind = "light" | "switch" | "climate" | "sensor" | "media_player";

export interface CreateHomeDeviceRequest {
  roomName: string;
  name: string;
  kind: HomeDeviceKind;
}

export interface CreateHomeRuleRequest {
  eventKind?: string;
  trigger: string;
  action: string;
  sceneId?: string;
  deviceId?: string;
  patch?: DevicePatch;
}

/* ---------------------------------------------------------------- storage (feature "storage") */

export interface StorageVolume {
  id: string;
  label: string;
  path: string;
  totalBytes: number;
  usedBytes: number;
}

/** A network share or external disk libraries live on. */
export interface StorageMount {
  id: string;
  label: string;
  state: "online" | "offline";
  /** Libraries whose scans pause while the mount is offline. */
  affectedLibraries: number;
  since: string;
}

export interface StorageReport {
  volumes: StorageVolume[];
  byLibrary: Array<{ libraryId: string; name: string; bytes: number }>;
  mounts: StorageMount[];
}

/* ---------------------------------------------------------------- cover art and recent works (feature "artwork") */

export type WorkKind = "movie" | "series" | "album" | "book" | "photo";

export interface CatalogWork {
  id: string;
  title: string;
  kind: WorkKind;
  /** Year, artist or season. */
  meta: string | null;
  artworkId: string | null;
  libraryId: string | null;
  addedAt: string;
}

export interface Catalog {
  /** Most recently added works, newest first. */
  recent: CatalogWork[];
  /** A few covers per library (library id → artwork ids), for library thumbnails. */
  libraryCovers: Record<string, string[]>;
}

/* ---------------------------------------------------------------- live metrics (feature "metrics") */

export interface SystemMetrics {
  /** 0–1 over the last minute. */
  cpu: number;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  /** Devices currently connected to the server (apps and players). */
  devicesOnline: number;
}

/* ---------------------------------------------------------------- MCP server (feature "mcp") */

export interface McpToolDescriptor {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  adminOnly: boolean;
  requiresApproval: boolean;
}

export interface McpClientConfigs {
  claudeDesktop: string;
  cursor: string;
  openclaw: string;
}

export interface McpInfoResponse {
  protocolVersion: string;
  endpointPath: string;
  serverName: string;
  serverVersion: string;
  tools: McpToolDescriptor[];
  clientConfigs: McpClientConfigs;
}

/* ---------------------------------------------------------------- downloads & subscriptions (feature "downloads") */

export type DownloaderKind = "qbittorrent" | "transmission" | "aria2";
export type DownloaderClientStatus = "online" | "offline" | "disabled";

export interface DownloaderClientSummary {
  id: string;
  name: string;
  kind: DownloaderKind;
  baseUrl: string;
  savePath: string | null;
  enabled: boolean;
  credentialConfigured: boolean;
  status: DownloaderClientStatus;
  statusMessage?: string | null;
  createdAt: string;
}

export interface CreateDownloaderClientRequest {
  id?: string;
  name: string;
  kind: DownloaderKind;
  baseUrl: string;
  username?: string;
  password?: string;
  savePath?: string;
  enabled?: boolean;
}

export type TransferState = "downloading" | "seeding" | "paused" | "queued" | "completed" | "error";

export interface DownloadTransfer {
  id: string;
  clientId: string;
  clientName: string;
  clientKind: DownloaderKind;
  name: string;
  state: TransferState;
  progress: number;
  sizeBytes: number;
  downloadedBytes: number;
  downloadSpeedBps: number;
  uploadSpeedBps: number;
  etaSeconds: number | null;
  savePath: string | null;
}

export interface CreateDownloadTaskRequest {
  clientId?: string;
  uri: string;
  savePath?: string;
  category?: string;
}

export interface CreateDownloadTaskResponse {
  accepted: boolean;
  clientId: string;
  clientName: string;
  message: string;
}

export type SubscriptionMediaKind = "series" | "movie" | "music" | "book";

export interface MediaSubscription {
  id: string;
  ownerId: string;
  title: string;
  mediaKind: SubscriptionMediaKind;
  qualityRule: string | null;
  feedUrl: string | null;
  targetLibraryId: string | null;
  enabled: boolean;
  lastCheckedAt: string | null;
  createdAt: string;
}

export interface CreateSubscriptionRequest {
  title: string;
  mediaKind: SubscriptionMediaKind;
  qualityRule?: string;
  feedUrl?: string;
  targetLibraryId?: string;
  enabled?: boolean;
}

export interface FeedMatchItem {
  title: string;
  link: string | null;
  pubDate: string | null;
  matchedQuality: boolean;
}

export interface SubscriptionCheckResult {
  subscription: MediaSubscription;
  totalFeedItems: number;
  matches: FeedMatchItem[];
  submittedCount: number;
}

export interface DownloadsOverviewResponse {
  clients: DownloaderClientSummary[];
  transfers: DownloadTransfer[];
  subscriptions: MediaSubscription[];
}

/* ---------------------------------------------------------------- media organizer (feature "organizer") */

export type OrganizeMode = "rename" | "hardlink";

export interface OrganizePreset {
  id: string;
  label: string;
  mediaKind: string;
  template: string;
  example: string;
}

export interface OrganizePreviewRequest {
  libraryId: string;
  mode?: OrganizeMode;
  template: string;
  targetSubdir?: string;
  limit?: number;
}

export interface OrganizeCandidate {
  rowId: string;
  itemId: string;
  sourcePath: string;
  targetPath: string;
  relativeBefore: string;
  relativeAfter: string;
  sizeBytes: number | null;
  conflict?: string | null;
  selectable: boolean;
}

export interface OrganizePreviewResponse {
  libraryId: string;
  libraryName: string;
  mode: OrganizeMode;
  template: string;
  readOnlyMount: boolean;
  collisionCount: number;
  readyCount: number;
  candidates: OrganizeCandidate[];
  previewBlock: Block;
  presets: OrganizePreset[];
}

export interface OrganizeApplyRequest {
  libraryId: string;
  mode: OrganizeMode;
  template: string;
  targetSubdir?: string;
  selectedItemIds: string[];
}

export interface OrganizeOperationRecord {
  itemId: string;
  fromPath: string;
  toPath: string;
  createdDirs: string[];
}

export interface OrganizeBatchSummary {
  id: string;
  actorId: string;
  libraryId: string;
  libraryName: string | null;
  mode: OrganizeMode;
  template: string;
  operationCount: number;
  operations: OrganizeOperationRecord[];
  undoneAt: string | null;
  createdAt: string;
}

export interface OrganizeBatchList {
  batches: OrganizeBatchSummary[];
  presets: OrganizePreset[];
}

/* ---------------------------------------------------------------- events on GET /events */

export interface ProvisionalEventMap {
  "task.updated": Task;
  "assistant.run.updated": AssistantRun;
  "home.device.updated": HomeDevice;
}
