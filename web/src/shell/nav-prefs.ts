/**
 * User-customizable navigation item ordering and visibility, persisted per browser.
 * Core modules (`overview`, `system`) are locked visible so users can never lock themselves out.
 */
import { useSyncExternalStore } from "react";
import { readLocal, removeLocal, writeLocal } from "@/lib/storage";
import type { ModuleDefinition } from "@/modules/types";

const NAV_PREFS_KEY = "palveni.nav.prefs";

/** Modules that cannot be hidden from navigation. */
export const LOCKED_NAV_MODULES: ReadonlySet<string> = new Set(["overview", "system"]);

export interface NavPrefs {
  /** Custom module order; unknown ids are ignored and newly registered modules append at the end. */
  order: string[];
  /** Module ids hidden from primary navigation (still reachable via command palette and direct URL). */
  hidden: string[];
}

const EMPTY_PREFS: NavPrefs = { order: [], hidden: [] };

const listeners = new Set<() => void>();

function parsePrefs(raw: string | null): NavPrefs {
  if (!raw) return EMPTY_PREFS;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return EMPTY_PREFS;
    const record = parsed as Record<string, unknown>;
    const order = Array.isArray(record.order) ? record.order.filter((id): id is string => typeof id === "string") : [];
    const hidden = Array.isArray(record.hidden)
      ? record.hidden.filter((id): id is string => typeof id === "string" && !LOCKED_NAV_MODULES.has(id))
      : [];
    return { order, hidden };
  } catch {
    return EMPTY_PREFS;
  }
}

let state: NavPrefs = typeof window !== "undefined" ? parsePrefs(readLocal(NAV_PREFS_KEY)) : EMPTY_PREFS;

function emit() {
  for (const listener of listeners) listener();
}

function save(next: NavPrefs) {
  const sanitized: NavPrefs = {
    order: [...new Set(next.order)],
    hidden: [...new Set(next.hidden.filter((id) => !LOCKED_NAV_MODULES.has(id)))],
  };
  state = sanitized;
  if (sanitized.order.length === 0 && sanitized.hidden.length === 0) removeLocal(NAV_PREFS_KEY);
  else writeLocal(NAV_PREFS_KEY, JSON.stringify(sanitized));
  emit();
}

/** Orders modules by user preference, appending any unconfigured modules in their registry order. */
export function orderModules(modules: readonly ModuleDefinition[], prefs: NavPrefs): ModuleDefinition[] {
  if (prefs.order.length === 0) return [...modules];
  const rank = new Map(prefs.order.map((id, index) => [id, index]));
  const last = prefs.order.length;
  return [...modules].sort((a, b) => (rank.get(a.id) ?? last) - (rank.get(b.id) ?? last) || a.order - b.order);
}

/** Applies custom order and filters out user-hidden modules (locked modules stay visible). */
export function applyNavPrefs(modules: readonly ModuleDefinition[], prefs: NavPrefs): ModuleDefinition[] {
  const hidden = new Set(prefs.hidden.filter((id) => !LOCKED_NAV_MODULES.has(id)));
  return orderModules(modules, prefs).filter((module) => !hidden.has(module.id));
}

export function moveNavModule(id: string, delta: -1 | 1, allIds: readonly string[]) {
  const index = allIds.indexOf(id);
  const target = allIds[index + delta];
  if (index !== -1 && target !== undefined) reorderNavModule(id, target, allIds);
}

export function reorderNavModule(fromId: string, toId: string, allIds: readonly string[]) {
  const fromIndex = allIds.indexOf(fromId);
  const toIndex = allIds.indexOf(toId);
  if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return;
  const reordered = [...allIds];
  reordered.splice(fromIndex, 1);
  reordered.splice(toIndex, 0, fromId);

  // Preserve the positions of modules this account cannot access.
  const accessible = new Set(allIds);
  const previous = new Set([...state.order, ...allIds]);
  let next = 0;
  const order = [...previous].map((id) => accessible.has(id) ? reordered[next++]! : id);
  save({ ...state, order });
}

export function toggleNavModuleHidden(id: string) {
  if (LOCKED_NAV_MODULES.has(id)) return;
  const hidden = state.hidden.includes(id) ? state.hidden.filter((entry) => entry !== id) : [...state.hidden, id];
  save({ ...state, hidden });
}

export function resetNavPrefs() {
  state = EMPTY_PREFS;
  removeLocal(NAV_PREFS_KEY);
  emit();
}

export function isNavCustomized(prefs: NavPrefs): boolean {
  return prefs.order.length > 0 || prefs.hidden.length > 0;
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

const getSnapshot = () => state;

export function useNavPrefs(): NavPrefs {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
