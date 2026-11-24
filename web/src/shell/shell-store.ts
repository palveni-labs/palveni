import { useSyncExternalStore } from "react";

/** What the user handed to the assistant (e.g. a selection in a library). */
export interface AssistantHandoff {
  label: string;
  itemIds?: string[];
  prompt?: string;
  /** Send the prompt right away (suggestions, "ask the assistant" from ⌘K). */
  autoSend?: boolean;
}

interface ShellState {
  panelOpen: boolean;
  paletteOpen: boolean;
  inboxOpen: boolean;
  handoff: AssistantHandoff | null;
}

let state: ShellState = { panelOpen: false, paletteOpen: false, inboxOpen: false, handoff: null };
const listeners = new Set<() => void>();

function set(next: Partial<ShellState>) {
  state = { ...state, ...next };
  for (const listener of listeners) listener();
}

export const shell = {
  openAssistant(handoff?: AssistantHandoff | string) {
    const next = typeof handoff === "string" ? { label: "", prompt: handoff, autoSend: true } : (handoff ?? null);
    set({ panelOpen: true, paletteOpen: false, handoff: next ?? state.handoff });
  },
  closeAssistant() {
    set({ panelOpen: false });
  },
  toggleAssistant() {
    set({ panelOpen: !state.panelOpen });
  },
  /** The assistant consumed the handoff (prompt sent or dismissed). */
  clearHandoff() {
    set({ handoff: null });
  },
  setPalette(open: boolean) {
    set({ paletteOpen: open });
  },
  setInbox(open: boolean) {
    set({ inboxOpen: open });
  },
};

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useShellState(): ShellState {
  return useSyncExternalStore(subscribe, () => state, () => state);
}
