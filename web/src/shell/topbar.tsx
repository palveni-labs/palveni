/**
 * The bar across the top. With `nav: topbar` it carries the wordmark, server and status, the
 * single-line navigation with dropdown secondary menus, and the global tools; with `nav: sidebar`
 * it is the content's toolbar (page breadcrumb, activity/status, page actions and global tools).
 */
import { Button, Kbd } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ChevronRight, MessageSquareText, PanelLeftOpen, PanelRightOpen, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useSlot } from "@/skins/scope";
import { CastDevicesPopover } from "@/player/cast-devices-popover";
import { registerToolbarActions, registerToolbarTitle } from "@/ui/page";
import { Inbox } from "./inbox";
import { ScrollTabs, TopNav, Wordmark } from "./nav";
import { shell, useShellState } from "./shell-store";
import { ActivityWindow, InlineStatus, ServerSwitch } from "./status";
import { useActiveModule } from "./use-shell-modules";
import { usePanelAvailable } from "./use-panel";
import { UserMenu } from "./user-menu";

type Tone = "shell" | "plain";

function SearchButton({ tone, prominent = false }: { tone: Tone; prominent?: boolean }) {
  const { t } = useTranslation();
  return (
    <>
      <button
        type="button"
        onClick={() => shell.setPalette(true)}
        className={cn(
          "hidden h-9 min-w-0 items-center gap-2 rounded-control border px-3 text-sm outline-none transition-all duration-150 focus-visible:ring-2 focus-visible:ring-focus",
          prominent ? "w-[17rem] max-w-[34vw] md:flex" : "w-[13rem] xl:flex",
          tone === "shell"
            ? "border-shell-line bg-shell-field text-shell-fg-muted hover:border-line-strong hover:text-shell-fg"
            : "border-line/80 bg-surface/85 text-fg-muted shadow-panel hover:border-line-strong hover:bg-surface hover:text-fg",
        )}
      >
        <Search className="size-3.5 shrink-0" aria-hidden />
        <span className="min-w-0 flex-1 truncate text-start text-xs">{t("palette.placeholder")}</span>
        <Kbd className="shrink-0">⌘K</Kbd>
      </button>
      <Button
        variant="ghost"
        isIconOnly
        size="sm"
        onPress={() => shell.setPalette(true)}
        aria-label={t("palette.title")}
        className={cn(prominent ? "md:hidden" : "xl:hidden", tone === "shell" && "text-shell-fg-muted data-[hovered=true]:bg-shell-field")}
      >
        <Search aria-hidden />
      </Button>
    </>
  );
}

/** Compact trigger for the ⌘J assistant panel (avoids repeating the primary "助手" nav item). */
function AssistantButton({ tone }: { tone: Tone }) {
  const { t } = useTranslation();
  const { panelOpen } = useShellState();
  if (!usePanelAvailable()) return null;
  return (
    <Button
      variant={panelOpen ? "secondary" : "ghost"}
      isIconOnly
      size="sm"
      onPress={() => shell.toggleAssistant()}
      aria-pressed={panelOpen}
      aria-label={t("assistant.title")}
      className={cn(tone === "shell" && !panelOpen && "text-shell-fg-muted hover:text-shell-fg data-[hovered=true]:bg-shell-field")}
    >
      <MessageSquareText aria-hidden />
    </Button>
  );
}

function Tools({
  tone,
  hideUserOnDesktop = false,
  hideSearchOnDesktop = false,
}: {
  tone: Tone;
  hideUserOnDesktop?: boolean;
  hideSearchOnDesktop?: boolean;
}) {
  return (
    <div className="flex shrink-0 items-center justify-end gap-1.5">
      <div className={cn(hideSearchOnDesktop && "md:hidden")}>
        <SearchButton tone={tone} />
      </div>
      <CastDevicesPopover tone={tone} placement="bottom end" />
      <AssistantButton tone={tone} />
      <Inbox tone={tone} />
      <div className={cn(hideUserOnDesktop && "md:hidden")}>
        <UserMenu />
      </div>
    </div>
  );
}

function ToolbarTitle() {
  const { t } = useTranslation();
  const { module, entry, pathname } = useActiveModule();
  if (!module) return null;
  return (
    <div className="flex min-w-0 items-center gap-1.5 text-[0.9375rem]">
      {pathname !== module.home && (
        <>
          <Link to={module.home} className="truncate text-fg-muted hover:text-fg">{t(module.titleKey)}</Link>
          <ChevronRight className="size-3.5 shrink-0 text-fg-subtle" aria-hidden />
        </>
      )}
      <span ref={registerToolbarTitle} className="peer truncate font-strong text-heading empty:hidden" />
      <span className="hidden truncate font-strong text-heading peer-empty:block">{t(entry?.titleKey ?? module.titleKey)}</span>
    </div>
  );
}

/** Top bar for `nav: topbar`. Aligned to the centered desktop stage on wide screens. */
export function TopBar() {
  const status = useSlot("status");
  const navPhone = useSlot("navPhone");
  return (
    <div className="shrink-0">
      <header className="relative z-20 flex h-(--pv-shell-h) items-center border-b border-shell-line/75 bg-shell/90 text-shell-fg backdrop-blur-xl">
        <div className="mx-auto flex w-full max-w-(--pv-layout-max) items-center gap-3 px-(--pv-layout-gutter)">
          <div className="flex min-w-0 grow items-center gap-1 md:grow-0">
            <Wordmark className="me-2 text-shell-fg" />
            <ServerSwitch className="max-lg:hidden" />
            {status === "inline" ? <InlineStatus className="ms-1 max-xl:hidden" /> : null}
          </div>
          <TopNav className="flex-1 max-md:hidden" />
          <div className="flex min-w-0 items-center justify-end gap-2">
            {status === "activity-window" ? <ActivityWindow className="max-w-[24rem] max-lg:hidden" /> : null}
            <Tools tone="shell" />
          </div>
        </div>
      </header>
      {navPhone === "top-scroll" ? <ScrollTabs /> : null}
      {status === "activity-window" ? <ActivityWindow className="m-2 lg:hidden" /> : null}
    </div>
  );
}

/** Toolbar for `nav: sidebar` (the sidebar carries the wordmark, primary nav and account). */
export function Toolbar({
  sidebarOpen,
  sidebarSide = "left",
  onOpenSidebar,
}: {
  sidebarOpen: boolean;
  sidebarSide?: "left" | "right";
  onOpenSidebar: () => void;
}) {
  const { t } = useTranslation();
  const status = useSlot("status");
  const pageTitle = useSlot("pageTitle");
  const navPhone = useSlot("navPhone");
  const contentHeader = pageTitle === "content";
  const openButton = !sidebarOpen ? (
    <button
      type="button"
      onClick={onOpenSidebar}
      aria-label={t("a11y.toggleSidebar")}
      className="grid size-8 place-items-center rounded-control text-fg-muted outline-none transition-colors hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus max-md:hidden"
    >
      {sidebarSide === "right" ? <PanelRightOpen className="size-4" aria-hidden /> : <PanelLeftOpen className="size-4" aria-hidden />}
    </button>
  ) : null;
  return (
    <div className="shrink-0 pt-3">
      <div className="mx-auto w-full max-w-(--pv-layout-max) px-(--pv-layout-gutter)">
      <header className="pv-glass-toolbar relative z-20 flex h-13 items-center gap-3 rounded-2xl px-3.5 md:px-4">
        <Wordmark className="text-heading md:hidden" />
        {sidebarSide === "left" ? openButton : null}
        {contentHeader ? (
          <div className="flex min-w-0 items-center max-md:hidden">
            <SearchButton tone="plain" prominent />
          </div>
        ) : (
          <div className="flex max-w-[22rem] min-w-0 shrink-0 items-center max-md:hidden">
            <ToolbarTitle />
          </div>
        )}
        <div className="flex-1 max-md:hidden" />
        {status === "activity-window" ? (
          <ActivityWindow className="w-[min(34rem,38vw)] min-w-[14rem] shrink max-lg:hidden" />
        ) : (
          <div className="flex shrink-0 items-center gap-1 rounded-full border border-line/70 bg-sunken/70 ps-1 pe-3 max-md:hidden">
            <ServerSwitch tone="plain" className="h-7 rounded-full px-2 text-xs" />
            <span aria-hidden className="h-3.5 w-px bg-line/80" />
            <InlineStatus tone="plain" className="ps-1.5" />
          </div>
        )}
        <div ref={registerToolbarActions} className="flex shrink-0 items-center gap-1.5 max-md:hidden empty:hidden" />
        <div className="flex shrink-0 items-center justify-end gap-1.5 max-md:flex-1">
          <Tools tone="plain" hideUserOnDesktop={sidebarOpen} hideSearchOnDesktop={contentHeader} />
          {sidebarSide === "right" ? openButton : null}
        </div>
      </header>
      {status === "activity-window" ? <ActivityWindow className="mt-2 lg:hidden" /> : null}
      {navPhone === "top-scroll" ? <ScrollTabs /> : null}
      </div>
    </div>
  );
}
