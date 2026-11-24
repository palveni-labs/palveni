import { useAccess } from "@/modules/access";
import { isEnabled, MODULES } from "@/modules/registry";

/** The module that contributes the ⌘J side panel (at most one). */
export const panelModule = MODULES.find((module) => module.panel);

/** The panel's module and the panel itself are visible to this user and enabled on the server. */
export function usePanelAvailable(): boolean {
  const access = useAccess();
  return Boolean(panelModule?.panel && isEnabled(panelModule, access) && isEnabled(panelModule.panel, access));
}
