import { buttonVariants, Dropdown, Label } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { FolderOpen, FolderPlus, Library as LibraryIcon, MoreHorizontal, RefreshCw, Settings2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries, useScanLibrary } from "@/api/queries/libraries";
import type { Library } from "@/api/types";
import { RelativeTime } from "@/components/relative-time";
import { formatNumber } from "@/lib/format";
import { LIBRARY_KIND_META, LIBRARY_KINDS } from "@/lib/kinds";
import { useSlot } from "@/skins/scope";
import { StatusLabel } from "@/ui/marks";
import { useSectionVariant } from "@/ui/section";
import { EmptyState, LoadingRows } from "@/ui/states";
import { CoverMosaic } from "@/ui/work";
import { useScanningLibraries } from "../use-scanning";

function LibraryCell({ library, covers, scanning }: { library: Library; covers: string[]; scanning: boolean }) {
  const { t, i18n } = useTranslation();
  const itemColor = useSlot("itemColor");
  const progress = useSlot("progress");
  const isAdmin = useIsAdmin();
  const scan = useScanLibrary();
  const Icon = LIBRARY_KIND_META[library.kind].icon;
  const empty = library.itemCount === 0 && !library.lastScanAt;
  const count = `${formatNumber(library.itemCount, i18n.resolvedLanguage ?? "en")} ${t(`units.${library.kind}`)}`;
  return (
    <div className={cn("relative flex min-h-[4.75rem] items-center gap-3 py-3.5 ps-(--pv-pad-panel) pe-2.5", empty && "bg-sunken/60")}>
      {itemColor === "on" ? (
        empty ? (
          <span className="grid size-11 shrink-0 place-items-center rounded-tile border-[1.5px] border-dashed border-line-strong text-fg-subtle">
            <Icon className="size-[1.1rem]" aria-hidden />
          </span>
        ) : (
          <CoverMosaic artworkIds={covers} title={library.name} icon={Icon} className="size-11 shrink-0 rounded-tile ring-1 ring-line ring-inset" />
        )
      ) : (
        <Icon className="size-[1.15rem] shrink-0 text-icon" aria-hidden />
      )}
      <Link to="/library/$libraryId" params={{ libraryId: library.id }} className="grid min-w-0 flex-1 gap-0.5 outline-none after:absolute after:inset-0 focus-visible:after:ring-2 focus-visible:after:ring-focus">
        <span className="truncate text-md font-strong text-fg">{library.name}</span>
        <span className="truncate text-sm text-fg-muted">
          <span data-num className="tabular-nums">
            {count}
          </span>
          <span aria-hidden> · </span>
          {scanning ? (
            <StatusLabel tone="active" className="text-sm">
              {t("libraries.scanning")}
            </StatusLabel>
          ) : library.lastScanAt ? (
            <RelativeTime value={library.lastScanAt} format={(time) => t("libraries.scannedAt", { time })} />
          ) : (
            t("libraries.neverScanned")
          )}
        </span>
      </Link>
      {empty && isAdmin ? (
        <Link to="/system/libraries" className={cn(buttonVariants({ variant: "ghost", size: "sm" }), "relative z-10 shrink-0")}>
          <FolderPlus aria-hidden />
          {t("libraries.addFolder")}
        </Link>
      ) : (
        <Dropdown>
          <Dropdown.Trigger aria-label={t("libraries.actionsFor", { name: library.name })} className="relative z-10 grid size-8 shrink-0 place-items-center rounded-control text-fg-muted outline-none hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus">
            <MoreHorizontal className="size-4" aria-hidden />
          </Dropdown.Trigger>
          <Dropdown.Popover placement="bottom end">
            <Dropdown.Menu
              aria-label={library.name}
              onAction={(key) => {
                if (key === "scan") scan.mutate(library.id);
              }}
            >
              <Dropdown.Item id="browse" textValue={t("adminLibraries.browse")} href={`/library/${library.id}`}>
                <FolderOpen className="size-4 text-fg-muted" aria-hidden />
                <Label>{t("adminLibraries.browse")}</Label>
              </Dropdown.Item>
              {isAdmin ? (
                <Dropdown.Item id="scan" textValue={t("adminLibraries.scan")}>
                  <RefreshCw className="size-4 text-fg-muted" aria-hidden />
                  <Label>{t("adminLibraries.scan")}</Label>
                </Dropdown.Item>
              ) : null}
              {isAdmin ? (
                <Dropdown.Item id="manage" textValue={t("nav.manageLibraries")} href="/system/libraries">
                  <Settings2 className="size-4 text-fg-muted" aria-hidden />
                  <Label>{t("nav.manageLibraries")}</Label>
                </Dropdown.Item>
              ) : null}
            </Dropdown.Menu>
          </Dropdown.Popover>
        </Dropdown>
      )}
      {scanning && progress === "separator" ? <span aria-hidden className="pv-dashed absolute inset-x-0 bottom-0 h-px text-active-mark" /> : null}
    </div>
  );
}

/** 媒体库: every library at a glance, three to a row. */
export default function LibrariesWidget() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const variant = useSectionVariant();
  const { data, isPending } = useLibraries();
  const scanning = useScanningLibraries();
  if (isPending) return <LoadingRows rows={3} className="px-5 pb-4" />;
  const libraries = [...(data?.libraries ?? [])].sort(
    (a, b) => LIBRARY_KINDS.indexOf(a.kind) - LIBRARY_KINDS.indexOf(b.kind) || a.name.localeCompare(b.name),
  );
  if (libraries.length === 0) {
    return (
      <EmptyState
        compact
        icon={LibraryIcon}
        title={isAdmin ? t("libraries.emptyAdmin") : t("libraries.emptyUser")}
        action={
          isAdmin ? (
            <Link to="/system/libraries" search={{ create: true }} className={buttonVariants({ variant: "primary" })}>
              <FolderPlus aria-hidden />
              {t("adminLibraries.create")}
            </Link>
          ) : null
        }
      />
    );
  }
  return (
    <ul
      className={cn(
        "grid sm:grid-cols-2 lg:grid-cols-3",
        variant === "plain" ? "gap-1 [&>li]:rounded-tile [&>li]:bg-sunken" : "border-t border-line [&>li]:border-line max-sm:divide-y max-sm:divide-line",
        variant !== "plain" &&
          "sm:max-lg:[&>li:nth-child(even)]:border-s sm:max-lg:[&>li:nth-child(n+3)]:border-t lg:[&>li:nth-child(n+4)]:border-t lg:[&>li:not(:nth-child(3n+1))]:border-s",
        variant === "rule" && "[&>li:first-child>div]:ps-0 lg:[&>li:nth-child(3n+1)>div]:ps-0",
      )}
    >
      {libraries.map((library) => (
        <li key={library.id} className="min-w-0">
          <LibraryCell library={library} covers={[]} scanning={scanning.has(library.id)} />
        </li>
      ))}
    </ul>
  );
}
