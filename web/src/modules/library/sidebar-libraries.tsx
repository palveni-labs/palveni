import { Dropdown, Header, Label, Separator } from "@heroui/react";
import { Link, useNavigate, useParams, useRouterState } from "@tanstack/react-router";
import { Check, ChevronDown, FolderOpen, Layers, Link2, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useLibraries } from "@/api/queries/libraries";
import type { NavMenuProps } from "@/modules/types";
import type { LibraryKind } from "@/api/types";
import { cn } from "@/lib/cn";
import { formatNumber } from "@/lib/format";
import { LIBRARY_KIND_META, LIBRARY_KINDS } from "@/lib/kinds";
import { useFeature } from "@/modules/access";
import { useSlot } from "@/skins/scope";
import { useScanningLibraries } from "./use-scanning";

const KIND_SWATCH: Record<LibraryKind, string> = {
  video: "#963340",
  music: "#275B8A",
  audiobook: "#735832",
  podcast: "#376D67",
  photo: "#C27A2C",
  book: "#5B605C",
  comic: "#4A692C",
};

function useSortedLibraries() {
  const { data } = useLibraries();
  return [...(data?.libraries ?? [])].sort(
    (a, b) => LIBRARY_KINDS.indexOf(a.kind) - LIBRARY_KINDS.indexOf(b.kind) || a.name.localeCompare(b.name),
  );
}

/** The libraries as a list in a sidebar navigation. */
export default function SidebarLibraries() {
  const { i18n } = useTranslation();
  const params = useParams({ strict: false });
  const videoEnabled = useFeature("video");
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const itemColor = useSlot("itemColor");
  const musicEnabled = useFeature("music");
  const photosEnabled = useFeature("photos");
  const readingEnabled = useFeature("reading");
  const scanning = useScanningLibraries();
  const libraries = useSortedLibraries();
  return (
    <ul className="grid gap-0.5">
      {libraries.map((library) => {
        const Icon = LIBRARY_KIND_META[library.kind].icon;
        const active = (pathname.startsWith("/library/") || pathname.startsWith("/video/")) && params.libraryId === library.id;
        const linkClass = cn(
          "relative flex h-[30px] items-center gap-2.5 rounded-control px-2.5 text-[0.8125rem] outline-none transition-all duration-150 focus-visible:ring-2 focus-visible:ring-focus",
          active ? "pv-nav-pill-active font-strong" : "font-mid text-shell-fg/85 hover:bg-hover/80 hover:text-shell-fg",
        );
        const content = (
          <>
            {itemColor === "on" ? (
              <span
                aria-hidden
                className="size-2 shrink-0 rounded-full ring-2 ring-current/15"
                style={{ backgroundColor: KIND_SWATCH[library.kind] }}
              />
            ) : (
              <Icon className={cn("size-4 shrink-0", active ? "text-current" : "text-icon")} aria-hidden />
            )}
            <span className="min-w-0 flex-1 truncate">{library.name}</span>
            {scanning.has(library.id) ? <Loader2 className={cn("pv-spin size-3.5", active ? "text-current" : "text-active-mark")} aria-hidden /> : null}
            <span
              data-num
              className={cn(
                "rounded-full px-1.5 py-0.5 text-[0.6875rem] font-mid tabular-nums",
                active ? "bg-current/12 text-current" : "bg-sunken/85 text-shell-fg-muted",
              )}
            >
              {formatNumber(library.itemCount, i18n.resolvedLanguage ?? "en")}
            </span>
          </>
        );
        return (
          <li key={library.id}>
            {musicEnabled && library.kind === "music" ? (
              <Link to="/music" search={{ library: library.id }} className={linkClass}>
                {content}
              </Link>
            ) : readingEnabled && (library.kind === "book" || library.kind === "comic") ? (
              <Link to="/reading" search={{ library: library.id }} className={linkClass}>
                {content}
              </Link>
            ) : photosEnabled && library.kind === "photo" ? (
              <Link to="/photos" search={{ libraryId: library.id }} className={linkClass}>
                {content}
              </Link>
            ) : (
              <Link
                to={videoEnabled && library.kind === "video" ? "/video/$libraryId" : "/library/$libraryId"}
                params={{ libraryId: library.id }}
                aria-current={active ? "page" : undefined}
                className={linkClass}
              >
                {content}
              </Link>
            )}
          </li>
        );
      })}
    </ul>
  );
}

export function LibraryMenu({ children, className, active }: NavMenuProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const params = useParams({ strict: false });
  const videoEnabled = useFeature("video");
  const musicEnabled = useFeature("music");
  const photosEnabled = useFeature("photos");
  const readingEnabled = useFeature("reading");
  const collectionsEnabled = useFeature("collections");
  const sharesEnabled = useFeature("shares");
  const libraries = useSortedLibraries();

  if (libraries.length === 0) {
    return (
      <Link to="/library" aria-current={active ? "page" : undefined} className={className}>
        {children}
      </Link>
    );
  }

  return (
    <Dropdown>
      <Dropdown.Trigger aria-current={active ? "page" : undefined} className={className}>
        {children}
        <ChevronDown className="size-3.5 shrink-0 opacity-65" aria-hidden />
      </Dropdown.Trigger>
      <Dropdown.Popover placement="bottom start" className="min-w-60">
        <Dropdown.Menu
          aria-label={t("nav.libraries")}
          onAction={(key) => {
            const id = String(key);
            if (id === "__all__") void navigate({ to: "/library" });
            else if (id === "__collections__") void navigate({ to: "/library/collections" });
            else if (id === "__shares__") void navigate({ to: "/library/shares" });
            else {
              const target = libraries.find((entry) => entry.id === id);
              if (musicEnabled && target?.kind === "music") {
                void navigate({ to: "/music", search: { library: id } });
              } else if (readingEnabled && (target?.kind === "book" || target?.kind === "comic")) {
                void navigate({ to: "/reading", search: { library: id } });
              } else if (photosEnabled && target?.kind === "photo") {
                void navigate({ to: "/photos", search: { libraryId: id } });
              } else if (videoEnabled && target?.kind === "video") {
                void navigate({ to: "/video/$libraryId", params: { libraryId: id } });
              } else {
                void navigate({ to: "/library/$libraryId", params: { libraryId: id } });
              }
            }
          }}
        >
          <Dropdown.Section>
            <Header className="px-2 py-1 text-xs font-mid text-fg-muted">{t("nav.libraries")}</Header>
            {libraries.map((library) => {
              const Icon = LIBRARY_KIND_META[library.kind].icon;
              const selected = params.libraryId === library.id;
              return (
                <Dropdown.Item key={library.id} id={library.id} textValue={library.name}>
                  <Icon className="size-4 text-fg-muted" aria-hidden />
                  <Label className="min-w-0 flex-1 truncate">{library.name}</Label>
                  <span data-num className="text-xs text-fg-muted tabular-nums">
                    {formatNumber(library.itemCount, i18n.resolvedLanguage ?? "en")}
                  </span>
                  {selected ? <Check className="size-3.5 text-fg" aria-hidden /> : null}
                </Dropdown.Item>
              );
            })}
          </Dropdown.Section>
          <Separator />
          <Dropdown.Item id="__all__" textValue={t("nav.allLibraries")}>
            <FolderOpen className="size-4 text-fg-muted" aria-hidden />
            <Label>{t("nav.allLibraries")}</Label>
          </Dropdown.Item>
          {collectionsEnabled ? (
            <Dropdown.Item id="__collections__" textValue={t("nav.collections")}>
              <Layers className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("nav.collections")}</Label>
            </Dropdown.Item>
          ) : null}
          {sharesEnabled ? (
            <Dropdown.Item id="__shares__" textValue={t("nav.shares")}>
              <Link2 className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("nav.shares")}</Label>
            </Dropdown.Item>
          ) : null}
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}
