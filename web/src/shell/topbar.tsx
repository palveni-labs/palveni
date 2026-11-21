/**
 * The bar across the top. With `nav: topbar` it carries the wordmark, server and status, the
 * single-line navigation with dropdown secondary menus, and the global tools; with `nav: sidebar`
 * it is the content's toolbar (page breadcrumb, activity/status, page actions and global tools).
 */
import { Button, Kbd } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ChevronRight, PanelLeftOpen, PanelRightOpen, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useSlot } from "@/skins/scope";
import { registerToolbarActions, registerToolbarTitle } from "@/ui/page";
import { Inbox } from "./inbox";
import { ScrollTabs, TopNav, Wordmark } from "./nav";
import { shell } from "./shell-store";
import { ActivityWindow, InlineStatus, ServerSwitch } from "./status";
import { useActiveModule } from "./use-shell-modules";
import { UserMenu } from "./user-menu";

type Tone = "shell" | "plain";

function SearchButton({ tone }: { tone: Tone }) {
  const { t } = useTranslation();
  return (
    <>
      <button
        type="button"
        onClick={() => shell.setPalette(true)}
        className={cn(
          "hidden h-8 w-[12.5rem] min-w-0 items-center gap-2 rounded-field border border-line ps-2.5 pe-1.5 text-sm outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus xl:flex",
          tone === "shell" ? "border-shell-line bg-shell-field text-shell-fg-muted hover:text-shell-fg" : "bg-sunken text-fg-muted hover:text-fg",
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
        className={cn("xl:hidden", tone === "shell" && "text-shell-fg-muted data-[hovered=true]:bg-shell-field")}
      >
        <Search aria-hidden />
      </Button>
    </>
  );
}

function Tools({ tone, hideUserOnDesktop = false }: { tone: Tone; hideUserOnDesktop?: boolean }) {
  return (
    <div className="flex shrink-0 items-center justify-end gap-1.5">
      <SearchButton tone={tone} />
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
      <header className="relative z-20 flex h-(--pv-shell-h) items-center border-b border-shell-line bg-shell text-shell-fg">
        <div className="mx-auto flex w-full max-w-[77.5rem] items-center gap-3 px-4 md:px-6">
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
  const openButton = !sidebarOpen ? (
    <button
      type="button"
      onClick={onOpenSidebar}
      aria-label={t("a11y.toggleSidebar")}
      className="grid size-8 place-items-center rounded-tile text-fg-muted outline-none hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus max-md:hidden"
    >
      {sidebarSide === "right" ? <PanelRightOpen className="size-4" aria-hidden /> : <PanelLeftOpen className="size-4" aria-hidden />}
    </button>
  ) : null;
  return (
    <div className="shrink-0">
      <header className="relative z-20 flex h-(--pv-shell-h) items-center gap-3 border-b border-line bg-page px-4 md:px-6">
        <Wordmark className="text-heading md:hidden" />
        {sidebarSide === "left" ? openButton : null}
        <div className="flex max-w-[22rem] min-w-0 shrink-0 items-center max-md:hidden">{pageTitle === "toolbar" ? <ToolbarTitle /> : null}</div>
        <div className="flex-1 max-md:hidden" />
        {status === "activity-window" ? (
          <ActivityWindow className="w-[min(34rem,38vw)] min-w-[14rem] shrink max-lg:hidden" />
        ) : (
          <div className="flex shrink-0 items-center gap-1 max-md:hidden">
            <ServerSwitch tone="plain" />
            <InlineStatus tone="plain" />
          </div>
        )}
        <div ref={registerToolbarActions} className="flex shrink-0 items-center gap-1.5 max-md:hidden empty:hidden" />
        <div className="flex shrink-0 items-center justify-end gap-1.5 max-md:flex-1">
          <Tools tone="plain" hideUserOnDesktop={sidebarOpen} />
          {sidebarSide === "right" ? openButton : null}
        </div>
      </header>
      {status === "activity-window" ? <ActivityWindow className="mx-3 mt-2 lg:hidden" /> : null}
      {navPhone === "top-scroll" ? <ScrollTabs /> : null}
    </div>
  );
}
