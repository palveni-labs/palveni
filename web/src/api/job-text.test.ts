import { describe, expect, it } from "vitest";
import i18n from "@/i18n";
import { jobDetail, scanGaps, scanGapsText, stepText, subjectTitle } from "./job-text";
import type { Job } from "./types";

const t = i18n.t.bind(i18n);

function job(patch: Partial<Job>): Job {
  return {
    id: "j",
    kind: "library.scan",
    state: "running",
    subject: { type: "library", id: "l", name: "家庭相册" },
    progress: null,
    step: null,
    message: null,
    error: null,
    createdAt: "2026-11-20T08:00:00Z",
    startedAt: null,
    finishedAt: null,
    ...patch,
  };
}

describe("job text", () => {
  it("titles a scan by its library", () => {
    expect(subjectTitle(t, "library.scan", { type: "library", id: "l", name: "家庭相册" })).toBe("扫描「家庭相册」");
    expect(subjectTitle(t, "library.scan", null)).toBeNull();
    expect(subjectTitle(t, "library.scan", { type: "playlist", id: "p", name: "x" })).toBeNull();
  });

  it("localises known step codes with their parameters", () => {
    expect(stepText(t, { code: "scan.discovering", params: { found: 312 } }, "Discovered 312")).toBe("已发现 312 个文件");
    expect(stepText(t, { code: "scan.indexing", params: { done: 40, total: 100 } }, null)).toBe("已索引 40 / 100 个条目");
    expect(stepText(t, { code: "scan.summary", params: { added: 3, updated: 1, removed: 0, unchanged: 9 } }, null)).toBe("新增 3，更新 1，移除 0，未变 9");
  });

  it("falls back to the server's message for unknown codes, and to the error when failed", () => {
    expect(stepText(t, { code: "scan.hashing", params: {} }, "Hashing files")).toBe("Hashing files");
    expect(jobDetail(t, job({ state: "failed", error: "Permission denied" }))).toBe("Permission denied");
    expect(jobDetail(t, job({ state: "cancelled" }))).toBeNull();
  });

  it("finds what a scan could not read, and says the items there were kept", () => {
    const clean = { code: "scan.summary", params: { added: 3, updated: 0, removed: 0, unchanged: 9, unavailablePaths: 0, unreadableFolders: 0 } };
    expect(scanGaps(clean)).toBeNull();
    expect(scanGaps({ code: "scan.summary", params: { added: 3, updated: 0, removed: 0, unchanged: 9 } })).toBeNull();
    expect(scanGaps({ code: "scan.indexing", params: { unavailablePaths: 2 } })).toBeNull();

    const offline = scanGaps({ ...clean, params: { ...clean.params, unavailablePaths: 1 } });
    expect(offline).toEqual({ unavailablePaths: 1, unreadableFolders: 0 });
    expect(scanGapsText(t, offline!)).toBe("1 个路径不可用，其中的条目保持不变");

    const both = scanGaps({ ...clean, params: { ...clean.params, unavailablePaths: 2, unreadableFolders: "3" } });
    expect(scanGapsText(t, both!)).toBe("2 个路径不可用、3 个文件夹无法读取，其中的条目保持不变");
  });
});
