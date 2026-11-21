import { request } from "./client";
import type {
  CreateLibraryRequest,
  UpdateLibraryRequest,
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
  Library,
  LibraryList,
  LoginRequest,
  LoginResponse,
  PluginList,
  ScanResponse,
  SetupResponse,
  InstalledSkin,
  RegistrySkin,
  RuntimeSettings,
  SystemInfo,
  User,
} from "./types";

const enc = encodeURIComponent;

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
  },
  libraries: {
    list: (signal?: AbortSignal) => request<LibraryList>("/libraries", { signal }),
    get: (id: string, signal?: AbortSignal) => request<Library>(`/libraries/${enc(id)}`, { signal }),
    create: (body: CreateLibraryRequest) => request<Library>("/libraries", { method: "POST", body }),
    update: (id: string, body: UpdateLibraryRequest) => request<Library>(`/libraries/${enc(id)}`, { method: "PATCH", body }),
    remove: (id: string) => request<void>(`/libraries/${enc(id)}`, { method: "DELETE" }),
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
  },
  skins: {
    installed: (signal?: AbortSignal) => request<{ skins: InstalledSkin[] }>("/skins/installed", { signal }),
    registry: (signal?: AbortSignal) => request<{ skins: RegistrySkin[] }>("/skins/registry", { signal }),
    install: (id: string, archive?: Blob) => request<InstalledSkin>(`/skins/registry/${enc(id)}/install`, { method: "POST", body: archive }),
    upload: (archive: Blob) => request<InstalledSkin>("/skins/upload", { method: "POST", body: archive }),
    uninstall: (id: string) => request<void>(`/skins/installed/${enc(id)}`, { method: "DELETE" }),
  },
};
