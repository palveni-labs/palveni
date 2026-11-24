/**
 * Main navigation, built from the module registry. Its form is two slots:
 *   nav      — `sidebar` (labelled, grouped side list with collapsible secondary menus)
 *              or `topbar` (single-row navigation with floating dropdown secondary menus)
 *   navPhone — `bottom-bar` (tab bar at the bottom) or `top-scroll` (a scrollable row under the top bar)
 */
import { Drawer, Dropdown, Header, Label } from "@heroui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Check, ChevronDown, MoreHorizontal, PanelLeftClose, PanelRightClose } from "lucide-react";
import { Suspense, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useAccess } from "@/modules/access";
import { isEnabled } from "@/modules/registry";
import type { ModuleDefinition, NavEntry } from "@/modules/types";
import { CountMark } from "@/ui/count-mark";
import { useActiveModule, useActiveModuleId, useVisibleModules } from "./use-shell-modules";
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
    <Link
      to="/"
      className={cn(
        "inline-flex shrink-0 items-center gap-2 rounded-control font-heading text-[1.125rem] leading-none font-title tracking-[-0.025em] outline-none focus-visible:ring-2 focus-visible:ring-focus",
        className,
      )}
    >
      <span
        aria-hidden
        className="pv-nav-pill-active grid size-7 place-items-center rounded-[9px] text-[0.8125rem] font-title"
      >
        P
      </span>
      <span className="text-[1.125rem]">{t("app.name")}</span>
    </Link>
  );
}

/* ---------------------------------------------------------------- topbar with dropdown secondary menus */

const topNavTriggerClass = (active: boolean, dimmed: boolean) =>
  cn(
    "relative flex h-(--pv-control-h) items-center gap-1 rounded-control px-3 text-sm whitespace-nowrap outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus",
    active ? "bg-nav-active font-strong text-nav-active-fg" : "font-mid text-shell-fg-muted hover:bg-shell-field/60 hover:text-shell-fg",
    dimmed && !active && "opacity-70",
  );

function ActiveIndicator() {
  return (
    <span
      aria-hidden
      className="absolute inset-x-2.5 h-0.5 bg-nav-indicator"
      style={{ bottom: "calc((var(--pv-control-h) + 6px - var(--pv-shell-h)) / 2)" }}
    />
  );
}

function TopModuleDropdown({
  module,
  entries,
  active,
  activeEntry,
  dimmed,
}: {
  module: ModuleDefinition;
  entries: NavEntry[];
  active: boolean;
  activeEntry: NavEntry | undefined;
  dimmed: boolean;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <Dropdown>
      <Dropdown.Trigger aria-current={active ? "page" : undefined} className={topNavTriggerClass(active, dimmed)}>
        {t(module.titleKey)}
        <Badge module={module} />
        <ChevronDown className="size-3.5 shrink-0 opacity-65" aria-hidden />
        {active ? <ActiveIndicator /> : null}
      </Dropdown.Trigger>
      <Dropdown.Popover placement="bottom start" className="min-w-56">
        <Dropdown.Menu
          aria-label={t(module.titleKey)}
          onAction={(key) => {
            const target = entries.find((entry) => entry.to === String(key));
            if (target) void navigate({ to: target.to });
          }}
        >
          <Dropdown.Section>
            <Header className="px-2 py-1 text-xs font-mid text-fg-muted">{t(module.titleKey)}</Header>
            {entries.map((entry) => {
              const Icon = entry.icon;
              const selected = activeEntry?.to === entry.to;
              return (
                <Dropdown.Item key={entry.to} id={entry.to} textValue={t(entry.titleKey)}>
                  {Icon ? <Icon className="size-4 text-fg-muted" aria-hidden /> : null}
                  <Label className="flex-1">{t(entry.titleKey)}</Label>
                  {selected ? <Check className="size-3.5 text-fg" aria-hidden /> : null}
                </Dropdown.Item>
              );
            })}
          </Dropdown.Section>
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}

/** One row of navigation inside the top bar, with floating dropdown secondary menus. */
export function TopNav({ className }: { className?: string }) {
  const { t } = useTranslation();
  const modules = useVisibleModules();
  const { module: activeModule, entry: activeEntry } = useActiveModule();
  const activeId = activeModule?.id;
  const access = useAccess();
  return (
    <nav aria-label={t("a11y.navigation")} className={cn("no-scrollbar min-w-0 overflow-x-auto", className)}>
      <ul className="mx-auto flex w-max items-center gap-0.5 rounded-control bg-nav-track p-[3px]">
        {modules.map((module) => {
          const active = module.id === activeId;
          const dimmed = !isEnabled(module, access);
          const entries = (module.nav ?? []).filter((entry) => isEnabled(entry, access));
          const Menu = module.sidebar?.menu;
          const label = <>
            {t(module.titleKey)}
            <Badge module={module} />
            {active ? <ActiveIndicator /> : null}
          </>;
          const triggerClass = topNavTriggerClass(active, dimmed);
          const homeLink = <Link to={module.home} aria-current={active ? "page" : undefined} className={triggerClass}>{label}</Link>;
          return (
            <li key={module.id}>
              {Menu && !dimmed ? (
                <Suspense fallback={homeLink}><Menu className={triggerClass} active={active}>{label}</Menu></Suspense>
              ) : entries.length >= 2 ? (
                <TopModuleDropdown module={module} entries={entries} active={active} activeEntry={active ? activeEntry : undefined} dimmed={dimmed} />
              ) : homeLink}
            </li>
          );
        })}
      </ul>
    </nav>
  );
}

/* ---------------------------------------------------------------- sidebar with collapsible secondary menus */

function SidebarItem({ module, current }: { module: ModuleDefinition; current?: "page" | "location" }) {
  const active = Boolean(current);
  const { t } = useTranslation();
  const access = useAccess();
  const Icon = module.icon;
  return (
    <Link
      to={module.home}
      aria-current={current}
      className={cn(
        "flex h-9.5 items-center gap-2.5 rounded-xl px-3 text-sm outline-none transition-all duration-150 focus-visible:ring-2 focus-visible:ring-focus",
        active
          ? "pv-nav-pill-active font-strong"
          : "font-mid text-shell-fg/82 hover:bg-hover/80 hover:text-shell-fg",
        !isEnabled(module, access) && !active && "opacity-60",
      )}
    >
      <Icon className={cn("size-[1.0625rem] shrink-0 transition-colors", active ? "text-white" : "text-icon")} aria-hidden />
      <span className="min-w-0 flex-1 truncate">{t(module.titleKey)}</span>
      <Badge module={module} />
    </Link>
  );
}

function SidebarGroupItem({
  module,
  entries,
  active,
  activeEntry,
  expanded,
  onToggle,
}: {
  module: ModuleDefinition;
  entries: NavEntry[];
  active: boolean;
  activeEntry: NavEntry | undefined;
  expanded: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  const access = useAccess();
  const Icon = module.icon;
  const parentHighlighted = active && !expanded;
  return (
    <div className="grid gap-0.5">
      <div
        className={cn(
          "flex h-9.5 items-center rounded-xl transition-all duration-150",
          parentHighlighted
            ? "pv-nav-pill-active"
            : "text-shell-fg/82 hover:bg-hover/80 hover:text-shell-fg",
          !isEnabled(module, access) && !active && "opacity-60",
        )}
      >
        <Link
          to={module.home}
          aria-current={parentHighlighted ? "page" : undefined}
          className={cn(
            "flex min-w-0 flex-1 items-center gap-2.5 ps-3 pe-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-focus",
            parentHighlighted ? "font-strong text-white" : "font-mid text-inherit",
          )}
        >
          <Icon className={cn("size-[1.0625rem] shrink-0 transition-colors", parentHighlighted ? "text-white" : "text-icon")} aria-hidden />
          <span className="min-w-0 flex-1 truncate">{t(module.titleKey)}</span>
          <Badge module={module} />
        </Link>
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={expanded}
          aria-label={t("nav.toggleSubmenu", { name: t(module.titleKey) })}
          className={cn(
            "me-1 grid size-7 shrink-0 place-items-center rounded-lg outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus",
            parentHighlighted ? "text-white/80 hover:text-white" : "text-shell-fg-muted hover:text-shell-fg",
          )}
        >
          <ChevronDown className={cn("size-3.5 transition-transform duration-150", !expanded && "-rotate-90")} aria-hidden />
        </button>
      </div>
      {expanded ? (
        <ul className="ms-4.5 mt-0.5 grid gap-0.5 border-s border-shell-line/70 ps-2.5">
          {entries.map((entry) => {
            const selected = active && activeEntry?.to === entry.to;
            return (
              <li key={entry.to}>
                <Link
                  to={entry.to}
                  aria-current={selected ? "page" : undefined}
                  className={cn(
                    "flex h-8 items-center rounded-lg px-2.5 text-[0.8125rem] outline-none transition-all duration-150 focus-visible:ring-2 focus-visible:ring-focus",
                    selected
                      ? "pv-nav-pill-active font-strong"
                      : "text-shell-fg-muted hover:bg-hover/75 hover:text-shell-fg",
                  )}
                >
                  <span className="min-w-0 flex-1 truncate">{t(entry.titleKey)}</span>
                </Link>
              </li>
            );
          })}
        </ul>
      ) : null}
    </div>
  );
}

const SIDEBAR_GROUP_LABELS: Partial<Record<NonNullable<ModuleDefinition["group"]>, "nav.media" | "nav.space" | "nav.manage">> = {
  media: "nav.media",
  space: "nav.space",
  manage: "nav.manage",
};

export function Sidebar({ onCollapse, side = "left" }: { onCollapse?: () => void; side?: "left" | "right" }) {
  const { t } = useTranslation();
  const modules = useVisibleModules();
  const { module: activeModule, entry: activeEntry, pathname } = useActiveModule();
  const activeId = activeModule?.id;
  const access = useAccess();
  const [toggled, setToggled] = useState<Record<string, boolean>>({});
  const groups: ModuleDefinition[][] = [];
  for (const module of modules) {
    const previous = groups.at(-1);
    const key = module.group ?? "main";
    if (previous && (previous[0]?.group ?? "main") === key) previous.push(module);
    else groups.push([module]);
  }

  const isExpanded = (moduleId: string) => toggled[moduleId] ?? (moduleId === activeId || moduleId === "system");
  const toggleGroup = (moduleId: string) => setToggled((prev) => ({ ...prev, [moduleId]: !isExpanded(moduleId) }));

  const renderModule = (module: ModuleDefinition) => {
    const entries = (module.nav ?? []).filter((entry) => isEnabled(entry, access));
    if (entries.length >= 2) {
      return (
        <SidebarGroupItem
          key={module.id}
          module={module}
          entries={entries}
          active={module.id === activeId}
          activeEntry={module.id === activeId ? activeEntry : undefined}
          expanded={isExpanded(module.id)}
          onToggle={() => toggleGroup(module.id)}
        />
      );
    }
    const current = module.id === activeId ? (pathname === module.home ? "page" : "location") : undefined;
    return <SidebarItem key={module.id} module={module} current={current} />;
  };

  const CollapseIcon = side === "right" ? PanelRightClose : PanelLeftClose;

  return (
    <aside
      data-sidebar-side={side}
      className={cn(
        "pv-glass-shell flex h-full w-(--pv-sidebar-w) shrink-0 flex-col text-shell-fg",
        side === "right" ? "border-s" : "border-e",
      )}
    >
      <div className="flex h-(--pv-shell-h) shrink-0 items-center justify-between px-4">
        <Wordmark className="text-heading" />
        {onCollapse ? (
          <button
            type="button"
            onClick={onCollapse}
            aria-label={t("a11y.toggleSidebar")}
            className="grid size-8 place-items-center rounded-control text-shell-fg-muted outline-none transition-colors hover:bg-hover hover:text-shell-fg focus-visible:ring-2 focus-visible:ring-focus"
          >
            <CollapseIcon className="size-4" aria-hidden />
          </button>
        ) : null}
      </div>
      <nav aria-label={t("a11y.navigation")} className="sidebar-scroll flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-3 pt-1 pb-5">
        {groups.map((group) => {
          const groupKind = group[0]?.group;
          const labelKey = groupKind ? SIDEBAR_GROUP_LABELS[groupKind] : undefined;
          return (
            <div key={group[0]!.id} className="grid gap-4">
              <div className="grid gap-0.5">
                {labelKey ? (
                  <p className="px-2.5 pb-1.5 text-[0.6875rem] font-strong tracking-[0.04em] text-shell-fg-muted/80 uppercase">
                    {t(labelKey)}
                  </p>
                ) : null}
                {group.map(renderModule)}
              </div>
              {group.map((module) => {
                if (!module.sidebar || !isEnabled(module, access)) return null;
                const Section = module.sidebar.component;
                return (
                  <div key={module.id} className="grid gap-0.5">
                    <p className="px-2.5 pb-1.5 text-[0.6875rem] font-strong tracking-[0.04em] text-shell-fg-muted/80 uppercase">
                      {t(module.sidebar.titleKey)}
                    </p>
                    <Suspense><Section /></Suspense>
                  </div>
                );
              })}
            </div>
          );
        })}
      </nav>
      <div className="border-t border-shell-line/70 p-3">
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
        <div className="grid h-16 auto-cols-fr grid-flow-col">
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
          {rest.length > 0 && (
            <button
              type="button"
              onClick={() => setMoreOpen(true)}
              aria-current={restActive ? "page" : undefined}
              className={cn("flex flex-col items-center justify-center gap-0.5 text-[0.6875rem]", restActive ? "font-strong text-shell-fg" : "text-shell-fg-muted")}
            >
              <MoreHorizontal className={cn("size-[1.35rem]", restActive && "text-active-mark")} aria-hidden />
              {t("nav.more")}
            </button>
          )}
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
