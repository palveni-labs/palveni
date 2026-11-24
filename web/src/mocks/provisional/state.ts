/**
 * In-memory state for the PROVISIONAL mock APIs. Library scans started through the M0 mock
 * also appear here as tasks, so the unified task list covers everything.
 */
import type { Block, DevicePatch } from "@/blocks/types";
import type { Job } from "@/api/types";
import type {
  AssistantConfigResponse,
  AssistantContext,
  AssistantMemory,
  AssistantMessage,
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
  DownloadTransfer,
  HomeBridgeSummary,
  HomeBridgeSyncResult,
  HomeDevice,
  HomeSnapshot,
  McpInfoResponse,
  MediaSubscription,
  OrganizeApplyRequest,
  OrganizeBatchSummary,
  OrganizeCandidate,
  OrganizePreset,
  OrganizePreviewRequest,
  OrganizePreviewResponse,
  RunStep,
  SaveHomeBridgeRequest,
  StorageMount,
  StorageReport,
  SubscriptionCheckResult,
  SystemMetrics,
  Task,
  TaskStep,
  UpdateAssistantConfigRequest,
} from "@/provisional/types";
import type { MockDb } from "../db";
import { stableUuid } from "../fixtures";
import { pickScript, type ScriptOutcome, type ScriptStep } from "./scripts";
import { SkinRegistryState } from "./skins";
import { seedCatalog, seedHome, seedMetrics, seedStorage, seedTasks, SUGGESTIONS } from "./seed";

let counter = 0;

/** Seed library keys → names (as in mocks/db.ts). */
const LIBRARY_NAMES: Record<string, string> = { movies: "电影", series: "剧集", music: "无损音乐", photos: "家庭相册", books: "电子书", comics: "漫画" };
const nextId = (prefix: string) => `${prefix}-${Date.now().toString(36)}-${(counter++).toString(36)}`;

export class ProvisionalState {
  tasks: Task[];
  home: HomeSnapshot;
  storage: StorageReport;
  catalog: Catalog;
  metrics: SystemMetrics = seedMetrics();
  suggestions = SUGGESTIONS;
  conversation: Conversation = { id: "c1", messages: [], runs: [] };
  assistantConfig: AssistantConfigResponse = {
    preset: "builtin",
    baseUrl: "",
    model: "builtin-planner",
    apiKeyConfigured: false,
    redactCloudPaths: true,
    presets: [
      { id: "builtin", label: "内置本地规划器（免密钥）", defaultBaseUrl: "", defaultModel: "builtin-planner", requiresApiKey: false },
      { id: "ollama", label: "Ollama（局域网 / 本机）", defaultBaseUrl: "http://127.0.0.1:11434/v1", defaultModel: "qwen2.5:7b", requiresApiKey: false },
      { id: "deepseek", label: "DeepSeek", defaultBaseUrl: "https://api.deepseek.com/v1", defaultModel: "deepseek-chat", requiresApiKey: true },
      { id: "qwen", label: "通义千问（Qwen）", defaultBaseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1", defaultModel: "qwen-plus", requiresApiKey: true },
      { id: "openai", label: "OpenAI", defaultBaseUrl: "https://api.openai.com/v1", defaultModel: "gpt-4o-mini", requiresApiKey: true },
      { id: "anthropic", label: "Anthropic", defaultBaseUrl: "https://api.anthropic.com/v1", defaultModel: "claude-3-5-sonnet-latest", requiresApiKey: true },
      { id: "gemini", label: "Google Gemini", defaultBaseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", defaultModel: "gemini-2.5-flash", requiresApiKey: true },
      { id: "custom", label: "自定义 OpenAI 兼容接口", defaultBaseUrl: "", defaultModel: "", requiresApiKey: false },
    ],
  };
  memories: AssistantMemory[] = [];
  bridges: HomeBridgeSummary[] = [];
  downloaderClients: DownloaderClientSummary[] = [];
  transfers: DownloadTransfer[] = [];
  subscriptions: MediaSubscription[] = [];
  organizeBatches: OrganizeBatchSummary[] = [];
  readonly organizerPresets: OrganizePreset[] = [
    {
      id: "movie-year",
      label: "电影标准（片名 + 年份）",
      mediaKind: "video",
      template: "{title} ({year})",
      example: "沙丘2 (2024).mkv",
    },
    {
      id: "series-season",
      label: "剧集分季（剧名 / Season / SxxExx）",
      mediaKind: "video",
      template: "{title}/Season {season}/{title} - S{season}E{episode}",
      example: "三体/Season 01/三体 - S01E01.mkv",
    },
    {
      id: "music-artist-title",
      label: "音乐曲目（艺人 - 标题）",
      mediaKind: "music",
      template: "{artist} - {title}",
      example: "周杰伦 - 晴天 Live.flac",
    },
    {
      id: "book-author-title",
      label: "电子书标准（作者 - 书名）",
      mediaKind: "book",
      template: "{author} - {title}",
      example: "刘慈欣 - 三体.epub",
    },
  ];
  private continuations = new Map<string, ScriptOutcome["approve"]>();
  private unsubscribe: () => void;
  private readonly db: MockDb;
  /** Multiplies every scripted delay (tests run the scripts fast). */
  private readonly timeScale: number;

  readonly skins: SkinRegistryState;

  constructor(
    db: MockDb,
    options: {
      simulate?: boolean;
      timeScale?: number;
      seedConversation?: boolean;
      /** Skin plugins installed at start, and where to remember changes. */
      installedSkins?: readonly string[];
      persistSkins?: (ids: string[]) => void;
    } = {},
  ) {
    this.db = db;
    this.timeScale = options.timeScale ?? 1;
    const iso = (offset = 0) => db.iso(offset);
    this.skins = new SkinRegistryState(() => db.iso(), options.installedSkins ?? [], options.persistSkins);
    this.storage = seedStorage(iso);
    this.catalog = seedCatalog(iso, (key) => db.libraries.find((library) => library.name === LIBRARY_NAMES[key])?.id ?? key);
    this.tasks = [...seedTasks((offset = 0) => db.iso(offset)), ...db.jobs.map((job) => this.jobTask(job))].sort((a, b) =>
      b.createdAt.localeCompare(a.createdAt),
    );
    this.home = seedHome((offset = 0) => db.iso(offset));
    this.downloaderClients = [
      {
        id: "dl-qb-1",
        name: "NAS qBittorrent",
        kind: "qbittorrent",
        baseUrl: "http://192.168.1.20:8080",
        savePath: "/downloads/media",
        enabled: true,
        credentialConfigured: true,
        status: "online",
        statusMessage: null,
        createdAt: iso(-3_600_000),
      },
    ];
    this.transfers = [
      {
        id: "tr-dune-4k",
        clientId: "dl-qb-1",
        clientName: "NAS qBittorrent",
        clientKind: "qbittorrent",
        name: "Dune.Part.Two.2024.2160p.WEB-DL.HEVC.mkv",
        state: "downloading",
        progress: 0.68,
        sizeBytes: 18_400_000_000,
        downloadedBytes: 12_512_000_000,
        downloadSpeedBps: 24_800_000,
        uploadSpeedBps: 1_200_000,
        etaSeconds: 237,
        savePath: "/downloads/media",
      },
    ];
    this.subscriptions = [
      {
        id: "sub-santi",
        ownerId: db.users[0]?.id ?? "u-admin",
        title: "三体",
        mediaKind: "series",
        qualityRule: "2160p HEVC",
        feedUrl: "https://example.com/rss/santi.xml",
        targetLibraryId: null,
        enabled: true,
        lastCheckedAt: null,
        createdAt: iso(-7_200_000),
      },
    ];
    // Seeded approval tasks need something to do once approved.
    this.continuations.set("t-covers", () => ({
      steps: [{ running: "正在写入封面…", done: "已写入 8 张封面", ms: 1500 }],
      blocks: [{ type: "text", text: "封面已写入，客户端下次同步时会显示。", tone: "success" }],
    }));
    this.unsubscribe = db.subscribe((event, data) => {
      if (event === "job.updated") this.upsertTask(this.jobTask(data as Job));
    });
    if (options.seedConversation) this.seedConversation();
    if (options.simulate ?? true) this.simulate();
  }

  /** The conversation the design comps show: a duplicate-photo run waiting for approval. */
  private seedConversation() {
    const outcome = pickScript({ text: "整理重复照片", db: this.db, activateScene: () => {} });
    const at = this.db.iso(-3 * 60_000);
    const run: AssistantRun = {
      id: "run-duplicates",
      title: outcome.title,
      subject: outcome.subject ?? null,
      request: outcome.request,
      previews: outcome.previews,
      state: "awaiting_approval",
      steps: outcome.steps.map((step, index) => ({ id: `run-duplicates-s${index}`, text: step.done, detail: step.detail, state: "done" })),
      blocks: outcome.blocks,
      taskId: null,
      createdAt: at,
    };
    this.conversation = {
      id: "c1",
      messages: [
        { id: "m-seed-1", role: "user", text: "帮我整理家庭相册里的重复照片", createdAt: at, runId: null },
        { id: "m-seed-2", role: "assistant", text: outcome.reply, createdAt: at, runId: run.id },
      ],
      runs: [run],
    };
    this.continuations.set(run.id, outcome.approve);
  }

  /* ------------------------------------------------------------ storage */

  reconnectMount(id: string): StorageMount | undefined {
    const mount = this.storage.mounts.find((entry) => entry.id === id);
    if (!mount) return undefined;
    const updated: StorageMount = { ...mount, state: "online", since: this.db.iso() };
    this.storage = { ...this.storage, mounts: this.storage.mounts.map((entry) => (entry.id === id ? updated : entry)) };
    return updated;
  }

  dispose() {
    this.unsubscribe();
  }

  /* ------------------------------------------------------------ tasks */

  private jobTask(job: Job): Task {
    const library = job.subject?.type === "library" ? (job.subject.name ?? undefined) : undefined;
    const done = job.state === "succeeded";
    const steps: TaskStep[] = [
      { id: "walk", label: "遍历目录", state: job.startedAt ? "done" : job.state === "queued" ? "pending" : "running", detail: null, at: job.startedAt },
      { id: "index", label: "识别并写入索引", state: done ? "done" : job.state === "failed" ? "failed" : job.state === "running" ? "running" : "pending", detail: null, at: job.finishedAt },
    ];
    const libraryId = job.subject?.type === "library" ? job.subject.id : undefined;
    return {
      id: job.id,
      kind: job.kind,
      title: library ? `扫描「${library}」` : "",
      subject: library ? { title: library, detail: null, artworkId: (libraryId && this.catalog?.libraryCovers[libraryId]?.[0]) ?? null } : null,
      scope: library ?? null,
      libraryId: libraryId ?? null,
      state: job.state,
      progress: job.progress,
      step: job.step,
      message: job.message,
      error: job.error,
      createdAt: job.createdAt,
      startedAt: job.startedAt,
      finishedAt: job.finishedAt,
      steps,
      result: [],
      origin: { module: "system", label: "媒体库" },
    };
  }

  upsertTask(task: Task) {
    const index = this.tasks.findIndex((entry) => entry.id === task.id);
    if (index === -1) this.tasks.unshift(task);
    else this.tasks[index] = task;
    this.db.emitRaw("task.updated", task);
  }

  getTask(id: string) {
    return this.tasks.find((task) => task.id === id);
  }

  /** Approving a task: either a backgrounded assistant run or a seeded continuation. */
  approveTask(id: string, selectedRowIds: string[]): Task | undefined {
    const task = this.getTask(id);
    if (!task || task.state !== "awaiting_approval") return task;
    const run = this.conversation.runs.find((entry) => entry.taskId === id);
    if (run) {
      this.approveRun(run.id, selectedRowIds);
      return this.getTask(id);
    }
    const continuation = this.continuations.get(id);
    const outcome = continuation?.(selectedRowIds);
    const steps = task.steps.map((step) => (step.state === "running" ? { ...step, state: "done" as const } : step));
    this.upsertTask({
      ...task,
      state: "running",
      progress: 0,
      message: outcome?.steps[0]?.running ?? null,
      steps,
      result: markApproval(task.result, "approved"),
    });
    this.db.schedule(() => {
      const current = this.getTask(id);
      if (!current) return;
      this.upsertTask({
        ...current,
        state: "succeeded",
        progress: 1,
        message: outcome?.steps.at(-1)?.done ?? null,
        finishedAt: this.db.iso(),
        steps: current.steps.map((step) => (step.state === "pending" ? { ...step, state: "done" as const, at: this.db.iso() } : step)),
        result: [...current.result, ...(outcome?.blocks ?? [])],
      });
    }, (outcome?.steps.reduce((sum, step) => sum + step.ms, 0) ?? 800) * this.timeScale);
    return this.getTask(id);
  }

  cancelTask(id: string): Task | undefined {
    const task = this.getTask(id);
    if (!task) return undefined;
    const run = this.conversation.runs.find((entry) => entry.taskId === id);
    if (run) this.cancelRun(run.id);
    else
      this.upsertTask({
        ...task,
        state: "cancelled",
        finishedAt: this.db.iso(),
        result: markApproval(task.result, "cancelled"),
        steps: task.steps.map((step) => (step.state === "running" || step.state === "pending" ? { ...step, state: "skipped" as const } : step)),
      });
    return this.getTask(id);
  }

  /** Running downloads and transcodes keep moving while the mock is open. */
  private simulate() {
    const tick = () => {
      for (const task of this.tasks) {
        if (task.state !== "running" || task.kind === "library.scan" || task.kind === "assistant.run") continue;
        if (task.progress === null) continue;
        const progress = Math.min(1, task.progress + 0.012 + Math.random() * 0.01);
        if (progress >= 1) {
          this.upsertTask({
            ...task,
            state: "succeeded",
            progress: 1,
            finishedAt: this.db.iso(),
            message: task.kind === "download" ? "已移入「电影」" : "转码完成",
            steps: task.steps.map((step) => ({ ...step, state: "done" as const, at: step.at ?? this.db.iso() })),
          });
        } else {
          this.upsertTask({ ...task, progress: Math.round(progress * 1000) / 1000 });
        }
      }
      this.db.schedule(tick, 2000);
    };
    this.db.schedule(tick, 2000);
  }

  /* ------------------------------------------------------------ assistant */

  private emitRun(run: AssistantRun) {
    const index = this.conversation.runs.findIndex((entry) => entry.id === run.id);
    if (index === -1) this.conversation.runs.push(run);
    else this.conversation.runs[index] = run;
    this.db.emitRaw("assistant.run.updated", run);
    if (run.taskId) this.mirrorTask(run);
  }

  private mirrorTask(run: AssistantRun) {
    const task = run.taskId ? this.getTask(run.taskId) : undefined;
    if (!task) return;
    const state = run.state === "running" ? "running" : run.state;
    this.upsertTask({
      ...task,
      state,
      message: run.steps.at(-1)?.text ?? null,
      finishedAt: state === "succeeded" || state === "failed" || state === "cancelled" ? this.db.iso() : null,
      steps: run.steps.map((step) => ({ id: step.id, label: step.text, state: step.state, detail: null, at: null })),
      result: run.blocks,
    });
  }

  /** Plays script steps one after another, then calls `finish`. */
  private play(runId: string, steps: ScriptStep[], finish: () => void) {
    const scale = this.timeScale;
    let delay = 350 * scale;
    steps.forEach((step, index) => {
      const stepId = `${runId}-s${this.getRun(runId)?.steps.length ?? 0}-${index}`;
      this.db.schedule(() => {
        const run = this.getRun(runId);
        if (!run || run.state === "cancelled") return;
        this.emitRun({ ...run, steps: [...run.steps, { id: stepId, text: step.running, state: "running" }] });
      }, delay);
      delay += step.ms * scale;
      this.db.schedule(() => {
        const run = this.getRun(runId);
        if (!run || run.state === "cancelled") return;
        this.emitRun({
          ...run,
          steps: run.steps.map((entry): RunStep => (entry.id === stepId ? { ...entry, text: step.done, detail: step.detail, state: "done" } : entry)),
        });
      }, delay);
    });
    this.db.schedule(() => {
      const run = this.getRun(runId);
      if (run && run.state !== "cancelled") finish();
    }, delay + 150 * scale);
  }

  getRun(id: string) {
    return this.conversation.runs.find((run) => run.id === id);
  }

  send(text: string, context: AssistantContext | undefined) {
    const now = this.db.iso();
    const outcome = pickScript({ text, context, db: this.db, activateScene: (id) => this.activateScene(id) });
    const run: AssistantRun = {
      id: nextId("run"),
      title: outcome.title,
      subject: outcome.subject ?? null,
      request: outcome.request,
      previews: outcome.previews,
      state: "running",
      steps: [],
      blocks: [],
      taskId: null,
      createdAt: now,
    };
    const message: AssistantMessage = { id: nextId("m"), role: "user", text, createdAt: now, runId: null };
    const reply: AssistantMessage = { id: nextId("m"), role: "assistant", text: outcome.reply, createdAt: now, runId: run.id };
    this.conversation.messages.push(message, reply);
    this.conversation.runs.push(run);
    this.continuations.set(run.id, outcome.approve);
    this.play(run.id, outcome.steps, () => {
      const current = this.getRun(run.id);
      if (!current) return;
      this.emitRun({ ...current, blocks: outcome.blocks, state: outcome.awaitsApproval ? "awaiting_approval" : "succeeded" });
    });
    return { message, reply, run };
  }

  approveRun(id: string, selectedRowIds: string[]): AssistantRun | undefined {
    const run = this.getRun(id);
    if (!run || run.state !== "awaiting_approval") return run;
    const outcome = this.continuations.get(id)?.(selectedRowIds);
    const kept = new Set(selectedRowIds);
    // Freeze the preview with exactly the rows that were approved.
    const blocks = markApproval(run.blocks, "approved").map((block): Block =>
      block.type === "change-preview" ? { ...block, rows: block.rows.map((row) => ({ ...row, selected: kept.has(row.id) })) } : block,
    );
    this.emitRun({ ...run, state: "running", blocks });
    this.play(id, outcome?.steps ?? [], () => {
      const current = this.getRun(id);
      if (current) this.emitRun({ ...current, state: "succeeded", blocks: [...current.blocks, ...(outcome?.blocks ?? [])] });
    });
    return this.getRun(id);
  }

  cancelRun(id: string): AssistantRun | undefined {
    const run = this.getRun(id);
    if (!run) return undefined;
    this.emitRun({
      ...run,
      state: "cancelled",
      steps: run.steps.map((step) => (step.state === "running" ? { ...step, state: "failed" as const } : step)),
      blocks: [...markApproval(run.blocks, "cancelled"), { type: "text", text: "已取消，没有改动任何内容。", tone: "muted" }],
    });
    return this.getRun(id);
  }

  /** Turns a run into a task that keeps going without the conversation open. */
  background(id: string): { run: AssistantRun; task: Task } | undefined {
    const run = this.getRun(id);
    if (!run) return undefined;
    const index = this.conversation.messages.findIndex((message) => message.runId === id);
    const prompt = this.conversation.messages[index - 1]?.text ?? "";
    const taskId = run.taskId ?? stableUuid(`task:${id}`);
    const task: Task = {
      id: taskId,
      kind: "assistant.run",
      title: prompt,
      state: run.state === "running" ? "running" : run.state,
      progress: null,
      message: run.steps.at(-1)?.text ?? null,
      error: null,
      createdAt: run.createdAt,
      startedAt: run.createdAt,
      finishedAt: null,
      steps: [],
      result: run.blocks,
      origin: { module: "assistant", label: "助手对话" },
    };
    this.upsertTask(task);
    const updated = { ...run, taskId };
    this.emitRun(updated);
    return { run: updated, task: this.getTask(taskId) ?? task };
  }

  reset() {
    this.conversation = { id: nextId("c"), messages: [], runs: [] };
    return this.conversation;
  }

  updateConfig(body: Partial<UpdateAssistantConfigRequest>): AssistantConfigResponse {
    const preset = body.preset ?? this.assistantConfig.preset;
    const found = this.assistantConfig.presets.find((entry) => entry.id === preset);
    const baseUrl = preset === "builtin" ? "" : (body.baseUrl?.trim() || found?.defaultBaseUrl || "");
    const model = preset === "builtin" ? "builtin-planner" : (body.model?.trim() || found?.defaultModel || "");
    const apiKeyConfigured =
      preset === "builtin" || body.clearApiKey
        ? false
        : body.apiKey && body.apiKey.trim().length > 0
          ? true
          : this.assistantConfig.apiKeyConfigured;
    const redactCloudPaths = body.redactCloudPaths ?? this.assistantConfig.redactCloudPaths;
    this.assistantConfig = { ...this.assistantConfig, preset, baseUrl, model, apiKeyConfigured, redactCloudPaths };
    return this.assistantConfig;
  }

  addMemory(content: string): AssistantMemory {
    const memory: AssistantMemory = { id: nextId("mem"), content: content.trim(), createdAt: this.db.iso() };
    this.memories = [memory, ...this.memories];
    return memory;
  }

  deleteMemory(id: string): boolean {
    const before = this.memories.length;
    this.memories = this.memories.filter((entry) => entry.id !== id);
    return this.memories.length < before;
  }

  /* ------------------------------------------------------------ home */

  updateDevice(id: string, patch: DevicePatch): HomeDevice | undefined {
    const device = this.home.devices.find((entry) => entry.id === id);
    if (!device || !device.online) return device;
    const state = { ...device.state, ...patch } as HomeDevice["state"];
    if (state.type === "climate") state.target = Math.min(30, Math.max(16, state.target));
    const updated: HomeDevice = { ...device, state };
    this.home = { ...this.home, devices: this.home.devices.map((entry) => (entry.id === id ? updated : entry)) };
    this.db.emitRaw("home.device.updated", updated);
    return updated;
  }

  activateScene(id: string) {
    const patches: Record<string, Record<string, DevicePatch>> = {
      movie: { "living-main": { on: false }, "living-strip": { on: true }, "living-tv": { status: "playing" } },
      home: { "living-main": { on: true }, "living-strip": { on: true }, "kitchen-light": { on: true } },
      night: { "living-main": { on: false }, "living-strip": { on: false }, "study-lamp": { on: false }, "kitchen-light": { on: false }, "bed-lamp": { on: false } },
      morning: { "living-main": { on: true }, "kitchen-light": { on: true }, "bed-lamp": { on: true } },
      away: { "living-main": { on: false }, "living-strip": { on: false }, "study-lamp": { on: false }, "kitchen-light": { on: false }, "living-ac": { on: false } },
    };
    for (const [deviceId, patch] of Object.entries(patches[id] ?? {})) this.updateDevice(deviceId, patch);
    if (id === "movie") this.updateDevice("living-strip", { brightness: 15 });
    this.home = { ...this.home, scenes: this.home.scenes.map((scene) => ({ ...scene, active: scene.id === id })) };
    return this.home;
  }

  setRule(id: string, enabled: boolean) {
    const rule = this.home.rules.find((entry) => entry.id === id);
    if (!rule) return undefined;
    const updated = { ...rule, enabled };
    this.home = { ...this.home, rules: this.home.rules.map((entry) => (entry.id === id ? updated : entry)) };
    return updated;
  }

  createDevice(req: CreateHomeDeviceRequest): HomeDevice {
    const roomName = req.roomName.trim();
    let room = this.home.rooms.find((entry) => entry.name === roomName);
    if (!room) {
      room = { id: nextId("room"), name: roomName };
      this.home = { ...this.home, rooms: [...this.home.rooms, room] };
    }
    const state: HomeDevice["state"] =
      req.kind === "light"
        ? { type: "light", on: false, brightness: 75 }
        : req.kind === "climate"
          ? { type: "climate", on: false, mode: "auto", current: 24, target: 24 }
          : req.kind === "sensor"
            ? { type: "sensor", readings: [{ kind: "temperature", value: 24, unit: "°C" }] }
            : req.kind === "media_player"
              ? { type: "media_player", status: "idle", title: null, subtitle: null, app: "Palveni", volume: 35 }
              : { type: "switch", on: false };
    const device: HomeDevice = { id: nextId("dev"), roomId: room.id, room: room.name, name: req.name.trim(), online: true, state };
    this.home = { ...this.home, devices: [...this.home.devices, device] };
    return device;
  }

  deleteDevice(id: string): boolean {
    const before = this.home.devices.length;
    this.home = { ...this.home, devices: this.home.devices.filter((entry) => entry.id !== id) };
    return this.home.devices.length < before;
  }

  createRule(req: CreateHomeRuleRequest): AutomationRule {
    const rule: AutomationRule = {
      id: nextId("rule"),
      trigger: req.trigger.trim(),
      action: req.action.trim(),
      enabled: true,
      lastRunAt: null,
    };
    this.home = { ...this.home, rules: [...this.home.rules, rule] };
    return rule;
  }

  deleteRule(id: string): boolean {
    const before = this.home.rules.length;
    this.home = { ...this.home, rules: this.home.rules.filter((entry) => entry.id !== id) };
    return this.home.rules.length < before;
  }

  saveBridge(req: SaveHomeBridgeRequest): HomeBridgeSummary {
    const bridge: HomeBridgeSummary = {
      id: req.id ?? nextId("bridge"),
      name: req.name.trim(),
      baseUrl: req.baseUrl.trim().replace(/\/+$/, ""),
      enabled: req.enabled ?? true,
      tokenConfigured: Boolean(req.token?.trim()),
      lastSyncAt: null,
    };
    const exists = this.bridges.some((entry) => entry.id === bridge.id);
    this.bridges = exists ? this.bridges.map((entry) => (entry.id === bridge.id ? bridge : entry)) : [...this.bridges, bridge];
    return bridge;
  }

  deleteBridge(id: string): boolean {
    const before = this.bridges.length;
    this.bridges = this.bridges.filter((entry) => entry.id !== id);
    return this.bridges.length < before;
  }

  syncBridge(id: string): HomeBridgeSyncResult | undefined {
    const bridge = this.bridges.find((entry) => entry.id === id);
    if (!bridge) return undefined;
    const updated: HomeBridgeSummary = { ...bridge, lastSyncAt: this.db.iso() };
    this.bridges = this.bridges.map((entry) => (entry.id === id ? updated : entry));
    return { bridge: updated, syncedDevices: 2, syncedScenes: 1, snapshot: this.home };
  }

  /* ------------------------------------------------------------ MCP, downloads & organizer */

  mcpInfo(isAdmin: boolean): McpInfoResponse {
    const endpointUrl = "http://127.0.0.1:3000/api/v1/mcp";
    const allTools = [
      { name: "catalog_overview", description: "Read media libraries and recent works.", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: false },
      { name: "storage_report", description: "Read storage volumes and mounts.", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: false },
      { name: "search_music", description: "Search music tracks.", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: false },
      { name: "search_videos", description: "Search films and series.", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: false },
      { name: "create_music_playlist", description: "Create awaiting_approval playlist preview.", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: true },
      { name: "control_home_device", description: "Control smart-home device (locks pause for approval).", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: false },
      { name: "activate_home_scene", description: "Activate smart-home scene.", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: false },
      { name: "trigger_library_scan", description: "Trigger incremental library scan (Admin).", inputSchema: { type: "object" }, adminOnly: true, requiresApproval: true },
      { name: "preview_media_organize", description: "Dry-run media organize preview (Admin).", inputSchema: { type: "object" }, adminOnly: true, requiresApproval: true },
      { name: "list_pending_tasks", description: "List pending and running tasks.", inputSchema: { type: "object" }, adminOnly: false, requiresApproval: false },
    ];
    return {
      protocolVersion: "2025-03-26",
      endpointPath: "/api/v1/mcp",
      serverName: this.db.runtimeSettings.serverName ?? this.db.name,
      serverVersion: this.db.version,
      tools: allTools.filter((tool) => isAdmin || !tool.adminOnly),
      clientConfigs: {
        claudeDesktop: JSON.stringify(
          {
            mcpServers: {
              palveni: {
                command: "npx",
                args: ["-y", "mcp-remote", endpointUrl, "--header", "Authorization: Bearer <YOUR_PALVENI_TOKEN>"],
              },
            },
          },
          null,
          2,
        ),
        cursor: JSON.stringify(
          {
            mcpServers: {
              palveni: {
                type: "http",
                url: endpointUrl,
                headers: { Authorization: "Bearer <YOUR_PALVENI_TOKEN>" },
              },
            },
          },
          null,
          2,
        ),
        openclaw: JSON.stringify(
          {
            mcp: {
              servers: {
                palveni: {
                  transport: "streamable-http",
                  url: endpointUrl,
                  headers: { Authorization: "Bearer <YOUR_PALVENI_TOKEN>" },
                },
              },
            },
          },
          null,
          2,
        ),
      },
    };
  }

  downloadsOverview(): DownloadsOverviewResponse {
    return {
      clients: this.downloaderClients,
      transfers: this.transfers,
      subscriptions: this.subscriptions,
    };
  }

  saveDownloaderClient(req: CreateDownloaderClientRequest): DownloaderClientSummary {
    const client: DownloaderClientSummary = {
      id: req.id ?? nextId("dl"),
      name: req.name.trim(),
      kind: req.kind,
      baseUrl: req.baseUrl.trim().replace(/\/+$/, ""),
      savePath: req.savePath?.trim() || null,
      enabled: req.enabled ?? true,
      credentialConfigured: Boolean(req.username?.trim() || req.password?.trim()),
      status: (req.enabled ?? true) ? "online" : "disabled",
      statusMessage: null,
      createdAt: this.db.iso(),
    };
    const exists = this.downloaderClients.some((entry) => entry.id === client.id);
    this.downloaderClients = exists
      ? this.downloaderClients.map((entry) => (entry.id === client.id ? client : entry))
      : [...this.downloaderClients, client];
    return client;
  }

  deleteDownloaderClient(id: string): boolean {
    const before = this.downloaderClients.length;
    this.downloaderClients = this.downloaderClients.filter((entry) => entry.id !== id);
    return this.downloaderClients.length < before;
  }

  createDownloadTask(req: CreateDownloadTaskRequest): CreateDownloadTaskResponse | null {
    const target = req.clientId
      ? this.downloaderClients.find((c) => c.id === req.clientId)
      : this.downloaderClients.find((c) => c.enabled);
    if (!target) return null;
    const label = req.uri.startsWith("magnet:")
      ? decodeURIComponent((/dn=([^&]+)/.exec(req.uri)?.[1] ?? "Magnet.Download.2160p.mkv").replace(/\+/g, " "))
      : (req.uri.split("/").pop() ?? "Download.Item.mkv");
    this.transfers = [
      {
        id: nextId("tr"),
        clientId: target.id,
        clientName: target.name,
        clientKind: target.kind,
        name: label,
        state: "downloading",
        progress: 0.05,
        sizeBytes: 4_200_000_000,
        downloadedBytes: 210_000_000,
        downloadSpeedBps: 15_400_000,
        uploadSpeedBps: 0,
        etaSeconds: 260,
        savePath: req.savePath?.trim() || target.savePath,
      },
      ...this.transfers,
    ];
    return {
      accepted: true,
      clientId: target.id,
      clientName: target.name,
      message: `已投递至下载器「${target.name}」`,
    };
  }

  createSubscription(ownerId: string, req: CreateSubscriptionRequest): MediaSubscription {
    const sub: MediaSubscription = {
      id: nextId("sub"),
      ownerId,
      title: req.title.trim(),
      mediaKind: req.mediaKind,
      qualityRule: req.qualityRule?.trim() || null,
      feedUrl: req.feedUrl?.trim() || null,
      targetLibraryId: req.targetLibraryId?.trim() || null,
      enabled: req.enabled ?? true,
      lastCheckedAt: null,
      createdAt: this.db.iso(),
    };
    this.subscriptions = [sub, ...this.subscriptions];
    return sub;
  }

  deleteSubscription(id: string): boolean {
    const before = this.subscriptions.length;
    this.subscriptions = this.subscriptions.filter((entry) => entry.id !== id);
    return this.subscriptions.length < before;
  }

  checkSubscription(id: string): SubscriptionCheckResult | undefined {
    const sub = this.subscriptions.find((entry) => entry.id === id);
    if (!sub) return undefined;
    const updated: MediaSubscription = { ...sub, lastCheckedAt: this.db.iso() };
    this.subscriptions = this.subscriptions.map((entry) => (entry.id === id ? updated : entry));
    const qualityTag = sub.qualityRule ?? "1080p";
    return {
      subscription: updated,
      totalFeedItems: 12,
      matches: [
        {
          title: `[Feed] ${sub.title} S01E06 ${qualityTag}`,
          link: "magnet:?xt=urn:btih:feedmatch00001",
          pubDate: this.db.iso(),
          matchedQuality: true,
        },
      ],
      submittedCount: 0,
    };
  }

  private collectLeafItems(libraryId: string) {
    const top = this.db.listChildren(libraryId, null);
    const leaves = top.filter((item) => item.kind !== "folder" && item.path);
    for (const folder of top.filter((item) => item.kind === "folder")) {
      for (const child of this.db.listChildren(libraryId, folder.id)) {
        if (child.kind !== "folder" && child.path) leaves.push(child);
      }
    }
    return leaves.slice(0, 12);
  }

  previewOrganize(req: OrganizePreviewRequest): OrganizePreviewResponse | undefined {
    const library = this.db.getLibrary(req.libraryId);
    if (!library) return undefined;
    const root = library.paths[0] ?? "/media";
    const mode = req.mode ?? "rename";
    const template = req.template.trim();
    const leaves = this.collectLeafItems(library.id);

    const candidates: OrganizeCandidate[] = leaves.map((item, idx) => {
      const srcPath = item.path ?? `${root}/${item.title}.mkv`;
      const extMatch = /\.([a-z0-9]+)$/i.exec(srcPath);
      const ext = extMatch?.[1] ?? "mkv";
      const relBefore = srcPath.startsWith(`${root}/`) ? srcPath.slice(root.length + 1) : srcPath;
      const expanded = template
        .replace(/\{title\}/g, item.title)
        .replace(/\{year\}/g, "2024")
        .replace(/\{season\}/g, "01")
        .replace(/\{episode\}/g, String(idx + 1).padStart(2, "0"))
        .replace(/\{artist\}/g, "周杰伦")
        .replace(/\{author\}/g, "刘慈欣");
      const relAfter = `${expanded}.${ext}`;
      const dstPath = `${root}/${relAfter}`;
      const same = dstPath === srcPath;
      return {
        rowId: `org-${idx}`,
        itemId: item.id,
        sourcePath: srcPath,
        targetPath: dstPath,
        relativeBefore: relBefore,
        relativeAfter: relAfter,
        sizeBytes: item.sizeBytes,
        conflict: same ? "unchanged" : null,
        selectable: !same,
      };
    });

    const readyCount = candidates.filter((c) => c.selectable).length;
    return {
      libraryId: library.id,
      libraryName: library.name,
      mode,
      template,
      readOnlyMount: false,
      collisionCount: 0,
      readyCount,
      candidates,
      previewBlock: {
        type: "change-preview",
        id: "organizer-preview",
        title: `「${library.name}」整理变更预览`,
        summary: `共 ${readyCount} 项可安全整理，执行后支持一键撤销恢复原路径。`,
        rows: candidates.map((c) => ({
          id: c.rowId,
          before: c.relativeBefore,
          after: c.relativeAfter,
          detail: mode === "rename" ? "原地重命名并同步索引" : "创建硬链接并更新索引",
          selected: c.selectable,
          group: c.selectable ? "ready" : "unchanged",
          sizeBytes: c.sizeBytes ?? undefined,
        })),
        groups: [
          { id: "ready", label: "可执行变更", collapseAfter: 25 },
          { id: "unchanged", label: "无需改动", collapseAfter: 5 },
        ],
        columns: { item: "原相对路径", change: "整理后路径", reason: "方式与检查" },
      },
      presets: this.organizerPresets,
    };
  }

  applyOrganize(actorId: string, req: OrganizeApplyRequest): OrganizeBatchSummary | undefined {
    const preview = this.previewOrganize({ libraryId: req.libraryId, mode: req.mode, template: req.template });
    if (!preview) return undefined;
    const chosen = new Set(req.selectedItemIds);
    const ops = preview.candidates
      .filter((c) => c.selectable && chosen.has(c.itemId))
      .map((c) => {
        const item = this.db.getItem(c.itemId);
        if (item) item.path = c.targetPath;
        return {
          itemId: c.itemId,
          fromPath: c.sourcePath,
          toPath: c.targetPath,
          createdDirs: [] as string[],
        };
      });
    const batch: OrganizeBatchSummary = {
      id: nextId("batch"),
      actorId,
      libraryId: preview.libraryId,
      libraryName: preview.libraryName,
      mode: req.mode,
      template: preview.template,
      operationCount: ops.length,
      operations: ops,
      undoneAt: null,
      createdAt: this.db.iso(),
    };
    this.organizeBatches = [batch, ...this.organizeBatches];
    return batch;
  }

  undoOrganizeBatch(id: string): OrganizeBatchSummary | "conflict" | undefined {
    const batch = this.organizeBatches.find((entry) => entry.id === id);
    if (!batch) return undefined;
    if (batch.undoneAt) return "conflict";
    for (const op of [...batch.operations].reverse()) {
      const item = this.db.getItem(op.itemId);
      if (item) item.path = op.fromPath;
    }
    const updated: OrganizeBatchSummary = { ...batch, undoneAt: this.db.iso() };
    this.organizeBatches = this.organizeBatches.map((entry) => (entry.id === id ? updated : entry));
    return updated;
  }
}

function markApproval(blocks: Block[], status: "approved" | "cancelled"): Block[] {
  return blocks.map((block) => (block.type === "approval" ? { ...block, status } : block));
}
