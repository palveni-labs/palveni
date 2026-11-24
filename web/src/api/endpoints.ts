import { request, requestBlob } from "./client";
import type {
  AuditLogPage,
  BackupList,
  BackupRestoreResult,
  BackupSummary,
  ChangePasswordRequest,
  CreateBackupRequest,
  CreateLibraryRequest,
  CreateUserRequest,
  ExternalImportRequest,
  ExternalImportResult,
  ManagedUser,
  MetadataConfigResponse,
  MetadataOverride,
  MetadataOverrideResponse,
  MetadataSearchRequest,
  MetadataSearchResponse,
  SaveMetadataOverrideRequest,
  ServerLogList,
  SystemDoctorReport,
  UpdateLibraryRequest,
  UpdateMetadataConfigRequest,
  UpdateUserRequest,
  UserList,
  UserSessionList,
  Credentials,
  HardwareReport,
  HealthResponse,
  Item,
  ItemListParams,
  MusicPage,
  SubsonicPassword,
  SubsonicPasswordStatus,
  ItemPage,
  JobList,
  InstallPluginRequest,
  Library,
  LibraryList,
  LoginRequest,
  LoginResponse,
  Plugin,
  PluginCatalogResponse,
  PluginList,
  UpdatePluginRequest,
  ScanResponse,
  ScanReviewList,
  ScanReviewPaths,
  SetupResponse,
  InstalledSkin,
  RegistrySkin,
  RuntimeSettings,
  SystemInfo,
  User,
} from "./types";

const enc = encodeURIComponent;

/** One function per M0 endpoint. */
export const api = {
  subsonicPassword: {
    status: (signal?: AbortSignal) => request<SubsonicPasswordStatus>("/me/subsonic-password", { signal }),
    issue: (personal: boolean) => request<SubsonicPassword>("/me/subsonic-password", { method: "POST", query: { personal } }),
    revoke: () => request<void>("/me/subsonic-password", { method: "DELETE" }),
  },
  system: {
    info: (signal?: AbortSignal) => request<SystemInfo>("/system/info", { signal, auth: false }),
    health: (signal?: AbortSignal) => request<HealthResponse>("/system/health", { signal, auth: false }),
    settings: (signal?: AbortSignal) => request<RuntimeSettings>("/system/settings", { signal }),
    updateSettings: (body: RuntimeSettings) => request<RuntimeSettings>("/system/settings", { method: "PUT", body }),
    hardware: (refresh: boolean, signal?: AbortSignal) =>
      request<HardwareReport>("/system/hardware", { signal, query: refresh ? { refresh: true } : undefined }),
  },
  setup: (body: Credentials) => request<SetupResponse>("/setup", { method: "POST", body, auth: false }),
  auth: {
    login: (body: LoginRequest) => request<LoginResponse>("/auth/login", { method: "POST", body, auth: false }),
    logout: () => request<void>("/auth/logout", { method: "POST" }),
    me: (signal?: AbortSignal) => request<User>("/auth/me", { signal }),
    changePassword: (body: ChangePasswordRequest) => request<void>("/me/password", { method: "PUT", body }),
    sessions: (signal?: AbortSignal) => request<UserSessionList>("/me/sessions", { signal }),
    revokeSession: (id: string) => request<void>(`/me/sessions/${enc(id)}`, { method: "DELETE" }),
  },
  users: {
    list: (signal?: AbortSignal) => request<UserList>("/users", { signal }),
    create: (body: CreateUserRequest) => request<ManagedUser>("/users", { method: "POST", body }),
    update: (id: string, body: UpdateUserRequest) => request<ManagedUser>(`/users/${enc(id)}`, { method: "PATCH", body }),
    remove: (id: string) => request<void>(`/users/${enc(id)}`, { method: "DELETE" }),
  },
  maintenance: {
    audit: (params?: { action?: string; cursor?: string | null; limit?: number }, signal?: AbortSignal) =>
      request<AuditLogPage>("/system/audit", {
        signal,
        query: {
          action: params?.action || undefined,
          cursor: params?.cursor ?? undefined,
          limit: params?.limit,
        },
      }),
    backups: (signal?: AbortSignal) => request<BackupList>("/system/backups", { signal }),
    createBackup: (body: CreateBackupRequest = {}) => request<BackupSummary>("/system/backups", { method: "POST", body }),
    downloadBackup: (id: string, signal?: AbortSignal) => requestBlob(`/system/backups/${enc(id)}/download`, { signal }),
    restoreBackup: (id: string) => request<BackupRestoreResult>(`/system/backups/${enc(id)}/restore`, { method: "POST" }),
    deleteBackup: (id: string) => request<void>(`/system/backups/${enc(id)}`, { method: "DELETE" }),
    importExternal: (body: ExternalImportRequest) => request<ExternalImportResult>("/system/import", { method: "POST", body }),
  },
  metadata: {
    config: (signal?: AbortSignal) => request<MetadataConfigResponse>("/metadata/config", { signal }),
    updateConfig: (body: UpdateMetadataConfigRequest) =>
      request<MetadataConfigResponse>("/metadata/config", { method: "PUT", body }),
    search: (body: MetadataSearchRequest) =>
      request<MetadataSearchResponse>("/metadata/search", { method: "POST", body }),
    getOverride: (itemId: string, signal?: AbortSignal) =>
      request<MetadataOverrideResponse>(`/items/${enc(itemId)}/metadata-override`, { signal }),
    saveOverride: (itemId: string, body: SaveMetadataOverrideRequest) =>
      request<MetadataOverride>(`/items/${enc(itemId)}/metadata-override`, { method: "PUT", body }),
    deleteOverride: (itemId: string) =>
      request<void>(`/items/${enc(itemId)}/metadata-override`, { method: "DELETE" }),
  },
  observability: {
    doctor: (signal?: AbortSignal) => request<SystemDoctorReport>("/system/doctor", { signal }),
    logs: (params?: { level?: string; limit?: number }, signal?: AbortSignal) =>
      request<ServerLogList>("/system/logs", {
        signal,
        query: {
          level: params?.level && params.level !== "all" ? params.level : undefined,
          limit: params?.limit,
        },
      }),
  },
  libraries: {
    list: (signal?: AbortSignal) => request<LibraryList>("/libraries", { signal }),
    get: (id: string, signal?: AbortSignal) => request<Library>(`/libraries/${enc(id)}`, { signal }),
    create: (body: CreateLibraryRequest) => request<Library>("/libraries", { method: "POST", body }),
    update: (id: string, body: UpdateLibraryRequest) => request<Library>(`/libraries/${enc(id)}`, { method: "PATCH", body }),
    remove: (id: string) => request<void>(`/libraries/${enc(id)}`, { method: "DELETE" }),
    scanReviews: (id: string, signal?: AbortSignal) =>
      request<ScanReviewList>(`/libraries/${enc(id)}/scan/reviews`, { signal }),
    scanReviewPaths: (id: string, revision: string, offset: number, limit: number, signal?: AbortSignal) =>
      request<ScanReviewPaths>(`/libraries/${enc(id)}/scan/reviews/${enc(revision)}/items`, { query: { offset, limit }, signal }),
    confirmScanReview: (id: string, revision: string) =>
      request<ScanResponse>(`/libraries/${enc(id)}/scan/reviews/${enc(revision)}/confirm`, { method: "POST" }),
    scan: (id: string) => request<ScanResponse>(`/libraries/${enc(id)}/scan`, { method: "POST" }),
  },
  items: {
    list: (params: ItemListParams, signal?: AbortSignal) =>
      request<ItemPage>("/items", {
        signal,
        query: {
          libraryId: params.libraryId,
          parentId: params.parentId,
          kind: params.kind,
          limit: params.limit,
          cursor: params.cursor,
        },
      }),
    get: (id: string, signal?: AbortSignal) => request<Item>(`/items/${enc(id)}`, { signal }),
  },
  music: {
    browse: (libraryId: string, artist: string | undefined, album: string | undefined, cursor: string | null, signal?: AbortSignal) =>
      request<MusicPage>(`/libraries/${enc(libraryId)}/music`, { signal, query: { artist, album, cursor } }),
  },
  jobs: {
    list: (limit?: number, signal?: AbortSignal) => request<JobList>("/jobs", { signal, query: { limit } }),
  },
  plugins: {
    list: (signal?: AbortSignal) => request<PluginList>("/plugins", { signal }),
    catalog: (signal?: AbortSignal) => request<PluginCatalogResponse>("/plugins/catalog", { signal }),
    install: (body: InstallPluginRequest) => request<Plugin>("/plugins/install", { method: "POST", body }),
    update: (id: string, body: UpdatePluginRequest) =>
      request<Plugin>(`/plugins/${enc(id)}`, { method: "PATCH", body }),
    rollback: (id: string) => request<Plugin>(`/plugins/${enc(id)}/rollback`, { method: "POST" }),
    uninstall: (id: string) => request<void>(`/plugins/${enc(id)}`, { method: "DELETE" }),
  },
  skins: {
    installed: (signal?: AbortSignal) => request<{ skins: InstalledSkin[] }>("/skins/installed", { signal }),
    registry: (signal?: AbortSignal) => request<{ skins: RegistrySkin[] }>("/skins/registry", { signal }),
    install: (id: string, archive?: Blob) => request<InstalledSkin>(`/skins/registry/${enc(id)}/install`, { method: "POST", body: archive }),
    upload: (archive: Blob) => request<InstalledSkin>("/skins/upload", { method: "POST", body: archive }),
    uninstall: (id: string) => request<void>(`/skins/installed/${enc(id)}`, { method: "DELETE" }),
  },
};
