import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import { applyServerEvent } from "./events";
import { queryKeys } from "./query-keys";
import type { Job, JobList, Library, LibraryList } from "./types";

const library: Library = {
  id: "l1",
  name: "电影",
  kind: "video",
  paths: ["/srv/movies"],
  itemCount: 0,
  lastScanAt: null,
  createdAt: "2026-11-20T08:00:00Z",
  updatedAt: "2026-11-20T08:00:00Z",
};

const job: Job = {
  id: "j1",
  kind: "library.scan",
  state: "running",
  subject: { type: "library", id: "lib-1", name: "电影" },
  progress: 0.4,
  step: { code: "scan.indexing", params: { done: 40, total: 100 } },
  message: "Indexed 40 of 100 files",
  error: null,
  createdAt: "2026-11-20T08:30:00Z",
  startedAt: "2026-11-20T08:30:01Z",
  finishedAt: null,
};

const event = (name: string, data: unknown) => ({ event: name, data: JSON.stringify(data), id: null });

describe("applyServerEvent", () => {
  it("upserts jobs into cached lists, newest first", () => {
    const client = new QueryClient();
    const older = { ...job, id: "j0", createdAt: "2026-11-19T08:00:00Z", state: "succeeded" as const };
    client.setQueryData<JobList>(queryKeys.jobs.list(50), { jobs: [older] });
    applyServerEvent(client, event("job.updated", job));
    applyServerEvent(client, event("job.updated", { ...job, progress: 0.9 }));
    const jobs = client.getQueryData<JobList>(queryKeys.jobs.list(50))?.jobs;
    expect(jobs?.map((entry) => entry.id)).toEqual(["j1", "j0"]);
    expect(jobs?.[0]?.progress).toBe(0.9);
  });

  it("updates libraries and invalidates their items after a scan", () => {
    const client = new QueryClient();
    client.setQueryData<LibraryList>(queryKeys.libraries.list, { libraries: [library] });
    client.setQueryData(queryKeys.items.list("l1", null), { pages: [], pageParams: [] });
    const scanned = { ...library, itemCount: 12, lastScanAt: "2026-11-20T08:31:00Z" };
    applyServerEvent(client, event("library.updated", scanned));
    expect(client.getQueryData<LibraryList>(queryKeys.libraries.list)?.libraries[0]).toEqual(scanned);
    expect(client.getQueryData(queryKeys.libraries.detail("l1"))).toEqual(scanned);
    expect(client.getQueryState(queryKeys.items.list("l1", null))?.isInvalidated).toBe(true);
  });

  it("invalidates all cached state on a lost cursor, including items", () => {
    const client = new QueryClient();
    const keys = [queryKeys.jobs.list(50), queryKeys.items.detail("i1"), queryKeys.libraries.detail("l1"), queryKeys.system.info];
    keys.forEach((key) => client.setQueryData(key, {}));
    applyServerEvent(client, event("settings.updated", { serverName: null, perfTier: null }));
    expect(client.getQueryState(queryKeys.system.info)?.isInvalidated).toBe(true);
    expect(client.getQueryState(queryKeys.items.detail("i1"))?.isInvalidated).toBe(false);
    applyServerEvent(client, event("stream.reset", {}));
    expect(keys.every((key) => client.getQueryState(key)?.isInvalidated)).toBe(true);
  });

  it("removes deleted libraries and ignores pings and malformed data", () => {
    const client = new QueryClient();
    client.setQueryData<LibraryList>(queryKeys.libraries.list, { libraries: [library] });
    applyServerEvent(client, event("ping", {}));
    applyServerEvent(client, { event: "library.updated", data: "{not json", id: null });
    applyServerEvent(client, event("library.deleted", { id: "l1" }));
    expect(client.getQueryData<LibraryList>(queryKeys.libraries.list)?.libraries).toEqual([]);
  });
});
