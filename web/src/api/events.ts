import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { useEffect, useSyncExternalStore } from "react";
import { API_BASE, handleUnauthorized } from "./client";
import { isActiveJob, upsertJob } from "./queries/jobs";
import { removeLibrary, upsertLibrary } from "./queries/libraries";
import { queryKeys } from "./query-keys";
import { getToken } from "./session";
import { connectSse, type SseConnectionState, type SseMessage } from "./sse";
import type { Library, LibraryList, ServerEventMap } from "./types";

/* ---------------------------------------------------------------- connection status store */

let status: SseConnectionState = "closed";
const statusListeners = new Set<() => void>();

export interface PlaybackCommandEvent {
  targetSessionId: string;
  command: "play" | "pause" | "toggle" | "seek" | "next" | "prev" | "volume" | "play_item";
  positionMs?: number | null;
  volume?: number | null;
  itemId?: string | null;
  issuedBy: string;
}

const playbackCommandListeners = new Set<(event: PlaybackCommandEvent) => void>();

/** Subscribes to live `playback.command` SSE frames delivered on the shared shell stream. */
export function subscribePlaybackCommands(listener: (event: PlaybackCommandEvent) => void): () => void {
  playbackCommandListeners.add(listener);
  return () => playbackCommandListeners.delete(listener);
}

function setStatus(next: SseConnectionState) {
  if (status === next) return;
  status = next;
  for (const listener of statusListeners) listener();
}

/** Live connection state of the event stream, for "live" indicators and polling fallbacks. */
export function useEventStreamStatus(): SseConnectionState {
  return useSyncExternalStore(
    (listener) => {
      statusListeners.add(listener);
      return () => statusListeners.delete(listener);
    },
    () => status,
    () => status,
  );
}

/* ---------------------------------------------------------------- cache updates */

function parseData<K extends keyof ServerEventMap>(message: SseMessage): ServerEventMap[K] | null {
  try {
    return JSON.parse(message.data) as ServerEventMap[K];
  } catch {
    return null;
  }
}

/** Applies one server event to the query cache. Exported for tests. */
export function applyServerEvent(queryClient: QueryClient, message: SseMessage) {
  switch (message.event) {
    case "stream.reset":
      void queryClient.invalidateQueries();
      break;
    case "settings.updated":
      void queryClient.invalidateQueries({ queryKey: ["system"] });
      break;
    case "job.updated": {
      const job = parseData<"job.updated">(message);
      if (job) {
        upsertJob(queryClient, job);
        if (job.kind === "library.scan" && job.subject?.type === "library" && !isActiveJob(job)) {
          void queryClient.invalidateQueries({ queryKey: queryKeys.libraries.scanReviews(job.subject.id) });
        }
        if (job.kind === "music.loudness" && job.subject?.type === "library" && !isActiveJob(job)) {
          void queryClient.invalidateQueries({ queryKey: queryKeys.items.loudness(job.subject.id) });
        }
      }
      break;
    }
    case "library.updated": {
      const library = parseData<"library.updated">(message);
      if (!library) break;
      const previous =
        queryClient.getQueryData<Library>(queryKeys.libraries.detail(library.id)) ??
        queryClient.getQueryData<LibraryList>(queryKeys.libraries.list)?.libraries.find((entry) => entry.id === library.id);
      upsertLibrary(queryClient, library);
      // A finished scan changes the item tree: refetch whatever is being browsed.
      if (
        !previous ||
        previous.lastScanAt !== library.lastScanAt ||
        previous.itemCount !== library.itemCount
      ) {
        void queryClient.invalidateQueries({ queryKey: queryKeys.items.libraryLists(library.id) });
      }
      break;
    }
    case "library.deleted": {
      const payload = parseData<"library.deleted">(message);
      if (payload) removeLibrary(queryClient, payload.id);
      break;
    }
    case "playback.session.updated": {
      void queryClient.invalidateQueries({ queryKey: queryKeys.cast.sessions });
      break;
    }
    case "playback.command": {
      try {
        const payload = JSON.parse(message.data) as PlaybackCommandEvent;
        if (payload && typeof payload.targetSessionId === "string" && typeof payload.command === "string") {
          for (const listener of playbackCommandListeners) listener(payload);
        }
      } catch {
        // Ignore malformed command payload.
      }
      break;
    }
    default:
      // `ping` keeps the connection alive; unknown events are ignored.
      break;
  }
}

export type ExtraEventHandlers = Record<string, Array<(queryClient: QueryClient, data: unknown) => void>>;

/**
 * Keeps one event stream open while `enabled` (signed in) and mirrors events into the cache.
 * `extra` handles event names outside the M0 contract (contributed by modules). After a
 * reconnect with no prior cursor or a server reset, active queries refetch current state.
 */
const NO_EXTRA: ExtraEventHandlers = {};

export function useServerEvents(enabled: boolean, extra: ExtraEventHandlers = NO_EXTRA) {
  const queryClient = useQueryClient();
  useEffect(() => {
    if (!enabled) return;
    return connectSse({
      url: `${API_BASE}/events`,
      getToken,
      onMessage: (message) => {
        applyServerEvent(queryClient, message);
        const handlers = extra[message.event];
        if (!handlers) return;
        let data: unknown;
        try {
          data = JSON.parse(message.data);
        } catch {
          return;
        }
        for (const handler of handlers) handler(queryClient, data);
      },
      onStateChange: setStatus,
      onReconnect: (cursor) => {
        if (!cursor) void queryClient.invalidateQueries();
      },
      onUnauthorized: handleUnauthorized,
    });
  }, [enabled, queryClient, extra]);
}
