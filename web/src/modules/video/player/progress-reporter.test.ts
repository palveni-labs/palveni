import { describe, expect, it, vi } from "vitest";
import { ApiError } from "@/api/errors";
import { createProgressReporter, type PlaybackUpdate } from "./progress-reporter";

describe("createProgressReporter", () => {
  it("throttles periodic ticks to 10s intervals and increments expectedVersion", async () => {
    let clock = 10_000;
    const calls: PlaybackUpdate[] = [];
    let serverVersion = 2;

    const reporter = createProgressReporter({
      initialVersion: serverVersion,
      initialPositionMs: 0,
      intervalMs: 10_000,
      now: () => clock,
      write: async (update) => {
        calls.push(update);
        serverVersion += 1;
        return Promise.resolve({ playbackVersion: serverVersion });
      },
    });

    reporter.tick(5);
    await vi.waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0]).toEqual({ positionMs: 5000, completed: false, expectedVersion: 2 });

    // Within 10s window -> ignored
    clock += 4_000;
    reporter.tick(9);
    await Promise.resolve();
    expect(calls).toHaveLength(1);

    // Past 10s window -> sent with updated expectedVersion
    clock += 7_000;
    reporter.tick(16);
    await vi.waitFor(() => expect(calls).toHaveLength(2));
    expect(calls[1]).toEqual({ positionMs: 16_000, completed: false, expectedVersion: 3 });
  });

  it("flushes immediately on pause/leave/ended and resets position to 0 on completion", async () => {
    let clock = 1_000;
    const calls: PlaybackUpdate[] = [];
    const reporter = createProgressReporter({
      initialVersion: 0,
      initialPositionMs: 0,
      now: () => clock,
      write: async (update) => {
        calls.push(update);
        return Promise.resolve({ playbackVersion: calls.length });
      },
    });

    reporter.flush(42.5, false);
    await vi.waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0]).toEqual({ positionMs: 42_500, completed: false, expectedVersion: 0 });

    clock += 1000;
    reporter.flush(120, true);
    await vi.waitFor(() => expect(calls).toHaveLength(2));
    expect(calls[1]).toEqual({ positionMs: 0, completed: true, expectedVersion: 1 });
  });

  it("stops reporting after a 409 conflict until version is resynced", async () => {
    const onError = vi.fn();
    const write = vi
      .fn<(update: PlaybackUpdate) => Promise<{ playbackVersion: number }>>()
      .mockRejectedValueOnce(new ApiError(409, "conflict", "Version conflict"))
      .mockResolvedValueOnce({ playbackVersion: 6 });

    const reporter = createProgressReporter({
      initialVersion: 1,
      initialPositionMs: 0,
      write,
      onError,
    });

    reporter.flush(12, false);
    await vi.waitFor(() => expect(onError).toHaveBeenCalledTimes(1));

    reporter.flush(24, false);
    await Promise.resolve();
    expect(write).toHaveBeenCalledTimes(1);

    reporter.syncVersion(5, 20_000);
    reporter.flush(30, false);
    await vi.waitFor(() => expect(write).toHaveBeenCalledTimes(2));
    expect(write).toHaveBeenLastCalledWith({ positionMs: 30_000, completed: false, expectedVersion: 5 });
  });
});
