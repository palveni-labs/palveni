/** Query keys per resource. Prefix keys (`all`) are used for broad invalidation. */
export const queryKeys = {
  system: {
    info: ["system", "info"] as const,
    settings: ["system", "settings"] as const,
    hardware: ["system", "hardware"] as const,
  },
  auth: {
    me: ["auth", "me"] as const,
    sessions: (userId: string) => ["auth", "sessions", userId] as const,
  },
  users: {
    all: ["users"] as const,
    list: ["users", "list"] as const,
  },
  maintenance: {
    auditAll: ["maintenance", "audit"] as const,
    audit: (action?: string) => ["maintenance", "audit", action ?? ""] as const,
    backups: ["maintenance", "backups"] as const,
    doctor: ["maintenance", "doctor"] as const,
    logsAll: ["maintenance", "logs"] as const,
    logs: (level?: string) => ["maintenance", "logs", level ?? "all"] as const,
    metadataConfig: ["maintenance", "metadata-config"] as const,
  },
  metadata: {
    override: (itemId: string) => ["metadata", "override", itemId] as const,
  },
  libraries: {
    all: ["libraries"] as const,
    list: ["libraries", "list"] as const,
    detail: (id: string) => ["libraries", "detail", id] as const,
    scanReviews: (id: string) => ["libraries", "scan-reviews", id] as const,
    reviewPaths: (id: string, revision: string, offset: number) => ["libraries", "scan-reviews", id, revision, offset] as const,
    /** Client-side record of the scan job last started for a library (jobs carry no library id). */
    scanJob: (id: string) => ["libraries", "scan-job", id] as const,
  },
  items: {
    all: ["items"] as const,
    lists: ["items", "list"] as const,
    globalMusicSearch: ["items", "global-music-search"] as const,
    music: (libraryId: string, artist?: string, album?: string) => ["items", "list", libraryId, "music", artist, album] as const,
    libraryLists: (libraryId: string) => ["items", "list", libraryId] as const,
    list: (libraryId: string, parentId: string | null) => ["items", "list", libraryId, parentId ?? "root"] as const,
    detail: (id: string) => ["items", "detail", id] as const,
    loudness: (libraryId: string) => ["items", "list", libraryId, "loudness"] as const,
    ancestors: (id: string) => ["items", "ancestors", id] as const,
  },
  jobs: {
    all: ["jobs"] as const,
    list: (limit: number) => ["jobs", "list", limit] as const,
  },
  skins: {
    all: ["skins"] as const,
    installed: ["skins", "installed"] as const,
    registry: ["skins", "registry"] as const,
  },
  plugins: {
    all: ["plugins"] as const,
    list: ["plugins", "list"] as const,
    catalog: ["plugins", "catalog"] as const,
  },
  cast: {
    sessions: ["cast", "sessions"] as const,
  },
  mqtt: {
    bridges: ["home", "mqtt", "bridges"] as const,
  },
};
