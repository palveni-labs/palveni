/// <reference types="node" />
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import cinema from "./builtin/cinema";
import sonic from "./builtin/sonic";
import studio from "./builtin/studio";
import workcolor from "./builtin/workcolor";
import { contrast, isPurple } from "./color";
import { BUILTIN_SKIN, BUILTIN_SKINS, DEFAULT_SKIN_ID } from "./registry";
import { COLOR_TOKENS, parseSkin, SLOT_NAMES, type SkinManifest } from "./schema";
import { checkSkin, describeIssue } from "./validate";
import { secureSurfaceProps } from "./secure";

const PLUGINS = path.resolve(process.cwd(), "../plugins");
const packages = readdirSync(PLUGINS)
  .filter((name) => name.startsWith("skin-"))
  .map((name) => ({ name, manifest: JSON.parse(readFileSync(path.join(PLUGINS, name, "skin.json"), "utf8")) as unknown, dir: path.join(PLUGINS, name) }));

/** Slot contract per built-in and official plugin skin. */
const SLOT_TABLE: Record<string, Partial<SkinManifest["slots"]>> = {
  studio: { nav: "sidebar", navPhone: "bottom-bar", status: "inline", progress: "bar", attention: "tint", section: "panel", itemColor: "on", measure: "mono", pageTitle: "content", summary: "cells" },
  sonic: { nav: "sidebar", navPhone: "bottom-bar", status: "inline", progress: "row-fill", attention: "thumbs", section: "panel", itemColor: "on", measure: "sans", pageTitle: "content", summary: "inline" },
  cinema: { nav: "sidebar", navPhone: "bottom-bar", status: "inline", progress: "bar", attention: "thumbs", section: "panel", itemColor: "on", measure: "sans", pageTitle: "content", summary: "inline" },
  workcolor: { nav: "sidebar", navPhone: "bottom-bar", status: "inline", progress: "row-fill", attention: "thumbs", section: "panel", itemColor: "on", measure: "sans", pageTitle: "content", summary: "inline" },
  native: { nav: "sidebar", navPhone: "bottom-bar", status: "activity-window", progress: "bar", attention: "tint", section: "plain", itemColor: "off", measure: "sans", pageTitle: "toolbar", summary: "cells" },
  ink: { nav: "topbar", navPhone: "bottom-bar", status: "inline", progress: "bar", attention: "seal", section: "rule", itemColor: "off", measure: "sans", pageTitle: "content", summary: "inline" },
  gear: { nav: "topbar", navPhone: "top-scroll", status: "inline", progress: "separator", attention: "lamp", section: "rule", itemColor: "off", measure: "mono", pageTitle: "content", summary: "cells" },
};

describe("the built-in skins", () => {
  it("default to studio and pass every validation rule", () => {
    expect(DEFAULT_SKIN_ID).toBe("studio");
    expect(BUILTIN_SKIN.manifest.id).toBe("studio");
    expect(BUILTIN_SKINS.map((entry) => entry.manifest.id)).toEqual(["studio", "sonic", "cinema", "workcolor"]);
    for (const raw of [studio, sonic, cinema, workcolor]) {
      expect(checkSkin(parseSkin(raw.manifest, "builtin")).map(describeIssue)).toEqual([]);
    }
  });

  it("ship all built-in fonts from this origin", async () => {
    for (const entry of BUILTIN_SKINS) {
      const fonts = await entry.loadFonts();
      expect(fonts.length).toBeGreaterThan(0);
      for (const font of fonts) expect(font.src).toMatch(/^\/(?!\/)/);
    }
  });
});

describe("the official skin plugins", () => {
  it("are the three packages in plugins/", () => {
    expect(packages.map((entry) => entry.name).sort()).toEqual(["skin-gear", "skin-ink", "skin-native"]);
  });

  it.each(packages.map((entry) => [entry.name, entry] as const))("%s parses and passes every rule", (_name, entry) => {
    const skin = parseSkin(entry.manifest, "package");
    expect(checkSkin(skin).map(describeIssue)).toEqual([]);
    for (const font of skin.fonts) {
      expect(font.src).toMatch(/^fonts\/[^/]+\.woff2$/);
      expect(readFileSync(path.join(entry.dir, font.src)).subarray(0, 4).toString("latin1")).toBe("wOF2");
    }
  });

  it("each keeps a licence file next to its fonts", () => {
    for (const entry of packages) {
      const files = readdirSync(path.join(entry.dir, "fonts"));
      expect(files.some((file) => file.startsWith("LICENSE"))).toBe(true);
    }
  });
});

describe("all built-in and official skins", () => {
  const all = [...BUILTIN_SKINS.map((entry) => entry.manifest), ...packages.map((entry) => parseSkin(entry.manifest))];

  it.each(all)("keeps $id styled in both modes, including readable button states and focus rings", (skin) => {
    for (const mode of ["light", "dark"] as const) {
      const entry = { ...BUILTIN_SKIN, source: "installed" as const, packageSource: "official" as const, manifest: skin };
      const props = secureSurfaceProps(entry, mode);
      expect(props["data-secure-surface"]).toBe("theme");
      const style = props.style as Record<string, string>;
      const colors = skin.colors[mode];
      for (const state of ["hover", "pressed"]) {
        expect(contrast(colors["confirm-fg"], style[`--pv-secure-confirm-${state}`] ?? "")).toBeGreaterThanOrEqual(4.5);
        expect(contrast(colors["fg-inverse"], style[`--pv-secure-danger-${state}`] ?? "")).toBeGreaterThanOrEqual(4.5);
      }
      for (const ground of ["bg", "surface", "surface-raised", "surface-sunken", "attention-soft"] as const) {
        expect(contrast(style["--pv-focus"] ?? colors.focus, colors[ground])).toBeGreaterThanOrEqual(3);
        expect(contrast(style["--pv-line-control"] ?? colors["line-control"], colors[ground])).toBeGreaterThanOrEqual(3);
      }
    }
  });

  it("match the slot table in the skin contract", () => {
    for (const skin of all) {
      expect(Object.keys(skin.slots).sort()).toEqual([...SLOT_NAMES].sort());
      expect(skin.slots).toEqual(SLOT_TABLE[skin.id]);
    }
  });

  it("use no purple in any colour, light or dark", () => {
    for (const skin of all) {
      for (const mode of ["light", "dark"] as const) {
        for (const token of COLOR_TOKENS) expect(isPurple(skin.colors[mode][token]), `${skin.id} ${mode} ${token}`).toBe(false);
      }
    }
  });
});
