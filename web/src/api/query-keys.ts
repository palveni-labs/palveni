/** Query keys per resource. Prefix keys (`all`) are used for broad invalidation. */
export const queryKeys = {
  system: {
    info: ["system", "info"] as const,
    settings: ["system", "settings"] as const,
    hardware: ["system", "hardware"] as const,
  },
  auth: {
    me: ["auth", "me"] as const,
  },
  libraries: {
    all: ["libraries"] as const,
    list: ["libraries", "list"] as const,
    detail: (id: string) => ["libraries", "detail", id] as const,
    /** Client-side record of the scan job last started for a library (jobs carry no library id). */
    scanJob: (id: string) => ["libraries", "scan-job", id] as const,
  },
  items: {
    all: ["items"] as const,
    lists: ["items", "list"] as const,
    music: (libraryId: string, artist?: string, album?: string) => ["items", "list", libraryId, "music", artist, album] as const,
    libraryLists: (libraryId: string) => ["items", "list", libraryId] as const,
    list: (libraryId: string, parentId: string | null) => ["items", "list", libraryId, parentId ?? "root"] as const,
    detail: (id: string) => ["items", "detail", id] as const,
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
    list: ["plugins", "list"] as const,
  },
};
