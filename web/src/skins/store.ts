/**
 * The active skin and the light/dark setting, applied to the document at runtime.
 *
 * The built-in skin is always available. Any other skin is a plugin installed on the server and
 * is fetched, parsed and checked before it applies (loader.ts); if it cannot be loaded the
 * built-in skin stays in place. Values reach the page only through CSSOM `setProperty` on the
 * root element — never as stylesheet text. Switching needs no reload, and the light/dark choice
 * (follow system, light, dark) is independent of the skin. Both choices persist per browser.
 */
import { useSyncExternalStore } from "react";
import { readLocal, removeLocal, writeLocal } from "@/lib/storage";
import { skinVariables, slotAttributes } from "./css";
import { retainFonts } from "./fonts";
import { fetchInstalledSkin, SkinLoadError, type LoadOptions, type SkinLoadFailure } from "./loader";
import { BUILTIN_SKIN, DEFAULT_SKIN_ID, type SkinEntry } from "./registry";
import type { Mode } from "./schema";

export type ModeSetting = "system" | "light" | "dark";

const MODE_KEY = "palveni.theme";
const SKIN_KEY = "palveni.skin";
/** How long the first paint waits for an installed skin before showing the built-in one. */
const BOOT_WAIT_MS = 2500;

export interface SkinState {
  /** The skin the user chose (persisted). */
  skinId: string;
  mode: ModeSetting;
  resolved: Mode;
  /** The skin actually applied. */
  active: SkinEntry;
  status: "ready" | "loading" | "fallback";
  /** Why the chosen skin is not the one applied. */
  failure: SkinLoadFailure | null;
  /** Installed skins loaded so far (for previews and switching). */
  installed: Record<string, SkinEntry>;
}

const listeners = new Set<() => void>();
let media: MediaQueryList | null = null;
let releaseFonts: (() => void) | null = null;
let fontsFor: string | null = null;
let appliedNames: string[] = [];
let appliedAttributes: string[] = [];
let state: SkinState = {
  skinId: DEFAULT_SKIN_ID,
  mode: "system",
  resolved: "light",
  active: BUILTIN_SKIN,
  status: "ready",
  failure: null,
  installed: {},
};
let loader: (id: string, options?: LoadOptions) => Promise<SkinEntry> = fetchInstalledSkin;

function emit() {
  for (const listener of listeners) listener();
}

function systemMode(): Mode {
  return media?.matches ? "dark" : "light";
}

function syncFonts(entry: SkinEntry) {
  const key = `skin:${entry.manifest.id}`;
  if (fontsFor === key) return;
  releaseFonts?.();
  releaseFonts = null;
  fontsFor = key;
  void entry.loadFonts().then(
    (assets) => {
      if (fontsFor === key) releaseFonts = retainFonts(key, assets);
    },
    () => {
      // Fonts are an enhancement: the stacks fall back to system faces.
    },
  );
}

function apply() {
  const resolved = state.mode === "system" ? systemMode() : state.mode;
  state = { ...state, resolved };
  if (typeof document !== "undefined") {
    const skin = state.active.manifest;
    const root = document.documentElement;
    const { "color-scheme": _scheme, ...variables } = skinVariables(skin, resolved);
    for (const name of appliedNames) if (!(name in variables)) root.style.removeProperty(name);
    for (const [name, value] of Object.entries(variables)) root.style.setProperty(name, value);
    appliedNames = Object.keys(variables);
    root.style.colorScheme = resolved;
    for (const name of appliedAttributes) root.removeAttribute(name);
    const attributes = slotAttributes(skin);
    for (const [name, value] of Object.entries(attributes)) root.setAttribute(name, value);
    appliedAttributes = Object.keys(attributes);
    root.dataset.skin = skin.id;
    root.dataset.theme = resolved;
    root.classList.toggle("dark", resolved === "dark");
    root.classList.toggle("light", resolved === "light");
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", skin.colors[resolved].shell);
    syncFonts(state.active);
  }
  emit();
}

function cache(entry: SkinEntry) {
  if (entry.source === "installed") state = { ...state, installed: { ...state.installed, [entry.manifest.id]: entry } };
}

/** Loads an installed skin (or returns the cached one); never applies anything. */
export async function loadSkin(id: string, options?: LoadOptions): Promise<SkinEntry> {
  if (id === DEFAULT_SKIN_ID) return BUILTIN_SKIN;
  const cached = state.installed[id];
  if (cached) return cached;
  const entry = await loader(id, options);
  cache(entry);
  emit();
  return entry;
}

function fallBack(failure: SkinLoadFailure) {
  // A skin that is gone or broken is forgotten; one that is merely unreachable is retried next time.
  if (failure !== "unreachable") {
    removeLocal(SKIN_KEY);
    state = { ...state, skinId: DEFAULT_SKIN_ID };
  }
  state = { ...state, active: BUILTIN_SKIN, status: "fallback", failure };
  apply();
}

/**
 * Applies a skin by id. Built-in applies at once; an installed skin is fetched and checked
 * first. Resolves to the applied entry; on failure the built-in skin applies and the error is
 * rethrown so the caller can say why.
 */
export async function setSkin(id: string): Promise<SkinEntry> {
  state = { ...state, skinId: id, status: id === DEFAULT_SKIN_ID ? "ready" : "loading", failure: null };
  if (id === DEFAULT_SKIN_ID) removeLocal(SKIN_KEY);
  else writeLocal(SKIN_KEY, id);
  emit();
  try {
    const entry = await loadSkin(id);
    if (state.skinId !== id) return entry; // superseded by a later choice
    state = { ...state, active: entry, status: "ready", failure: null };
    apply();
    return entry;
  } catch (error) {
    if (state.skinId === id) fallBack(error instanceof SkinLoadError ? error.reason : "invalid");
    throw error;
  }
}

/** Reads stored preferences and applies them; resolves once the first skin is in place. */
export async function initSkin(): Promise<void> {
  const mode = readLocal(MODE_KEY);
  const stored = readLocal(SKIN_KEY);
  state = {
    ...state,
    mode: mode === "light" || mode === "dark" || mode === "system" ? mode : "system",
    skinId: stored ?? DEFAULT_SKIN_ID,
    active: BUILTIN_SKIN,
  };
  media = typeof window !== "undefined" && typeof window.matchMedia === "function" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
  media?.addEventListener?.("change", () => {
    if (state.mode === "system") apply();
  });
  apply();
  if (!stored || stored === DEFAULT_SKIN_ID) return;
  const pending = setSkin(stored).then(
    () => undefined,
    () => undefined,
  );
  await Promise.race([pending, new Promise((resolve) => setTimeout(resolve, BOOT_WAIT_MS))]);
}

export function setMode(mode: ModeSetting) {
  state = { ...state, mode };
  writeLocal(MODE_KEY, mode);
  apply();
}

/** After an uninstall: drop the cached entry and leave the skin if it was active. */
export function forgetSkin(id: string) {
  const { [id]: _removed, ...installed } = state.installed;
  state = { ...state, installed };
  if (state.skinId === id || state.active.manifest.id === id) {
    removeLocal(SKIN_KEY);
    state = { ...state, skinId: DEFAULT_SKIN_ID, active: BUILTIN_SKIN, status: "ready", failure: null };
    apply();
    return;
  }
  emit();
}

/** Tests replace the network loader. */
export function setSkinLoader(next: typeof fetchInstalledSkin) {
  loader = next;
}

export function resetSkinStore() {
  state = { skinId: DEFAULT_SKIN_ID, mode: "system", resolved: "light", active: BUILTIN_SKIN, status: "ready", failure: null, installed: {} };
  loader = fetchInstalledSkin;
  apply();
}

export function getSkinState(): SkinState {
  return state;
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

const getState = () => state;

export function useSkinState(): SkinState {
  return useSyncExternalStore(subscribe, getState, getState);
}

export function useSkin() {
  const snapshot = useSkinState();
  return { ...snapshot, entry: snapshot.active, setMode, setSkin, loadSkin, forgetSkin };
}
