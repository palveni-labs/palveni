import { useServerEvents } from "@/api/events";
import { MODULES } from "@/modules/registry";
import type { EventHandler } from "@/modules/types";

/** Event handlers contributed by modules, merged by event name. */
const HANDLERS: Record<string, EventHandler[]> = {};
for (const module of MODULES) {
  for (const [event, handler] of Object.entries(module.events ?? {})) (HANDLERS[event] ??= []).push(handler);
}

/** One SSE connection for the whole shell: M0 events plus every module's own events. */
export function useShellEvents() {
  useServerEvents(true, HANDLERS);
}
