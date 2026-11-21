import { describe, expect, it, vi } from "vitest";
import { ApiError } from "@/api/errors";
import { spokenProgress } from "./spoken-progress";

describe("spoken playback progress", () => {
  it("sends the final position after an in-flight checkpoint and uses the returned version", async () => {
    let release: (value: { playbackVersion: number }) => void = () => {};
    const write = vi.fn()
      .mockImplementationOnce(() => new Promise((resolve) => { release = resolve; }))
      .mockResolvedValueOnce({ playbackVersion: 32 });
    const failed = vi.fn();
    const save = spokenProgress({ positionMs: 0, playbackVersion: 30 }, write, failed);
    save(15, false);
    save(18, false);
    save(0, true);
    expect(write).toHaveBeenCalledTimes(1);
    release({ playbackVersion: 31 });
    await vi.waitFor(() => expect(write).toHaveBeenCalledTimes(2));
    expect(write.mock.calls).toEqual([
      [{ expectedVersion: 30, positionMs: 15_000, completed: false }],
      [{ expectedVersion: 31, positionMs: 0, completed: true }],
    ]);
    save(0, true);
    expect(write).toHaveBeenCalledTimes(2);
    expect(failed).not.toHaveBeenCalled();
  });

  it("keeps a newer device's resume point after a version conflict", async () => {
    const conflict = new ApiError(409, "conflict", "progress changed");
    const write = vi.fn().mockRejectedValue(conflict);
    const failed = vi.fn();
    const save = spokenProgress({ positionMs: 2_000, playbackVersion: 5 }, write, failed);
    save(2, false);
    expect(write).not.toHaveBeenCalled();
    save(3, false);
    await vi.waitFor(() => expect(failed).toHaveBeenCalledWith(conflict));
    save(4, false);
    expect(write).toHaveBeenCalledTimes(1);
  });
});
