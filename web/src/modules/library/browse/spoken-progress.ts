import { ApiError } from "@/api/errors";
import type { components } from "@/api/generated";

type Update = components["schemas"]["PlaybackUpdate"];
type Position = Pick<Update, "positionMs" | "completed">;

/** A seek or pause can arrive while the last heartbeat still owns the current version. */
export function spokenProgress(
  initial: { playbackVersion: number; positionMs: number },
  write: (value: Update) => Promise<{ playbackVersion: number }>,
  onError: (error: unknown) => void,
) {
  let version = initial.playbackVersion;
  let saved: Position = { positionMs: initial.positionMs, completed: false };
  let pending: Position | undefined;
  let writing = false;
  let conflicted = false;

  async function drain() {
    writing = true;
    try {
      while (pending) {
        const value = pending;
        pending = undefined;
        if (value.positionMs === saved.positionMs && value.completed === saved.completed) continue;
        const result = await write({ ...value, expectedVersion: version });
        version = result.playbackVersion;
        saved = value;
      }
    } catch (error) {
      conflicted = error instanceof ApiError && error.status === 409;
      pending = undefined;
      onError(error);
    } finally {
      writing = false;
    }
  }
  return (seconds: number, completed: boolean) => {
    if (conflicted || !Number.isFinite(seconds) || seconds < 0) return;
    pending = { positionMs: completed ? 0 : Math.round(seconds * 1000), completed };
    if (!writing) void drain();
  };
}
