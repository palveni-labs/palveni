/// <reference types="node" />
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import workcolor from "./builtin/workcolor";
import { contrast, isPurple } from "./color";
import { BUILTIN_SKIN, DEFAULT_SKIN_ID } from "./registry";
import { COLOR_TOKENS, parseSkin, SLOT_NAMES, type SkinManifest } from "./schema";
import { checkSkin, describeIssue } from "./validate";
import { secureSurfaceProps } from "./secure";

const PLUGINS = path.resolve(process.cwd(), "../plugins");
const packages = readdirSync(PLUGINS)
  .filter((name) => name.startsWith("skin-"))
  .map((name) => ({ name, manifest: JSON.parse(readFileSync(path.join(PLUGINS, name, "skin.json"), "utf8")) as unknown, dir: path.join(PLUGINS, name) }));

const SLOT_TABLE: Record<string, Partial<SkinManifest["slots"]>> = {
  workcolor: { nav: "topbar", navPhone: "top-scroll", status: "inline", progress: "row-fill", attention: "thumbs", section: "panel", itemColor: "on", measure: "sans", pageTitle: "content", summary: "inline" },
  native: { nav: "sidebar", navPhone: "bottom-bar", status: "activity-window", progress: "bar", attention: "tint", section: "plain", itemColor: "off", measure: "sans", pageTitle: "toolbar", summary: "cells" },
  ink: { nav: "topbar", navPhone: "bottom-bar", status: "inline", progress: "bar", attention: "seal", section: "rule", itemColor: "off", measure: "sans", pageTitle: "content", summary: "inline" },
  gear: { nav: "topbar", navPhone: "top-scroll", status: "inline", progress: "separator", attention: "lamp", section: "rule", itemColor: "off", measure: "mono", pageTitle: "content", summary: "cells" },
};

describe("the built-in skin", () => {
  it("is work colour, the default, and passes every rule", () => {
    expect(DEFAULT_SKIN_ID).toBe("workcolor");
    expect(BUILTIN_SKIN.manifest.id).toBe("workcolor");
    expect(checkSkin(parseSkin(workcolor.manifest, "builtin")).map(describeIssue)).toEqual([]);
  });

  it("ships its fonts from this origin", async () => {
    const fonts = await BUILTIN_SKIN.loadFonts();
    expect(fonts.length).toBeGreaterThan(0);
    for (const font of fonts) expect(font.src).toMatch(/^\/(?!\/)/);
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

describe("all four skins", () => {
  const all = [BUILTIN_SKIN.manifest, ...packages.map((entry) => parseSkin(entry.manifest))];

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

  it("use the expected layout slots", () => {
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
