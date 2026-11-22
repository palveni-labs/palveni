import { Dropdown, Header, Label, Separator } from "@heroui/react";
import { Link, useNavigate, useParams, useRouterState } from "@tanstack/react-router";
import { Check, ChevronDown, FolderOpen, Loader2 } from "lucide-react";
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
  const scanning = useScanningLibraries();
  const libraries = useSortedLibraries();
  return (
    <ul className="grid gap-0.5">
      {libraries.map((library) => {
        const Icon = LIBRARY_KIND_META[library.kind].icon;
        const active = (pathname.startsWith("/library/") || pathname.startsWith("/video/")) && params.libraryId === library.id;
        const linkClass = cn(
          "flex h-8 items-center gap-2.5 rounded-tile px-2.5 text-sm outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus",
          active ? "bg-nav-active font-strong text-nav-active-fg shadow-panel" : "font-mid text-shell-fg hover:bg-hover",
        );
        const content = (
          <>
            {itemColor === "on" ? (
              <span aria-hidden className="ms-0.5 me-0.5 size-2 shrink-0 rounded-[3px]" style={{ backgroundColor: KIND_SWATCH[library.kind] }} />
            ) : (
              <Icon className={cn("size-4 shrink-0", active ? "text-nav-active-fg" : "text-icon")} aria-hidden />
            )}
            <span className="min-w-0 flex-1 truncate">{library.name}</span>
            {scanning.has(library.id) ? <Loader2 className={cn("pv-spin size-3.5", active ? "text-nav-active-fg" : "text-active-mark")} aria-hidden /> : null}
            <span data-num className={cn("text-xs tabular-nums", active ? "text-nav-active-fg" : "text-shell-fg-muted")}>
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
            else {
              const target = libraries.find((entry) => entry.id === id);
              if (musicEnabled && target?.kind === "music") {
                void navigate({ to: "/music", search: { library: id } });
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
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}
