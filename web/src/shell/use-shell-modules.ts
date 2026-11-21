import { useRouterState } from "@tanstack/react-router";
import { useAccess } from "@/modules/access";
import { isVisible, MODULES, moduleForPath } from "@/modules/registry";
import type { ModuleDefinition, NavEntry } from "@/modules/types";
import { applyNavPrefs, orderModules, useNavPrefs } from "./nav-prefs";

export function useActiveModuleId(): string | undefined {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  return moduleForPath(pathname)?.id;
}

export function useActiveModule(): { module: ModuleDefinition | undefined; entry: NavEntry | undefined; pathname: string } {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const module = moduleForPath(pathname);
  const entry = module?.nav?.find((candidate) => pathname === candidate.to || pathname.startsWith(`${candidate.to}/`));
  return { module, entry, pathname };
}

/** All modules the current user may access, ordered by user preference (including hidden ones). */
export function useAccessibleModules(): ModuleDefinition[] {
  const access = useAccess();
  const prefs = useNavPrefs();
  return orderModules(MODULES.filter((module) => isVisible(module, access)), prefs);
}

/** Modules shown in primary navigation (ordered and filtered by user customization). */
export function useVisibleModules(): ModuleDefinition[] {
  const access = useAccess();
  const prefs = useNavPrefs();
  return applyNavPrefs(MODULES.filter((module) => isVisible(module, access)), prefs);
}
