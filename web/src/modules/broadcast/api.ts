import { request } from "@/api/client";
import type { components } from "@/api/generated";

type Schemas = components["schemas"];
export type BroadcastKind = Schemas["BroadcastKind"];
export type BroadcastSource = Schemas["BroadcastSource"];
export type BroadcastImport = Schemas["BroadcastImport"];
export type BroadcastEntry = Schemas["BroadcastEntry"];

const root = "/broadcast/sources";
const path = (id: string) => root + "/" + encodeURIComponent(id);

export const broadcastApi = {
  list: (kind: BroadcastKind, signal?: AbortSignal) =>
    request<Schemas["BroadcastSourceList"]>(root, { query: { kind }, signal }),
  entries: (id: string, version: number, offset: number, signal?: AbortSignal) =>
    request<Schemas["BroadcastEntryPage"]>(path(id) + "/entries", {
      query: { version, offset, limit: 50 },
      signal,
    }),
  save: (id: string, body: BroadcastImport) =>
    request<BroadcastSource>(path(id), { method: "PUT", body }),
  remove: (source: BroadcastSource) =>
    request<void>(path(source.id), { method: "DELETE", query: { version: source.version } }),
  favorite: (source: string, entry: string, favorite: boolean) =>
    request<void>(path(source) + "/favorites/" + encodeURIComponent(entry), {
      method: "PUT",
      body: { favorite },
    }),
};

export function newSourceId(): string {
  // A household's plain HTTP LAN origin has getRandomValues but may lack randomUUID.
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6]! & 15) | 64;
  bytes[8] = (bytes[8]! & 63) | 128;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20),
  ].join("-");
}
