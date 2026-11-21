/**
 * Main navigation, built from the module registry. Its form is two slots:
 *   nav      — `sidebar` (labelled, grouped side list) or `topbar` (one row of tabs in the top bar)
 *   navPhone — `bottom-bar` (tab bar at the bottom) or `top-scroll` (a scrollable row under the top bar)
 */
import { Drawer } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { MoreHorizontal, PanelLeftClose } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useAccess } from "@/modules/access";
import { isEnabled } from "@/modules/registry";
import type { ModuleDefinition } from "@/modules/types";
import { CountMark } from "@/ui/count-mark";
import { useActiveModuleId, useVisibleModules } from "./use-shell-modules";
import { UserMenu } from "./user-menu";

const useNoBadge = () => 0;

function Badge({ module, className }: { module: ModuleDefinition; className?: string }) {
  const useCount = module.badge ?? useNoBadge;
  const count = useCount();
  return <CountMark count={count} className={className} />;
}

export function Wordmark({ className }: { className?: string }) {
  const { t } = useTranslation();
  return (
    <Link to="/" className={cn("shrink-0 rounded-sm font-heading text-[1.1875rem] leading-none font-title tracking-[-0.015em] outline-none focus-visible:ring-2 focus-visible:ring-focus", className)}>
      {t("app.name")}
    </Link>
  );
}

/* ---------------------------------------------------------------- topbar tabs */

/** One row of tabs inside the top bar. Pill or underline comes from the nav tokens. */
export function TopNav({ className }: { className?: string }) {
  const { t } = useTranslation();
  const modules = useVisibleModules();
  const activeId = useActiveModuleId();
  const access = useAccess();
  return (
    <nav aria-label={t("a11y.navigation")} className={cn("min-w-0", className)}>
      <ul className="flex items-center gap-0.5 rounded-control bg-nav-track p-[3px]">
        {modules.map((module) => {
          const active = module.id === activeId;
          return (
            <li key={module.id}>
              <Link
                to={module.home}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "relative flex h-(--pv-control-h) items-center gap-1.5 rounded-control px-3.5 text-sm whitespace-nowrap outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus",
                  active ? "bg-nav-active font-strong text-nav-active-fg" : "font-mid text-shell-fg-muted hover:text-shell-fg",
                  !isEnabled(module, access) && !active && "opacity-70",
                )}
              >
                {t(module.titleKey)}
                <Badge module={module} />
                {active ? (
                  <span
                    aria-hidden
                    className="absolute inset-x-3 h-0.5 bg-nav-indicator"
                    style={{ bottom: "calc((var(--pv-control-h) + 6px - var(--pv-shell-h)) / 2)" }}
                  />
                ) : null}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}

/* ---------------------------------------------------------------- sidebar */

function SidebarItem({ module, active }: { module: ModuleDefinition; active: boolean }) {
  const { t } = useTranslation();
  const access = useAccess();
  const Icon = module.icon;
  return (
    <Link
      to={module.home}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex h-[1.875rem] items-center gap-2.5 rounded-tile px-2.5 text-base outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus",
        active ? "bg-nav-active font-mid text-nav-active-fg" : "text-shell-fg hover:bg-hover",
        !isEnabled(module, access) && !active && "opacity-70",
      )}
    >
      <Icon className={cn("size-4 shrink-0", active ? "text-nav-active-fg" : "text-active-mark")} aria-hidden />
      <span className="min-w-0 flex-1 truncate">{t(module.titleKey)}</span>
      <Badge module={module} />
    </Link>
  );
}

export function Sidebar({ onCollapse }: { onCollapse?: () => void }) {
  const { t } = useTranslation();
  const modules = useVisibleModules();
  const activeId = useActiveModuleId();
  const access = useAccess();
  const main = modules.filter((module) => module.group !== "manage");
  const manage = modules.filter((module) => module.group === "manage");
  const sections = modules.filter((module) => module.sidebar && isEnabled(module, access));
  return (
    <aside className="flex h-full w-(--pv-sidebar-w) shrink-0 flex-col border-e border-shell-line bg-shell text-shell-fg">
      <div className="flex h-(--pv-shell-h) shrink-0 items-center justify-between px-5">
        <Wordmark className="text-heading" />
        {onCollapse ? (
          <button type="button" onClick={onCollapse} aria-label={t("a11y.toggleSidebar")} className="grid size-7 place-items-center rounded-tile text-shell-fg-muted hover:bg-hover hover:text-shell-fg">
            <PanelLeftClose className="size-4" aria-hidden />
          </button>
        ) : null}
      </div>
      <nav aria-label={t("a11y.navigation")} className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-2.5 pb-4">
        <div className="grid gap-0.5">
          {main.map((module) => (
            <SidebarItem key={module.id} module={module} active={module.id === activeId} />
          ))}
        </div>
        {sections.map((module) => {
          const Section = module.sidebar?.component;
          if (!Section || !module.sidebar) return null;
          return (
            <div key={module.id} className="grid gap-0.5">
              <h2 className="px-2.5 pb-1 text-xs font-mid text-shell-fg-muted">{t(module.sidebar.titleKey)}</h2>
              <Section />
            </div>
          );
        })}
        {manage.length > 0 ? (
          <div className="grid gap-0.5">
            <h2 className="px-2.5 pb-1 text-xs font-mid text-shell-fg-muted">{t("nav.manage")}</h2>
            {manage.map((module) => (
              <SidebarItem key={module.id} module={module} active={module.id === activeId} />
            ))}
          </div>
        ) : null}
      </nav>
      <div className="border-t border-shell-line p-2.5">
        <UserMenu layout="row" />
      </div>
    </aside>
  );
}

/* ---------------------------------------------------------------- phone */

const PHONE_SLOTS = 4;

/**
 * Phone tab bar at the bottom: the first destinations and a sheet for the rest. The tabs have no
 * fill, so the current one is told by its mark colour and weight in the shell's own text colours.
 */
export function BottomTabBar() {
  const { t } = useTranslation();
  const modules = useVisibleModules();
  const activeId = useActiveModuleId();
  const [moreOpen, setMoreOpen] = useState(false);
  const primary = modules.slice(0, PHONE_SLOTS);
  const rest = modules.slice(PHONE_SLOTS);
  const restActive = rest.some((module) => module.id === activeId);
  return (
    <>
      <nav
        aria-label={t("a11y.navigation")}
        className="fixed inset-x-0 bottom-0 z-30 border-t border-shell-line bg-shell pb-[env(safe-area-inset-bottom)] md:hidden"
      >
        <div className="grid h-16 grid-cols-5">
          {primary.map((module) => {
            const active = module.id === activeId;
            const Icon = module.icon;
            return (
              <Link
                key={module.id}
                to={module.home}
                aria-current={active ? "page" : undefined}
                className={cn("flex flex-col items-center justify-center gap-0.5 text-[0.6875rem]", active ? "font-strong text-shell-fg" : "text-shell-fg-muted")}
              >
                <span className="relative">
                  <Icon className={cn("size-[1.35rem]", active && "text-active-mark")} strokeWidth={active ? 2.2 : 1.8} aria-hidden />
                  <Badge module={module} className="absolute -top-1.5 -right-3" />
                </span>
                {t(module.titleKey)}
              </Link>
            );
          })}
          <button
            type="button"
            onClick={() => setMoreOpen(true)}
            aria-current={restActive ? "page" : undefined}
            className={cn("flex flex-col items-center justify-center gap-0.5 text-[0.6875rem]", restActive ? "font-strong text-shell-fg" : "text-shell-fg-muted")}
          >
            <MoreHorizontal className={cn("size-[1.35rem]", restActive && "text-active-mark")} aria-hidden />
            {t("nav.more")}
          </button>
        </div>
      </nav>
      <Drawer.Backdrop isOpen={moreOpen} onOpenChange={setMoreOpen}>
        <Drawer.Content placement="bottom">
          <Drawer.Dialog className="pb-[calc(env(safe-area-inset-bottom)+1rem)]">
            <Drawer.Handle />
            <Drawer.Header>
              <Drawer.Heading>{t("nav.more")}</Drawer.Heading>
            </Drawer.Header>
            <Drawer.Body>
              <div className="grid grid-cols-3 gap-2">
                {rest.map((module) => {
                  const Icon = module.icon;
                  return (
                    <Link
                      key={module.id}
                      to={module.home}
                      onClick={() => setMoreOpen(false)}
                      className="flex flex-col items-center gap-2 rounded-tile bg-sunken px-2 py-4 text-sm"
                    >
                      <Icon className="size-5 text-active-mark" aria-hidden />
                      {t(module.titleKey)}
                    </Link>
                  );
                })}
              </div>
            </Drawer.Body>
          </Drawer.Dialog>
        </Drawer.Content>
      </Drawer.Backdrop>
    </>
  );
}

/** Phone: every destination in one horizontally scrollable row under the top bar. */
export function ScrollTabs() {
  const { t } = useTranslation();
  const modules = useVisibleModules();
  const activeId = useActiveModuleId();
  return (
    <nav aria-label={t("a11y.navigation")} className="no-scrollbar overflow-x-auto border-b border-shell-line bg-shell md:hidden">
      <ul className="flex w-max items-center gap-1 px-3 py-2">
        {modules.map((module) => {
          const active = module.id === activeId;
          return (
            <li key={module.id}>
              <Link
                to={module.home}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "relative flex h-8 items-center gap-1.5 rounded-control px-3 text-sm whitespace-nowrap",
                  active ? "bg-nav-active font-strong text-nav-active-fg" : "text-shell-fg-muted",
                )}
              >
                {t(module.titleKey)}
                <Badge module={module} />
                {active ? <span aria-hidden className="absolute inset-x-3 -bottom-2 h-0.5 bg-nav-indicator" /> : null}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
