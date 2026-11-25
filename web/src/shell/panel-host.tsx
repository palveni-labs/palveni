import { Drawer, Spinner } from "@heroui/react";
import { Suspense, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useIsWide } from "@/lib/use-media";
import { useAccess } from "@/modules/access";
import { FeatureUnavailable } from "./feature-gate";
import { shell, useShellState } from "./shell-store";
import { panelModule, usePanelAvailable } from "./use-panel";

function PanelBody() {
  const access = useAccess();
  const panel = panelModule?.panel;
  if (!panel) return null;
  if (panel.feature && !access.features.includes(panel.feature)) return <FeatureUnavailable titleKey={panel.titleKey} compact />;
  const Component = panel.component;
  return (
    <Suspense fallback={<div className="grid flex-1 place-items-center"><Spinner size="sm" /></div>}>
      <Component onClose={() => shell.closeAssistant()} />
    </Suspense>
  );
}

/** ⌘J side panel (contributed by a module): docked beside the page on wide screens, a sheet elsewhere. */
export function PanelHost() {
  const { t } = useTranslation();
  const { panelOpen } = useShellState();
  const wide = useIsWide();
  const available = usePanelAvailable();

  useEffect(() => {
    if (!available) return;
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "j") {
        event.preventDefault();
        shell.toggleAssistant();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [available]);

  if (!available || !panelModule?.panel) return null;
  const title = t(panelModule.panel.titleKey);

  if (wide) {
    if (!panelOpen) return null;
    return (
      <aside aria-label={title} className="relative z-20 flex w-[26rem] shrink-0 flex-col border-s border-line bg-surface">
        <PanelBody />
      </aside>
    );
  }
  return (
    <Drawer.Backdrop isOpen={panelOpen} onOpenChange={(open) => (open ? undefined : shell.closeAssistant())}>
      <Drawer.Content placement="right">
        <Drawer.Dialog aria-label={title} className="flex h-full w-[min(28rem,94vw)] max-w-none flex-col gap-0 overflow-hidden border-s border-line p-0">
          <PanelBody />
        </Drawer.Dialog>
      </Drawer.Content>
    </Drawer.Backdrop>
  );
}
