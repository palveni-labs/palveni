/**
 * Skin entries: the built-in modern skins (`studio`, `sonic`, `cinema`, `workcolor`, compiled into
 * the app with their bundled fonts) and skins installed on the server as plugins, fetched at
 * runtime (see loader.ts).
 *
 * Every manifest — built-in or installed — goes through the same parser and rules.
 */
import cinema from "./builtin/cinema";
import sonic from "./builtin/sonic";
import studio from "./builtin/studio";
import workcolor from "./builtin/workcolor";
import { parseFontAsset, parseSkin, SkinError, type FontAsset, type FontSource, type SkinManifest } from "./schema";
import { checkSkin, describeIssue } from "./validate";

export interface SkinEntry {
  manifest: SkinManifest;
  source: "builtin" | "installed";
  /** Installation provenance supplied by the host alongside the manifest. */
  packageSource?: "official" | "upload";
  /** Version of the installed plugin package (undefined for the built-in skin). */
  version?: string;
  /** Resolves the font assets to register while the skin is in use (absolute URLs). */
  loadFonts: () => Promise<FontAsset[]>;
}

export interface BuiltinSkin {
  manifest: unknown;
  fonts: () => Promise<FontAsset[]>;
}

/** Parses and checks a skin; throws SkinError listing every rule it breaks. */
export function acceptSkin(input: unknown, fontSource: FontSource = "package"): SkinManifest {
  const manifest = parseSkin(input, fontSource);
  const issues = checkSkin(manifest).filter((issue) => fontSource === "builtin" || issue.kind !== "purple");
  if (issues.length > 0) throw new SkinError(issues.map(describeIssue).join("\n"));
  return manifest;
}

function builtin({ manifest, fonts }: BuiltinSkin): SkinEntry {
  const parsed = acceptSkin(manifest, "builtin");
  return {
    manifest: parsed,
    source: "builtin",
    loadFonts: async () => (await fonts()).map((asset, index) => parseFontAsset(asset, `${parsed.id}.fonts[${index}]`, "builtin")),
  };
}

export const DEFAULT_SKIN_ID = "studio";

/** Built-in skins shipped in the app bundle (default first). */
export const BUILTIN_SKINS: readonly SkinEntry[] = [
  builtin(studio),
  builtin(sonic),
  builtin(cinema),
  builtin(workcolor),
];

const BUILTIN_BY_ID = new Map<string, SkinEntry>(BUILTIN_SKINS.map((entry) => [entry.manifest.id, entry]));

export function findBuiltinSkin(id: string): SkinEntry | undefined {
  return BUILTIN_BY_ID.get(id);
}

/** The default built-in skin (`studio`). */
export const BUILTIN_SKIN: SkinEntry = BUILTIN_SKINS[0]!;
