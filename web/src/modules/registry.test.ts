import { describe, expect, it } from "vitest";
import i18n from "@/i18n";
import { allWidgets, basePathOf, collectModules, isEnabled, isVisible, MODULES, moduleForPath } from "./registry";
import type { ModuleDefinition } from "./types";

function hasKey(resource: unknown, key: string): boolean {
  let current: unknown = resource;
  const parts = key.split(".");
  for (const [index, part] of parts.entries()) {
    if (typeof current !== "object" || current === null) return false;
    const record = current as Record<string, unknown>;
    if (index === parts.length - 1 && (part in record || `${part}_other` in record)) return true;
    current = record[part];
  }
  return false;
}

const routeFiles = import.meta.glob("./*/routes/*.tsx");

describe("module registry", () => {
  it("discovers every module folder, ordered and with unique ids", () => {
    const ids = MODULES.map((module) => module.id);
    expect(ids).toEqual(["overview", "music", "video", "library", "broadcast", "tasks", "apps", "system"]);
    expect(new Set(ids).size).toBe(ids.length);
    const withRoutes = new Set(Object.keys(routeFiles).map((file) => file.split("/")[1]));
    expect([...withRoutes].sort()).toEqual([...ids].sort());
  });

  it("rejects a module whose id does not match its folder, and duplicates", () => {
    const definition = { id: "tasks", order: 1, titleKey: "nav.jobs", icon: () => null, home: "/" } as unknown as ModuleDefinition;
    expect(() => collectModules({ "./other/module.tsx": { default: definition } })).toThrow(/declares id/);
    expect(() =>
      collectModules({ "./tasks/module.tsx": { default: definition }, "./tasks//module.tsx": { default: definition } }),
    ).toThrow();
  });

  it("only uses message keys that exist in both locales", () => {
    const zhBundle = i18n.getResourceBundle("zh-CN", "translation") as unknown;
    const enBundle = i18n.getResourceBundle("en", "translation") as unknown;
    const keys = MODULES.flatMap((module) => [
      module.titleKey,
      ...(module.nav ?? []).map((entry) => entry.titleKey),
      ...(module.widgets ?? []).map((widget) => widget.titleKey),
      ...(module.commands ?? []).map((command) => command.titleKey),
    ]);
    for (const key of keys) {
      expect(hasKey(zhBundle, key), `zh-CN is missing ${key}`).toBe(true);
      expect(hasKey(enBundle, key), `en is missing ${key}`).toBe(true);
    }
  });

  it("keeps navigation inside each module's own path and widget ids unique", () => {
    for (const module of MODULES) {
      const base = basePathOf(module);
      for (const path of [module.home, ...(module.nav ?? []).map((entry) => entry.to)]) {
        expect(base === "/" ? path === "/" : path === base || path.startsWith(`${base}/`), `${module.id}: ${path}`).toBe(true);
      }
    }
    const widgetIds = allWidgets().map((widget) => widget.id);
    expect(new Set(widgetIds).size).toBe(widgetIds.length);
  });

  it("maps paths to their owning module", () => {
    expect(moduleForPath("/")?.id).toBe("overview");
    expect(moduleForPath("/library/abc")?.id).toBe("library");
    expect(moduleForPath("/system/hardware")?.id).toBe("system");
    expect(moduleForPath("/nowhere")).toBeUndefined();
  });

  it("gates by role and by server feature", () => {
    const member = { features: ["libraries", "jobs"], user: { role: "user" as const } };
    const admin = { features: ["libraries", "jobs"], user: { role: "admin" as const } };
    expect(isVisible({ adminOnly: true }, member)).toBe(false);
    expect(isVisible({ adminOnly: true }, admin)).toBe(true);
    expect(isEnabled({ feature: "unknown-feature" }, admin)).toBe(false);
    expect(isEnabled({ feature: "jobs" }, member)).toBe(true);
    for (const id of ["broadcast", "music", "video"]) {
      const module = MODULES.find((entry) => entry.id === id)!;
      expect(module.feature).toBe(id);
      expect(isEnabled(module, member)).toBe(false);
    }
  });
});
