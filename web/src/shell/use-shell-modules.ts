import { useRouterState } from "@tanstack/react-router";
import { useAccess } from "@/modules/access";
import { isVisible, MODULES, moduleForPath } from "@/modules/registry";
import type { ModuleDefinition, NavEntry } from "@/modules/types";

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

/** Modules the current user may see (feature-gated ones included, shown as unavailable). */
export function useVisibleModules(): ModuleDefinition[] {
  const access = useAccess();
  return MODULES.filter((module) => isVisible(module, access));
}
