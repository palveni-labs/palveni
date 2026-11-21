/**
 * The bar across the top. With `nav: topbar` it carries the wordmark, server and status, the
 * tabs and the global tools; with `nav: sidebar` it is the content's toolbar (page title when
 * `pageTitle: toolbar`, the status display, the tools).
 */
import { Button, Kbd } from "@heroui/react";
import { Link, useParams } from "@tanstack/react-router";
import { PanelLeftOpen, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useLibrary } from "@/api/queries/libraries";
import { useSlot } from "@/skins/scope";
import { registerToolbarActions } from "@/ui/page";
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
          "hidden h-9 w-[13.5rem] min-w-0 items-center gap-2 rounded-field ps-3 pe-1.5 text-sm outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus xl:flex",
          tone === "shell" ? "bg-shell-field text-shell-fg-muted hover:text-shell-fg" : "bg-sunken text-fg-muted hover:text-fg",
        )}
      >
        <Search className="size-4 shrink-0" aria-hidden />
        <span className="min-w-0 flex-1 truncate text-start">{t("palette.placeholder")}</span>
        <Kbd className="shrink-0">⌘K</Kbd>
      </button>
      <Button
        variant="ghost"
        isIconOnly
        onPress={() => shell.setPalette(true)}
        aria-label={t("palette.title")}
        className={cn("xl:hidden", tone === "shell" && "text-shell-fg-muted data-[hovered=true]:bg-shell-field")}
      >
        <Search aria-hidden />
      </Button>
    </>
  );
}

function Tools({ tone }: { tone: Tone }) {
  return (
    <div className="flex shrink-0 items-center justify-end gap-1.5">
      <SearchButton tone={tone} />
      <Inbox tone={tone} />
      <UserMenu />
    </div>
  );
}

function LibraryName({ libraryId }: { libraryId: string }) {
  const { data } = useLibrary(libraryId);
  return <>{data?.name ?? "…"}</>;
}

/** Current page, for the toolbar title. */
function ToolbarTitle() {
  const { t } = useTranslation();
  const { module, entry } = useActiveModule();
  const params = useParams({ strict: false });
  if (!module) return null;
  const sub = entry ? t(entry.titleKey) : params.libraryId ? <LibraryName libraryId={params.libraryId} /> : null;
  return (
    <div className="flex min-w-0 items-baseline gap-2 text-[1.0625rem]">
      {sub ? (
        <>
          <Link to={module.home} className="truncate text-fg-muted hover:text-fg">
            {t(module.titleKey)}
          </Link>
          <span className="text-fg-subtle" aria-hidden>
            /
          </span>
          <span className="truncate font-strong text-heading">{sub}</span>
        </>
      ) : (
        <span className="truncate font-strong text-heading">{t(module.titleKey)}</span>
      )}
    </div>
  );
}

/** Top bar for `nav: topbar`. */
export function TopBar() {
  const status = useSlot("status");
  const navPhone = useSlot("navPhone");
  return (
    <div className="shrink-0">
      <header className="relative z-20 flex h-(--pv-shell-h) items-center gap-4 border-b border-shell-line bg-shell px-4 text-shell-fg md:grid md:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] md:px-5 lg:px-6">
        <div className="flex min-w-0 flex-1 items-center gap-1">
          <Wordmark className="me-2 text-shell-fg" />
          <ServerSwitch className="max-md:hidden" />
          {status === "inline" ? <InlineStatus className="ms-1 max-sm:hidden" /> : null}
        </div>
        <TopNav className="max-md:hidden" />
        <div className="flex min-w-0 items-center justify-end gap-3">
          {status === "activity-window" ? <ActivityWindow className="max-w-[26rem] max-lg:hidden" /> : null}
          <Tools tone="shell" />
        </div>
      </header>
      {navPhone === "top-scroll" ? <ScrollTabs /> : null}
      {status === "activity-window" ? <ActivityWindow className="m-2 lg:hidden" /> : null}
    </div>
  );
}

/** Toolbar for `nav: sidebar` (the sidebar carries the wordmark and navigation). */
export function Toolbar({ sidebarOpen, onOpenSidebar }: { sidebarOpen: boolean; onOpenSidebar: () => void }) {
  const { t } = useTranslation();
  const status = useSlot("status");
  const pageTitle = useSlot("pageTitle");
  const navPhone = useSlot("navPhone");
  return (
    <div className="shrink-0">
      <header className="relative z-20 flex h-(--pv-shell-h) items-center gap-3 border-b border-line bg-page px-4 md:px-6">
        <Wordmark className="text-heading md:hidden" />
        {!sidebarOpen ? (
          <button type="button" onClick={onOpenSidebar} aria-label={t("a11y.toggleSidebar")} className="grid size-8 place-items-center rounded-tile text-fg-muted hover:bg-hover max-md:hidden">
            <PanelLeftOpen className="size-4" aria-hidden />
          </button>
        ) : null}
        {/* The title keeps its width (up to a limit); the activity window gives way first. */}
        <div className="flex max-w-[22rem] min-w-0 shrink-0 items-center max-md:hidden">{pageTitle === "toolbar" ? <ToolbarTitle /> : null}</div>
        <div className="flex-1 max-md:hidden" />
        {status === "activity-window" ? (
          <ActivityWindow className="w-[min(37rem,40vw)] min-w-[14rem] shrink max-lg:hidden" />
        ) : (
          <>
            <ServerSwitch tone="plain" className="max-md:hidden" />
            <InlineStatus />
          </>
        )}
        <div ref={registerToolbarActions} className="flex shrink-0 items-center gap-1.5 max-md:hidden empty:hidden" />
        <div className="flex shrink-0 justify-end max-md:flex-1">
          <Tools tone="plain" />
        </div>
      </header>
      {status === "activity-window" ? <ActivityWindow className="mx-3 mt-2 lg:hidden" /> : null}
      {navPhone === "top-scroll" ? <ScrollTabs /> : null}
    </div>
  );
}
