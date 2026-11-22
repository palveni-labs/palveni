/**
 * Version-serialized progress reporter for video playback:
 * - Periodic ticks report at most every `intervalMs` (default 10s).
 * - Immediate flushes (pause, seek, ended, unmount) bypass the throttle while still coalescing
 *   concurrent in-flight writes so `expectedVersion` never races.
 */
import { ApiError } from "@/api/errors";
import type { components } from "@/api/generated";

export type PlaybackUpdate = components["schemas"]["PlaybackUpdate"];

interface PendingWrite {
  positionMs: number;
  completed: boolean;
}

export interface ProgressReporterOptions {
  initialVersion: number;
  initialPositionMs: number;
  intervalMs?: number;
  now?: () => number;
  write: (update: PlaybackUpdate) => Promise<{ playbackVersion: number }>;
  onError?: (error: unknown) => void;
}

export interface ProgressReporter {
  /** Called on timeupdate; sends at most once per `intervalMs`. */
  tick: (positionSeconds: number) => void;
  /** Sends immediately (pause, seek, ended, leave). */
  flush: (positionSeconds: number, completed?: boolean) => void;
  /** Updates optimistic version after reloading personal state. */
  syncVersion: (version: number, positionMs: number) => void;
  /** Last reported position in ms. */
  lastPositionMs: () => number;
}

export function createProgressReporter(options: ProgressReporterOptions): ProgressReporter {
  const intervalMs = options.intervalMs ?? 10_000;
  const now = options.now ?? (() => Date.now());
  let version = options.initialVersion;
  let saved: PendingWrite = { positionMs: options.initialPositionMs, completed: false };
  let pending: PendingWrite | undefined;
  let writing = false;
  let conflicted = false;
  let lastReportAt = 0;

  async function drain() {
    if (writing) return;
    writing = true;
    try {
      while (pending) {
        const next = pending;
        pending = undefined;
        if (next.positionMs === saved.positionMs && next.completed === saved.completed) continue;
        const result = await options.write({
          positionMs: next.positionMs,
          completed: next.completed,
          expectedVersion: version,
        });
        version = result.playbackVersion;
        saved = next;
        lastReportAt = now();
      }
    } catch (error) {
      if (error instanceof ApiError && error.status === 409) {
        conflicted = true;
      }
      pending = undefined;
      options.onError?.(error);
    } finally {
      writing = false;
    }
  }

  function enqueue(positionSeconds: number, completed: boolean) {
    if (conflicted || !Number.isFinite(positionSeconds) || positionSeconds < 0) return;
    pending = {
      positionMs: completed ? 0 : Math.round(positionSeconds * 1000),
      completed,
    };
    if (!writing) void drain();
  }

  return {
    tick(positionSeconds: number) {
      if (conflicted || !Number.isFinite(positionSeconds) || positionSeconds < 0) return;
      if (now() - lastReportAt < intervalMs) return;
      lastReportAt = now();
      enqueue(positionSeconds, false);
    },
    flush(positionSeconds: number, completed = false) {
      lastReportAt = now();
      enqueue(positionSeconds, completed);
    },
    syncVersion(nextVersion: number, positionMs: number) {
      version = nextVersion;
      saved = { positionMs, completed: false };
      conflicted = false;
    },
    lastPositionMs() {
      return pending?.positionMs ?? saved.positionMs;
    },
  };
}
