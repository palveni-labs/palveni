/**
 * Loads a skin installed on the server as a plugin package (feature "skins").
 *
 * A package's manifest and assets are public, read-only URLs: the browser requests font files
 * from CSS/FontFace without an Authorization header, and the active skin must also apply on the
 * login page. They carry only presentation data.
 *
 *   GET /api/v1/skins/installed/{id}/skin.json      the manifest
 *   GET /api/v1/skins/installed/{id}/assets/{path}  fonts/*.woff2, preview.{png,webp,avif,jpg}
 *
 * A manifest that fails to parse or breaks a rule is refused as a whole; nothing of it applies.
 */
import { API_BASE } from "@/api/client";
import { acceptSkin, type SkinEntry } from "./registry";
import { SkinError } from "./schema";

export const SKINS_BASE = `${API_BASE}/skins`;
/** Largest manifest the client will read. */
export const MAX_MANIFEST_BYTES = 512 * 1024;

const SKIN_ID = /^[a-z0-9][a-z0-9.-]{1,62}$/;
const ASSET_PATH = /^(?:fonts\/[A-Za-z0-9][A-Za-z0-9._-]{0,120}\.woff2|preview\.(?:png|webp|avif|jpg))$/;

export function installedManifestUrl(id: string): string {
  return `${SKINS_BASE}/installed/${encodeURIComponent(id)}/skin.json`;
}

export function installedAssetUrl(id: string, path: string): string {
  if (!ASSET_PATH.test(path) || path.includes("..")) throw new SkinError(`"${path}" is not an asset a skin may ship`);
  return `${SKINS_BASE}/installed/${encodeURIComponent(id)}/assets/${path}`;
}

export type SkinLoadFailure = "missing" | "unreachable" | "invalid";

export class SkinLoadError extends Error {
  override readonly name = "SkinLoadError";
  readonly reason: SkinLoadFailure;
  constructor(reason: SkinLoadFailure, message: string) {
    super(message);
    this.reason = reason;
  }
}

export interface LoadOptions {
  signal?: AbortSignal;
  fetchImpl?: typeof fetch;
}

/** Fetches, parses and checks an installed skin. Throws SkinLoadError with the reason. */
export async function fetchInstalledSkin(id: string, options: LoadOptions = {}): Promise<SkinEntry> {
  if (!SKIN_ID.test(id)) throw new SkinLoadError("invalid", `"${id}" is not a skin id`);
  const fetchImpl = options.fetchImpl ?? fetch;
  let response: Response;
  try {
    response = await fetchImpl(installedManifestUrl(id), {
      headers: { Accept: "application/json" },
      credentials: "omit",
      signal: options.signal,
    });
  } catch (error) {
    throw new SkinLoadError("unreachable", error instanceof Error ? error.message : String(error));
  }
  if (response.status === 404) throw new SkinLoadError("missing", `Skin "${id}" is not installed`);
  if (!response.ok) throw new SkinLoadError("unreachable", `The server answered ${response.status}`);
  const declared = Number(response.headers.get("Content-Length") ?? "0");
  if (declared > MAX_MANIFEST_BYTES) throw new SkinLoadError("invalid", "The skin manifest is too large");
  const text = await response.text();
  if (text.length > MAX_MANIFEST_BYTES) throw new SkinLoadError("invalid", "The skin manifest is too large");
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    throw new SkinLoadError("invalid", "The skin manifest is not valid JSON");
  }
  let manifest;
  try {
    manifest = acceptSkin(json, "package");
  } catch (error) {
    throw new SkinLoadError("invalid", error instanceof Error ? error.message : String(error));
  }
  if (manifest.id !== id) throw new SkinLoadError("invalid", `The package declares skin "${manifest.id}", not "${id}"`);
  const fonts = manifest.fonts.map((font) => ({ ...font, src: installedAssetUrl(id, font.src) }));
  return { manifest, source: "installed", loadFonts: () => Promise.resolve(fonts) };
}
