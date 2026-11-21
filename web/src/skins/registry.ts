/**
 * Skin entries: the one built-in skin (`workcolor`, compiled into the app with its fonts) and
 * skins installed on the server as plugins, fetched at runtime (see loader.ts).
 *
 * Every manifest — built-in or installed — goes through the same parser and rules.
 */
import workcolor from "./builtin/workcolor";
import { parseFontAsset, parseSkin, SkinError, type FontAsset, type FontSource, type SkinManifest } from "./schema";
import { checkSkin, describeIssue } from "./validate";

export interface SkinEntry {
  manifest: SkinManifest;
  source: "builtin" | "installed";
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

export const DEFAULT_SKIN_ID = "workcolor";

/** The only skin in the app bundle. */
export const BUILTIN_SKIN: SkinEntry = builtin(workcolor);
