import type { FontAsset } from "./schema";

interface Entry {
  users: number;
  faces: FontFace[];
}

const loaded = new Map<string, Entry>();

function canUseFontApi(): boolean {
  return typeof document !== "undefined" && "fonts" in document && typeof FontFace === "function";
}

function createFace(asset: FontAsset): FontFace | null {
  try {
    return new FontFace(asset.family, `url("${asset.src}") format("woff2")`, {
      weight: asset.weight,
      style: asset.style,
      display: "swap",
      ...(asset.unicodeRange ? { unicodeRange: asset.unicodeRange } : {}),
      ...(asset.sizeAdjust !== undefined ? { sizeAdjust: `${asset.sizeAdjust}%` } : {}),
      ...(asset.ascentOverride !== undefined ? { ascentOverride: `${asset.ascentOverride}%` } : {}),
      ...(asset.descentOverride !== undefined ? { descentOverride: `${asset.descentOverride}%` } : {}),
    });
  } catch {
    return null;
  }
}

/** Registers a skin's faces (reference counted per key); returns the release function. */
export function retainFonts(key: string, assets: readonly FontAsset[]): () => void {
  if (!canUseFontApi()) return () => {};
  let entry = loaded.get(key);
  if (!entry) {
    const faces = assets.map(createFace).filter((face): face is FontFace => face !== null);
    for (const face of faces) document.fonts.add(face);
    entry = { users: 0, faces };
    loaded.set(key, entry);
  }
  entry.users += 1;
  let released = false;
  return () => {
    if (released) return;
    released = true;
    const current = loaded.get(key);
    if (!current) return;
    current.users -= 1;
    if (current.users > 0) return;
    for (const face of current.faces) document.fonts.delete(face);
    loaded.delete(key);
  };
}

/** Keys whose faces are currently registered (for tests and diagnostics). */
export function registeredFontKeys(): string[] {
  return [...loaded.keys()];
}
