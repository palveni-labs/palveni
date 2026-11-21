/**
 * Module registry. Every `src/modules/<id>/module.tsx` is picked up automatically (and its
 * `routes/` folder is mounted by src/routes.ts), so adding a module never edits the shell.
 */
import type { AccessContext, Availability, ModuleDefinition, WidgetDefinition } from "./types";

const found = import.meta.glob<{ default: ModuleDefinition }>("./*/module.tsx", { eager: true });

export function collectModules(entries: Record<string, { default: ModuleDefinition }>): ModuleDefinition[] {
  const modules = Object.entries(entries).map(([file, exported]) => {
    const folder = /^\.\/([^/]+)\/module\.tsx$/.exec(file)?.[1];
    const definition = exported.default;
    if (folder !== definition.id) throw new Error(`Module in ${file} declares id "${definition.id}"`);
    return definition;
  });
  const ids = new Set<string>();
  for (const module of modules) {
    if (ids.has(module.id)) throw new Error(`Duplicate module id "${module.id}"`);
    ids.add(module.id);
  }
  return modules.sort((a, b) => a.order - b.order);
}

export const MODULES: readonly ModuleDefinition[] = collectModules(found);

/** The module that owns "/" — every other module lives under "/<id>". */
export const HOME_MODULE_ID = "overview";

export function basePathOf(module: Pick<ModuleDefinition, "id">): string {
  return module.id === HOME_MODULE_ID ? "/" : `/${module.id}`;
}

/** The module owning a pathname (longest prefix wins; "/" falls back to the overview). */
export function moduleForPath(pathname: string, modules: readonly ModuleDefinition[] = MODULES): ModuleDefinition | undefined {
  let match: ModuleDefinition | undefined;
  for (const module of modules) {
    const base = basePathOf(module);
    if (base === "/") {
      match ??= pathname === "/" ? module : match;
      continue;
    }
    if (pathname === base || pathname.startsWith(`${base}/`)) match = module;
  }
  return match;
}

/** Hidden entirely (wrong role). Feature-gated items stay visible but show as unavailable. */
export function isVisible(item: Availability, access: AccessContext): boolean {
  return !item.adminOnly || access.user?.role === "admin";
}

export function isEnabled(item: Availability, access: AccessContext): boolean {
  return isVisible(item, access) && (!item.feature || access.features.includes(item.feature));
}

export function allWidgets(modules: readonly ModuleDefinition[] = MODULES): Array<WidgetDefinition & { moduleId: string }> {
  return modules.flatMap((module) =>
    (module.widgets ?? []).map((widget) => ({ ...widget, moduleId: module.id, feature: widget.feature ?? module.feature })),
  );
}
