/** localStorage helpers for per-viewer conveniences; failures are silent by design. */
export function readLocal(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writeLocal(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Not persisted.
  }
}

export function removeLocal(key: string) {
  try {
    window.localStorage.removeItem(key);
  } catch {
    // Nothing stored.
  }
}
