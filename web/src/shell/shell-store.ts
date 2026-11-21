import { useSyncExternalStore } from "react";

interface ShellState {
  paletteOpen: boolean;
  inboxOpen: boolean;
}

let state: ShellState = { paletteOpen: false, inboxOpen: false };
const listeners = new Set<() => void>();

function set(next: Partial<ShellState>) {
  state = { ...state, ...next };
  for (const listener of listeners) listener();
}

export const shell = {
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
