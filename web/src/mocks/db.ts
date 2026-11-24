/**
 * In-memory server state behind the mock handlers: users and tokens, libraries and their item
 * trees, jobs (with a simulated scan), the hardware report, and an event bus for SSE.
 */
import type {
  AuditLogEntry,
  BackupSummary,
  HardwareReport,
  Item,
  Job,
  Library,
  LibraryKind,
  ManagedUser,
  Plugin,
  Role,
  RuntimeSettings,
  ServerEventMap,
  ServerEventName,
  User,
  UserPolicy,
  UserSession,
} from "@/api/types";
import { generateItems, sampleHardwareReport, stableUuid } from "./fixtures";

export type MockScenario = "seeded" | "fresh";

/** What a real M0 server reports in `GET /system/info` → `features`. */
export const M0_FEATURES = ["libraries", "jobs", "hardware", "plugins", "skins", "settings"] as const;

export interface MockDbOptions {
  scenario?: MockScenario;
  /** Feature flags to report; defaults to the M0 set. */
  features?: readonly string[];
  /** Multiplies generated item counts (tests use a small scale). */
  scale?: number;
  /** Total simulated scan duration. */
  scanDurationMs?: number;
  /** Seed a scan of 家庭相册 that keeps running (progress unknown), as the design comps show. */
  liveScan?: boolean;
  now?: () => number;
}

interface StoredUser extends User {
  password: string;
  disabled: boolean;
  updatedAt: string;
  policy: UserPolicy;
  allowedLibraryIds: string[];
}

interface StoredSession {
  id: string;
  userId: string;
  deviceId: string | null;
  deviceName: string;
  token: string;
  createdAt: string;
  lastUsedAt: string;
}

export interface StoredBackup {
  summary: BackupSummary;
  payloadJson: string;
  snapshot: {
    libraries: Library[];
    runtimeSettings: RuntimeSettings;
  };
}

type Listener = (event: string, data: unknown) => void;

const HOUR = 3_600_000;

/** `scan.summary` parameters and the server's English text for them (see the scanner). */
function summaryParams(added: number, unavailablePaths: number) {
  return { added, updated: 0, removed: 0, unchanged: 0, unavailablePaths, unreadableFolders: 0 };
}

function summaryMessage(added: number, unavailablePaths: number): string {
  const text = `${added} added, 0 updated, 0 removed, 0 unchanged`;
  return unavailablePaths > 0 ? `${text}; ${unavailablePaths} library paths unavailable` : text;
}

export const MOCK_ACCOUNTS = {
  admin: { username: "admin", password: "admin1234" },
  user: { username: "family", password: "family1234" },
} as const;

export class MockDb {
  readonly serverId = "6f1c2a9e-3b7d-4c55-9e21-8a4f0d6b7c13";
  readonly version = "0.1.0-dev";
  readonly name: string;
  readonly features: readonly string[];
  users: StoredUser[] = [];
  libraries: Library[] = [];
  jobs: Job[] = [];
  plugins: Plugin[] = [];
  auditLogs: AuditLogEntry[] = [];
  backups: StoredBackup[] = [];
  runtimeSettings: RuntimeSettings = { serverName: null, perfTier: null };
  hardware: HardwareReport;
  private sessions: StoredSession[] = [];
  private items = new Map<string, Item[]>();
  private children = new Map<string, Item[]>();
  private itemsById = new Map<string, Item>();
  private revoked = new Set<string>();
  private listeners = new Set<Listener>();
  private timers = new Set<ReturnType<typeof setTimeout>>();
  /** libraryId → id of its running scan job. */
  private activeScans = new Map<string, string>();
  private readonly scale: number;
  private readonly scanDurationMs: number;
  readonly now: () => number;

  constructor(options: MockDbOptions = {}) {
    this.scale = options.scale ?? 1;
    this.scanDurationMs = options.scanDurationMs ?? 7000;
    this.now = options.now ?? (() => Date.now());
    this.name = "客厅 NAS";
    this.features = options.features ?? M0_FEATURES;
    this.hardware = sampleHardwareReport(this.iso(-2 * HOUR));
    if ((options.scenario ?? "seeded") === "seeded") this.seed(options.liveScan ?? false);
  }

  iso(offsetMs = 0): string {
    return new Date(this.now() + offsetMs).toISOString().replace(/\.\d{3}Z$/, "Z");
  }

  /* ------------------------------------------------------------ users and tokens */

  get setupCompleted(): boolean {
    return this.users.length > 0;
  }

  addUser(
    username: string,
    password: string,
    role: Role,
    createdAt = this.iso(),
    policy?: UserPolicy,
    allowedLibraryIds: string[] = [],
  ): User {
    const baseId = stableUuid(`user:${username}`);
    const id = this.users.some((u) => u.id === baseId) ? stableUuid(`user:${username}:${this.now()}`) : baseId;
    const user: StoredUser = {
      id,
      username,
      password,
      role,
      createdAt,
      updatedAt: createdAt,
      disabled: false,
      policy: policy ?? {
        allLibraries: true,
        allowDownload: true,
        allowTranscode: true,
        dailyAiLimit: 100,
      },
      allowedLibraryIds: [...allowedLibraryIds],
    };
    this.users.push(user);
    return publicUser(user);
  }

  managedUsers(): ManagedUser[] {
    return this.users.map((u) => this.toManagedUser(u));
  }

  toManagedUser(u: StoredUser): ManagedUser {
    const activeSessions = u.disabled
      ? 0
      : this.sessions.filter((s) => s.userId === u.id && !this.revoked.has(s.token)).length;
    const liveIds = new Set(this.libraries.map((lib) => lib.id));
    return {
      id: u.id,
      username: u.username,
      role: u.role,
      disabled: u.disabled,
      policy: { ...u.policy },
      allowedLibraryIds: u.allowedLibraryIds.filter((id) => liveIds.has(id)),
      activeSessions,
      createdAt: u.createdAt,
      updatedAt: u.updatedAt,
    };
  }

  findUserByCredentials(username: string, password: string): User | null {
    const key = username.trim().toLowerCase();
    const user = this.users.find(
      (entry) => entry.username.toLowerCase() === key && entry.password === password && !entry.disabled,
    );
    return user ? publicUser(user) : null;
  }

  /** Tokens embed the user id so seeded sessions survive a page reload of the mock. */
  issueToken(user: User, deviceName = "Browser"): string {
    const nonce = Math.random().toString(36).slice(2, 12);
    const token = `mock.${user.id}.${nonce}`;
    const now = this.iso();
    this.sessions.unshift({
      id: stableUuid(`session:${token}`),
      userId: user.id,
      deviceId: stableUuid(`device:${user.id}:${deviceName}:${nonce}`),
      deviceName,
      token,
      createdAt: now,
      lastUsedAt: now,
    });
    return token;
  }

  userForToken(token: string | null): User | null {
    if (!token || this.revoked.has(token)) return null;
    const [prefix, userId] = token.split(".");
    if (prefix !== "mock" || !userId) return null;
    const user = this.users.find((entry) => entry.id === userId);
    if (!user || user.disabled) return null;
    return publicUser(user);
  }

  revoke(token: string) {
    this.revoked.add(token);
  }

  revokeSessionsForUser(userId: string, keepToken?: string) {
    for (const s of this.sessions) {
      if (s.userId === userId && s.token !== keepToken) {
        this.revoked.add(s.token);
      }
    }
  }

  listSessionsForUser(userId: string, currentToken: string): UserSession[] {
    return this.sessions
      .filter((s) => s.userId === userId && !this.revoked.has(s.token))
      .map((s) => ({
        id: s.id,
        deviceId: s.deviceId,
        deviceName: s.deviceName,
        current: s.token === currentToken,
        createdAt: s.createdAt,
        lastUsedAt: s.lastUsedAt,
        expiresAt: null,
      }))
      .sort((a, b) => Number(b.current) - Number(a.current));
  }

  revokeUserSession(userId: string, sessionId: string): boolean {
    const found = this.sessions.find((s) => s.id === sessionId && s.userId === userId);
    if (!found) return false;
    this.revoked.add(found.token);
    return true;
  }

  canAccessLibrary(user: User, libraryId: string): boolean {
    if (user.role === "admin") return true;
    const stored = this.users.find((u) => u.id === user.id);
    if (!stored || stored.policy.allLibraries) return true;
    return stored.allowedLibraryIds.includes(libraryId);
  }

  visibleLibraries(user: User): Library[] {
    return this.libraries.filter((lib) => this.canAccessLibrary(user, lib.id));
  }

  recordAudit(action: string, actor: User | null, target: string | null = null, detail: Record<string, unknown> | null = null) {
    this.auditLogs.unshift({
      id: stableUuid(`audit:${this.now()}:${this.auditLogs.length}:${Math.random()}`),
      occurredAt: this.iso(),
      actorId: actor?.id ?? null,
      actorUsername: actor?.username ?? null,
      action,
      target,
      detail: detail ? JSON.stringify(detail) : null,
    });
  }

  allLeafItems(): Item[] {
    return [...this.itemsById.values()].filter((item) => item.kind !== "folder");
  }

  /* ------------------------------------------------------------ libraries and items */

  getLibrary(id: string): Library | undefined {
    return this.libraries.find((library) => library.id === id);
  }

  createLibrary(name: string, kind: LibraryKind, paths: string[]): Library {
    const now = this.iso();
    const library: Library = {
      id: stableUuid(`library:${name}:${this.now()}:${Math.random()}`),
      name,
      kind,
      paths,
      itemCount: 0,
      lastScanAt: null,
      createdAt: now,
      updatedAt: now,
    };
    this.libraries.push(library);
    this.setItems(library.id, []);
    this.emit("library.updated", library);
    return library;
  }

  deleteLibrary(id: string) {
    this.libraries = this.libraries.filter((library) => library.id !== id);
    for (const item of this.items.get(id) ?? []) this.itemsById.delete(item.id);
    this.items.delete(id);
    for (const key of [...this.children.keys()]) if (key.startsWith(`${id}:`)) this.children.delete(key);
    this.emit("library.deleted", { id });
  }

  getItem(id: string): Item | undefined {
    return this.itemsById.get(id);
  }

  /** Children of a folder (or the top level), folders first, then by title. */
  listChildren(libraryId: string, parentId: string | null): Item[] {
    return this.children.get(`${libraryId}:${parentId ?? "root"}`) ?? [];
  }

  private setItems(libraryId: string, items: Item[]) {
    for (const item of this.items.get(libraryId) ?? []) this.itemsById.delete(item.id);
    for (const key of [...this.children.keys()]) if (key.startsWith(`${libraryId}:`)) this.children.delete(key);
    this.items.set(libraryId, items);
    for (const item of items) {
      this.itemsById.set(item.id, item);
      const key = `${libraryId}:${item.parentId ?? "root"}`;
      const list = this.children.get(key);
      if (list) list.push(item);
      else this.children.set(key, [item]);
    }
    const collator = new Intl.Collator("zh-CN", { numeric: true });
    for (const list of this.children.values()) {
      list.sort((a, b) => (a.kind === "folder" ? 0 : 1) - (b.kind === "folder" ? 0 : 1) || collator.compare(a.title, b.title));
    }
  }

  private populate(library: Library, scannedAt: number, variant?: "series") {
    const items = generateItems({
      libraryId: library.id,
      kind: library.kind,
      root: library.paths[0] ?? "/",
      scale: this.scale,
      scannedAt,
      variant,
    });
    this.setItems(library.id, items);
    library.itemCount = items.filter((item) => item.kind !== "folder").length;
  }

  /* ------------------------------------------------------------ jobs */

  private upsertJob(job: Job) {
    const index = this.jobs.findIndex((entry) => entry.id === job.id);
    if (index === -1) this.jobs.unshift(job);
    else this.jobs[index] = job;
    this.emit("job.updated", job);
  }

  /** Starts (or joins) a simulated scan that reports progress over `scanDurationMs`. */
  startScan(libraryId: string): Job {
    const existing = this.activeScans.get(libraryId);
    const running = existing ? this.jobs.find((job) => job.id === existing) : undefined;
    if (running) return running;

    const library = this.getLibrary(libraryId);
    let job: Job = {
      id: stableUuid(`job:${libraryId}:${this.now()}:${Math.random()}`),
      kind: "library.scan",
      state: "queued",
      subject: { type: "library", id: libraryId, name: library?.name ?? null },
      progress: null,
      step: null,
      message: null,
      error: null,
      createdAt: this.iso(),
      startedAt: null,
      finishedAt: null,
    };
    this.activeScans.set(libraryId, job.id);
    this.upsertJob(job);

    const steps = 14;
    const stepMs = this.scanDurationMs / (steps + 2);
    const estimate = 400 + Math.floor(Math.random() * 1800);
    let step = 0;

    const tick = () => {
      if (!this.getLibrary(libraryId)) {
        // Failed or cancelled jobs carry neither step nor message.
        job = { ...job, state: "cancelled", finishedAt: this.iso(), step: null, message: null };
        this.activeScans.delete(libraryId);
        this.upsertJob(job);
        return;
      }
      step++;
      if (step === 1) {
        const found = 120 + Math.floor(Math.random() * 400);
        job = { ...job, state: "running", startedAt: this.iso(), step: { code: "scan.discovering", params: { found } }, message: `Discovering files: ${found} found` };
      } else if (step <= steps) {
        const progress = Math.min(0.97, (step - 1) / steps + Math.random() * 0.03);
        const done = Math.round(progress * estimate);
        job = {
          ...job,
          progress: Math.round(progress * 1000) / 1000,
          step: { code: "scan.indexing", params: { done, total: estimate } },
          message: `Updating index: ${done} of ${estimate}`,
        };
      } else {
        const current = this.getLibrary(libraryId);
        if (current) {
          const scannedAt = this.now();
          this.populate(current, scannedAt);
          current.lastScanAt = this.iso();
          current.updatedAt = current.lastScanAt;
          this.emit("library.updated", { ...current });
        }
        const added = current?.itemCount ?? 0;
        job = {
          ...job,
          state: "succeeded",
          progress: 1,
          finishedAt: this.iso(),
          step: { code: "scan.summary", params: summaryParams(added, 0) },
          message: summaryMessage(added, 0),
        };
        this.activeScans.delete(libraryId);
        this.upsertJob(job);
        return;
      }
      this.upsertJob(job);
      this.schedule(tick, stepMs);
    };
    this.schedule(tick, stepMs);
    return job;
  }

  /* ------------------------------------------------------------ events */

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  emit<K extends ServerEventName>(event: K, data: ServerEventMap[K]) {
    this.emitRaw(event, data);
  }

  /** Events outside the M0 contract (provisional mocks). */
  emitRaw(event: string, data: unknown) {
    for (const listener of this.listeners) listener(event, data);
  }

  schedule(callback: () => void, ms: number) {
    const timer = setTimeout(() => {
      this.timers.delete(timer);
      callback();
    }, ms);
    this.timers.add(timer);
  }

  /** Stops simulated work (tests). */
  dispose() {
    for (const timer of this.timers) clearTimeout(timer);
    this.timers.clear();
    this.listeners.clear();
  }

  /* ------------------------------------------------------------ seed */

  private seed(liveScan: boolean) {
    const admin = this.addUser(MOCK_ACCOUNTS.admin.username, MOCK_ACCOUNTS.admin.password, "admin", this.iso(-40 * 24 * HOUR));
    const member = this.addUser(MOCK_ACCOUNTS.user.username, MOCK_ACCOUNTS.user.password, "user", this.iso(-38 * 24 * HOUR));
    this.recordAudit("setup.completed", admin, admin.username, null);
    this.recordAudit("user.created", admin, member.username, { role: "user" });

    const seeds: Array<{
      key: string;
      name: string;
      kind: LibraryKind;
      paths: string[];
      scannedHoursAgo: number | null;
      unavailablePaths?: number;
      variant?: "series";
    }> = [
      { key: "movies", name: "电影", kind: "video", paths: ["/srv/media/movies"], scannedHoursAgo: 3 },
      // The USB archive was unplugged during the last scan: its items were kept, the scan says so.
      { key: "series", name: "剧集", kind: "video", paths: ["/srv/media/tv", "/mnt/usb-archive/tv"], scannedHoursAgo: 26, variant: "series", unavailablePaths: 1 },
      { key: "music", name: "无损音乐", kind: "music", paths: ["/srv/media/music"], scannedHoursAgo: 5 },
      { key: "photos", name: "家庭相册", kind: "photo", paths: ["/srv/photos"], scannedHoursAgo: 50 },
      { key: "books", name: "电子书", kind: "book", paths: ["/srv/books"], scannedHoursAgo: 120 },
      { key: "comics", name: "漫画", kind: "comic", paths: ["/srv/comics"], scannedHoursAgo: null },
    ];

    for (const seed of seeds) {
      const created = this.iso(-30 * 24 * HOUR);
      const library: Library = {
        id: stableUuid(`seed-library:${seed.key}`),
        name: seed.name,
        kind: seed.kind,
        paths: seed.paths,
        itemCount: 0,
        lastScanAt: null,
        createdAt: created,
        updatedAt: created,
      };
      this.libraries.push(library);
      if (seed.scannedHoursAgo === null) {
        this.setItems(library.id, []);
        continue;
      }
      const scannedAt = this.now() - seed.scannedHoursAgo * HOUR;
      this.populate(library, scannedAt, seed.variant);
      library.lastScanAt = new Date(scannedAt).toISOString().replace(/\.\d{3}Z$/, "Z");
      library.updatedAt = library.lastScanAt;

      const duration = 20_000 + Math.min(library.itemCount * 6, 600_000);
      this.jobs.push({
        id: stableUuid(`seed-job:${seed.key}`),
        kind: "library.scan",
        state: "succeeded",
        subject: { type: "library", id: library.id, name: library.name },
        progress: 1,
        step: { code: "scan.summary", params: summaryParams(library.itemCount, seed.unavailablePaths ?? 0) },
        message: summaryMessage(library.itemCount, seed.unavailablePaths ?? 0),
        error: null,
        createdAt: new Date(scannedAt - duration - 1500).toISOString().replace(/\.\d{3}Z$/, "Z"),
        startedAt: new Date(scannedAt - duration).toISOString().replace(/\.\d{3}Z$/, "Z"),
        finishedAt: library.lastScanAt,
      });
    }

    this.jobs.push({
      id: stableUuid("seed-job:photos-failed"),
      kind: "library.scan",
      state: "failed",
      subject: { type: "library", id: stableUuid("seed-library:photos"), name: "家庭相册" },
      progress: 0.42,
      step: null,
      message: null,
      error: "Permission denied (os error 13): /srv/photos/.private",
      createdAt: this.iso(-74 * HOUR),
      startedAt: this.iso(-74 * HOUR + 1500),
      finishedAt: this.iso(-74 * HOUR + 95_000),
    });
    if (liveScan) {
      this.jobs.push({
        id: stableUuid("seed-job:photos-live"),
        kind: "library.scan",
        state: "running",
        subject: { type: "library", id: stableUuid("seed-library:photos"), name: "家庭相册" },
        progress: null,
        step: { code: "scan.discovering", params: { found: 312 } },
        message: "Discovering files: 312 found",
        error: null,
        createdAt: this.iso(-4 * 60_000),
        startedAt: this.iso(-4 * 60_000 + 1000),
        finishedAt: null,
      });
    }
    this.jobs.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  }
}

function publicUser(user: StoredUser): User {
  return { id: user.id, username: user.username, role: user.role, createdAt: user.createdAt };
}
