/** Query keys for PROVISIONAL resources (kept apart from the M0 keys in api/query-keys.ts). */
export const provisionalKeys = {
  tasks: ["provisional", "tasks"] as const,
  conversation: ["provisional", "assistant", "conversation"] as const,
  suggestions: ["provisional", "assistant", "suggestions"] as const,
  assistantConfig: ["provisional", "assistant", "config"] as const,
  assistantMemories: ["provisional", "assistant", "memories"] as const,
  home: ["provisional", "home"] as const,
  homeBridges: ["provisional", "home", "bridges"] as const,
  storage: ["provisional", "storage"] as const,
  catalog: ["provisional", "catalog"] as const,
  artwork: (id: string) => ["provisional", "artwork", id] as const,
  metrics: ["provisional", "metrics"] as const,
  mcpInfo: ["provisional", "mcp", "info"] as const,
  downloads: ["provisional", "downloads"] as const,
  organizerBatches: ["provisional", "organizer", "batches"] as const,
};
