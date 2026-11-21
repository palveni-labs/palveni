import { Outlet, useRouterState } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { ScrollContainerContext } from "@/lib/scroll-container";
import { readLocal, writeLocal } from "@/lib/storage";
import { useSlot } from "@/skins/scope";
import { CommandPalette } from "./command-palette";
import { FeatureGate } from "./feature-gate";
import { InboxCollectors } from "./inbox-store";
import { ModuleTabs } from "./module-tabs";
import { BottomTabBar, Sidebar } from "./nav";
import { useShellEvents } from "./server-events";
import { Toolbar, TopBar } from "./topbar";

const SIDEBAR_KEY = "palveni.sidebar";

/**
 * The authenticated frame. Its arrangement comes from the skin's `nav` and `navPhone` slots;
 * everything module-specific comes from the registry.
 */
export function AppShell() {
  useShellEvents();
  const nav = useSlot("nav");
  const navPhone = useSlot("navPhone");
  const scrollRef = useRef<HTMLDivElement>(null);
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const searchStr = useRouterState({ select: (state) => state.location.searchStr });
  const [sidebarOpen, setSidebarOpen] = useState(() => readLocal(SIDEBAR_KEY) !== "closed");

  // The workspace is the scroll container: start every page (and folder) at the top.
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [pathname, searchStr]);

  const toggleSidebar = (open: boolean) => {
    setSidebarOpen(open);
    writeLocal(SIDEBAR_KEY, open ? "open" : "closed");
  };

  return (
    <ScrollContainerContext value={scrollRef}>
      <InboxCollectors />
      <div className="flex h-svh overflow-hidden bg-page text-fg">
        {nav === "sidebar" && sidebarOpen ? (
          <div className="max-md:hidden">
            <Sidebar onCollapse={() => toggleSidebar(false)} />
          </div>
        ) : null}
        <div className="flex min-w-0 flex-1 flex-col">
          {nav === "sidebar" ? <Toolbar sidebarOpen={sidebarOpen} onOpenSidebar={() => toggleSidebar(true)} /> : <TopBar />}
          <main
            ref={scrollRef}
            id="workspace-content"
            className={cn("min-h-0 flex-1 overflow-y-auto overscroll-contain", navPhone === "bottom-bar" && "max-md:pb-[calc(4rem+env(safe-area-inset-bottom))]")}
          >
            <div key={pathname} className="workspace-page flex min-h-full flex-col">
              <ModuleTabs />
              <FeatureGate>
                <Outlet />
              </FeatureGate>
            </div>
          </main>
        </div>
        {navPhone === "bottom-bar" ? <BottomTabBar /> : null}
        <CommandPalette />
      </div>
    </ScrollContainerContext>
  );
}
