/**
 * Access token storage. The token lives in localStorage so it survives reloads and is shared
 * between tabs; a logout in one tab is picked up by the others through the `storage` event.
 */

const STORAGE_KEY = "palveni.session";

export interface StoredSession {
  accessToken: string;
  expiresAt: string | null;
}

type Listener = () => void;

const listeners = new Set<Listener>();
let cached: StoredSession | null | undefined;

function readStorage(): StoredSession | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<StoredSession>;
    if (typeof parsed.accessToken !== "string" || parsed.accessToken === "") return null;
    return { accessToken: parsed.accessToken, expiresAt: parsed.expiresAt ?? null };
  } catch {
    return null;
  }
}

function writeStorage(session: StoredSession | null) {
  try {
    if (session) window.localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
    else window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Storage unavailable (private mode, blocked site data): the session stays in memory.
  }
}

function emit() {
  for (const listener of listeners) listener();
}

function isExpired(session: StoredSession, now = Date.now()): boolean {
  if (!session.expiresAt) return false;
  const expires = Date.parse(session.expiresAt);
  return Number.isFinite(expires) && expires <= now;
}

export function getSession(): StoredSession | null {
  if (cached === undefined) cached = readStorage();
  if (cached && isExpired(cached)) {
    cached = null;
    writeStorage(null);
  }
  return cached;
}

export function getToken(): string | null {
  return getSession()?.accessToken ?? null;
}

export function setSession(session: StoredSession) {
  cached = session;
  writeStorage(session);
  emit();
}

export function clearSession() {
  if (getSession() === null) return;
  cached = null;
  writeStorage(null);
  emit();
}

export function subscribeSession(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key !== null && event.key !== STORAGE_KEY) return;
    cached = readStorage();
    emit();
  });
}

/** Test helper: forget the in-memory copy so the next read goes to storage. */
export function resetSessionCache() {
  cached = undefined;
}
