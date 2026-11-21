import { describe, expect, it } from "vitest";
import workcolor from "./builtin/workcolor";
import { contrast, isPurple, parseColor } from "./color";
import { skinVariables } from "./css";
import { acceptSkin, BUILTIN_SKIN, type SkinEntry } from "./registry";
import { parseSkin, SkinError, type SkinManifest } from "./schema";
import { secureSurfaceProps } from "./secure";
import { checkSkin } from "./validate";

/** A valid package manifest to break in one place at a time. */
function base(): Record<string, unknown> {
  const manifest = structuredClone(workcolor.manifest) as Record<string, unknown>;
  return { ...manifest, id: "test-skin", fonts: [{ family: "Figtree", src: "fonts/figtree.woff2", weight: "300 900", style: "normal" }] };
}

/** Sets (or, with `undefined`, deletes) the value at a dotted path of a manifest copy. */
function mutate(path: string, value: unknown): Record<string, unknown> {
  const manifest = base();
  const keys = path.split(".");
  let node = manifest;
  for (const key of keys.slice(0, -1)) {
    const next = node[key];
    if (Array.isArray(next)) node = next[0] as Record<string, unknown>;
    else node = next as Record<string, unknown>;
  }
  const last = keys[keys.length - 1] ?? "";
  if (value === undefined) delete node[last];
  else node[last] = value;
  return manifest;
}

describe("the skin grammar", () => {
  it("accepts a complete, valid manifest", () => {
    expect(() => acceptSkin(base())).not.toThrow();
  });

  const hostile: Array<[string, unknown]> = [
    ["an unknown top-level field", mutate("css", "body{display:none}")],
    ["an unknown colour token", mutate("colors.light.backdrop", "#000")],
    ["a missing colour token", mutate("colors.dark.fg", undefined)],
    ["url() in a colour", mutate("colors.light.bg", "url(https://evil.example/x.png)")],
    ["var() in a colour", mutate("colors.light.fg", "var(--anything)")],
    ["a declaration break in a colour", mutate("colors.light.fg", "#000; background: red")],
    ["CSS text in a shadow", mutate("elevation.light.raised", "0 0 0 #000; } body { display:none")],
    ["url() in a shadow", mutate("elevation.dark.overlay", "0 0 4px url(x)")],
    ["@import in a font family", mutate("type.ui", ["@import url(x)", "sans-serif"])],
    ["quotes in a font family", mutate("type.heading", ['A", "B', "serif"])],
    ["a family stack without a generic family", mutate("type.mono", ["Fragment Mono"])],
    ["a font from another host", mutate("fonts.src", "https://fonts.example.com/f.woff2")],
    ["a protocol-relative font", mutate("fonts.src", "//evil.example/f.woff2")],
    ["a font outside the package", mutate("fonts.src", "fonts/../../secret.woff2")],
    ["an SVG font", mutate("fonts.src", "fonts/face.svg")],
    ["a script as a font", mutate("fonts.src", "fonts/face.js")],
    ["an oversized font", mutate("fonts.bytes", 50 * 1024 * 1024)],
    ["an unknown font field", mutate("fonts.fontDisplay", "block")],
    ["a bad unicode-range", mutate("fonts.unicodeRange", "U+0000-00FF; src: url(x)")],
    ["a slot value outside its enumeration", mutate("slots.nav", "floating")],
    ["an unknown slot", mutate("slots.layout", "magazine")],
    ["markup in the name", mutate("name", { en: "<img src=x onerror=alert(1)>" })],
    ["an out-of-range shape value", mutate("shape.control-h", 400)],
    ["a non-numeric type size", mutate("type.size.base", "14px")],
    ["a wrong format version", mutate("format", 2)],
    ["a non-object manifest", "body { color: red }"],
  ];

  it.each(hostile)("refuses %s", (_label, manifest) => {
    expect(() => acceptSkin(manifest)).toThrow(SkinError);
  });

  it("enforces the official palette while permitting third-party chart colours", () => {
    expect(isPurple("oklch(0.6 0.2 300)")).toBe(true);
    expect(isPurple("#7c3aed")).toBe(true);
    expect(isPurple("oklch(0.5 0.12 250)")).toBe(false);
    expect(isPurple("oklch(0.5 0 300)")).toBe(false);
    const thirdParty = mutate("colors.dark.chart-3", "oklch(0.6 0.2 300)");
    expect(() => acceptSkin({ ...thirdParty, fonts: [] }, "builtin")).toThrow(/purple/);
    expect(acceptSkin(thirdParty).colors.dark["chart-3"]).toBe("oklch(0.6 0.2 300)");
  });

  it("refuses text that fails the contrast floor", () => {
    expect(() => acceptSkin(mutate("colors.light.fg-muted", "oklch(0.8 0 0)"))).toThrow(/fg-muted on bg/);
    const dark = parseSkin(base()).colors.dark;
    expect(() => acceptSkin(mutate("colors.dark.confirm-fg", dark.confirm))).toThrow(/confirm-fg on confirm/);
  });

  it("measures contrast the WCAG way", () => {
    expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 1);
    expect(contrast("#777777", "#ffffff")).toBeCloseTo(4.48, 1);
    const white = parseColor("oklch(1 0 0)");
    expect(white?.r).toBeCloseTo(1, 5);
    expect(white?.b).toBeCloseTo(1, 5);
  });

  it("only ever emits validated values as custom properties", () => {
    const skin = parseSkin(base());
    for (const [name, value] of Object.entries(skinVariables(skin, "light"))) {
      expect(name).toMatch(/^(--[a-z0-9-]+|color-scheme)$/);
      expect(value).not.toMatch(/url\(|@import|[;{}<>]/);
    }
  });
});

describe("decision surfaces", () => {
  it("keeps package fonts and size tokens outside protected content, including on older servers", () => {
    const manifest = parseSkin(mutate("type.ui", ["Sample Font", "serif"]));
    const entry: SkinEntry = { manifest, source: "installed", loadFonts: () => Promise.resolve([]) };
    for (const mode of ["light", "dark"] as const) {
      for (const packageSource of [undefined, "upload"] as const) {
        const surface = secureSurfaceProps({ ...entry, packageSource }, mode);
        expect(surface["data-secure-surface"]).toBe("host");
        expect(surface.style).toMatchObject({
          "--pv-bg": BUILTIN_SKIN.manifest.colors[mode].bg,
          "--pv-font-ui": "system-ui, sans-serif",
          "--pv-text-base": skinVariables(BUILTIN_SKIN.manifest, mode)["--pv-text-base"],
          "--field-placeholder": "var(--pv-fg-subtle)",
        });
      }
    }
  });

  it("uses host colours if a verified theme has an unreadable decision state", () => {
    const skin: SkinManifest = parseSkin(base());
    const broken: SkinManifest = { ...skin, colors: { ...skin.colors, light: { ...skin.colors.light, "action-hover": skin.colors.light["action-fg"] } } };
    expect(checkSkin(broken)).toEqual([]);
    const entry: SkinEntry = { source: "installed", packageSource: "official", manifest: broken, loadFonts: () => Promise.resolve([]) };
    expect(secureSurfaceProps(entry, "dark")["data-secure-surface"]).toBe("theme");
    const surface = secureSurfaceProps(entry, "light");
    expect(surface["data-secure-surface"]).toBe("host");
    expect(surface.style).toMatchObject({ "--pv-action-hover": BUILTIN_SKIN.manifest.colors.light["action-hover"] });
  });
});
