/**
 * The two views of a folder: a grid of tiles and a list. Both are virtualised against the
 * workspace scroll container, so a folder of 100k files renders only what is on screen, and both
 * fetch the next page as the end of what is loaded comes into view.
 */
import { Skeleton } from "@heroui/react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Check, Folder } from "lucide-react";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { Item, LibraryKind } from "@/api/types";
import { ItemArtwork } from "@/components/artwork";
import { formatBytes, formatDateTime } from "@/lib/format";
import { LIBRARY_KIND_META } from "@/lib/kinds";
import { useScrollContainer } from "@/lib/scroll-container";
import { useElementWidth, useOffsetWithin } from "@/lib/use-element-size";
import { Measure } from "@/ui/marks";
import { useRowInset, useSectionVariant } from "@/ui/section";
import { gridLayout } from "./grid-layout";

export interface ItemViewProps {
  items: Item[];
  libraryKind: LibraryKind;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
  onOpen: (item: Item) => void;
  /** Present while multi-select is on: activating an item toggles it instead of opening it. */
  selection?: ReadonlySet<string>;
}

const LIST_ROW_HEIGHT = 52;

/** The check in the corner of a tile (or at the start of a row) while selecting. */
function SelectMark({ checked, className }: { checked: boolean; className?: string }) {
  return (
    <span
      aria-hidden
      className={cn(
        "grid size-5 shrink-0 place-items-center rounded-full border-[1.5px] transition-colors",
        checked ? "border-selected bg-selected text-selected-fg" : "border-line-control bg-surface",
        className,
      )}
    >
      {checked ? <Check className="size-3" strokeWidth={3} /> : null}
    </span>
  );
}

/** Fetches the next page once the rendered window gets close to the loaded end. */
function useLoadMore(lastIndex: number | undefined, total: number, props: ItemViewProps, threshold: number) {
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = props;
  useEffect(() => {
    if (lastIndex === undefined) return;
    if (lastIndex >= total - threshold && hasNextPage && !isFetchingNextPage) fetchNextPage();
  }, [lastIndex, total, threshold, hasNextPage, isFetchingNextPage, fetchNextPage]);
}

function useItemSubtitle() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  return (item: Item) => (item.kind === "folder" ? t("itemKinds.folder") : item.sizeBytes !== null ? formatBytes(item.sizeBytes, locale) : t(`itemKinds.${item.kind}`));
}

/** Folders as compact tiles above the files: they hold no artwork worth a poster-sized tile. */
function FolderTiles({ folders, onOpen, selection }: { folders: Item[]; onOpen: (item: Item) => void; selection?: ReadonlySet<string> }) {
  return (
    <ul className="grid grid-cols-[repeat(auto-fill,minmax(10.5rem,1fr))] gap-2.5 sm:grid-cols-[repeat(auto-fill,minmax(13rem,1fr))]">
      {folders.map((folder) => {
        const checked = selection?.has(folder.id) ?? false;
        return (
          <li key={folder.id} className="min-w-0">
            <button
              type="button"
              onClick={() => onOpen(folder)}
              aria-pressed={selection ? checked : undefined}
              title={folder.title}
              className={cn(
                "flex h-14 w-full min-w-0 items-center gap-3 rounded-tile border px-3.5 text-start transition-colors outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--pv-focus)",
                checked ? "border-selected bg-selected/10" : "border-line bg-surface hover:bg-hover",
              )}
            >
              {selection ? <SelectMark checked={checked} /> : null}
              <Folder className="size-5 shrink-0 text-icon" strokeWidth={1.7} aria-hidden />
              <span className="truncate text-base font-mid text-fg">{folder.title}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function GroupLabel({ children }: { children: string }) {
  return <h3 className="text-sm font-mid text-fg-muted">{children}</h3>;
}

export function ItemGrid(props: ItemViewProps) {
  const { t } = useTranslation();
  const folders = props.items.filter((item) => item.kind === "folder");
  const files = props.items.filter((item) => item.kind !== "folder");
  const both = folders.length > 0 && (files.length > 0 || props.hasNextPage);
  return (
    <div className="grid gap-4">
      {folders.length > 0 ? (
        <div className="grid gap-2.5">
          {both ? <GroupLabel>{t("libraries.folders")}</GroupLabel> : null}
          <FolderTiles folders={folders} onOpen={props.onOpen} selection={props.selection} />
        </div>
      ) : null}
      {files.length > 0 || props.hasNextPage ? (
        <div className={cn("grid gap-2.5", both && "mt-3")}>
          {both && files.length > 0 ? <GroupLabel>{t("browse.files")}</GroupLabel> : null}
          <FileGrid {...props} items={files} />
        </div>
      ) : null}
    </div>
  );
}

function FileGrid(props: ItemViewProps) {
  const { items, libraryKind, hasNextPage, onOpen, selection } = props;
  const scrollRef = useScrollContainer();
  const containerRef = useRef<HTMLDivElement>(null);
  const width = useElementWidth(containerRef);
  const scrollMargin = useOffsetWithin(containerRef, scrollRef);
  const subtitle = useItemSubtitle();
  const { aspect, aspectClass } = LIBRARY_KIND_META[libraryKind];
  const { columns, gap, rowHeight } = gridLayout(width, aspect);
  const rowCount = Math.ceil(items.length / columns) + (hasNextPage ? 1 : 0);

  // eslint-disable-next-line react-hooks/incompatible-library -- the virtualizer is read during render by design
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 3,
    scrollMargin,
  });

  useEffect(() => {
    virtualizer.measure();
  }, [virtualizer, rowHeight, columns]);

  const virtualRows = virtualizer.getVirtualItems();
  useLoadMore(virtualRows.at(-1)?.index, rowCount, props, 2);

  return (
    <div ref={containerRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
      {width > 0
        ? virtualRows.map((row) => {
            const start = row.index * columns;
            const rowItems = items.slice(start, start + columns);
            return (
              <div
                key={row.key}
                className="absolute inset-x-0 top-0 grid"
                style={{
                  transform: `translateY(${row.start - scrollMargin}px)`,
                  gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                  columnGap: gap,
                  height: rowHeight - gap,
                }}
              >
                {rowItems.length === 0
                  ? Array.from({ length: columns }, (_, index) => (
                      <div key={index} className="flex flex-col gap-2">
                        <Skeleton className={cn("w-full rounded-tile", aspectClass)} />
                        <Skeleton className="h-3.5 w-3/4 rounded-full" />
                      </div>
                    ))
                  : rowItems.map((item) => {
                      const checked = selection?.has(item.id) ?? false;
                      return (
                        <button
                          key={item.id}
                          type="button"
                          onClick={() => onOpen(item)}
                          aria-pressed={selection ? checked : undefined}
                          title={item.title}
                          className="group flex min-w-0 flex-col gap-2 rounded-tile text-start outline-none focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-(--pv-focus)"
                        >
                          <span className="relative block w-full">
                            <ItemArtwork
                              item={item}
                              className={cn(
                                "w-full rounded-tile ring-1 ring-line ring-inset transition-[box-shadow,translate] duration-200 ease-skin group-hover:-translate-y-0.5 group-hover:shadow-raised group-active:translate-y-0",
                                aspectClass,
                                checked && "ring-2 ring-selected",
                              )}
                            />
                            {selection ? <SelectMark checked={checked} className="absolute top-2 start-2" /> : null}
                          </span>
                          <span className="grid min-w-0 gap-0.5 px-0.5">
                            <span className="truncate text-sm font-mid text-fg">{item.title}</span>
                            <span className="truncate text-xs text-fg-muted">{subtitle(item)}</span>
                          </span>
                        </button>
                      );
                    })}
              </div>
            );
          })
        : null}
    </div>
  );
}

export function ItemList(props: ItemViewProps) {
  const { items, hasNextPage, onOpen, selection } = props;
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const variant = useSectionVariant();
  const inset = useRowInset();
  const scrollRef = useScrollContainer();
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollMargin = useOffsetWithin(containerRef, scrollRef);
  const rowCount = items.length + (hasNextPage ? 1 : 0);

  // eslint-disable-next-line react-hooks/incompatible-library -- the virtualizer is read during render by design
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => LIST_ROW_HEIGHT,
    overscan: 10,
    scrollMargin,
  });
  const virtualRows = virtualizer.getVirtualItems();
  useLoadMore(virtualRows.at(-1)?.index, rowCount, props, 15);

  const columns = "grid-cols-[minmax(0,1fr)_auto] sm:grid-cols-[minmax(0,1fr)_6rem_6rem] lg:grid-cols-[minmax(0,1fr)_6rem_6rem_11rem]";
  const plain = variant === "plain";

  return (
    <div>
      <div aria-hidden className={cn("grid items-center gap-4 py-2 text-xs font-mid text-fg-muted", columns, inset, !plain && "border-b border-line")}>
        <span>{t("browse.columns.name")}</span>
        <span className="max-sm:hidden">{t("browse.columns.kind")}</span>
        <span className="text-end">{t("browse.columns.size")}</span>
        <span className="text-end max-lg:hidden">{t("browse.columns.modified")}</span>
      </div>
      <div ref={containerRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
        {virtualRows.map((row) => {
          const item = items[row.index];
          const checked = item ? (selection?.has(item.id) ?? false) : false;
          return (
            <div
              key={row.key}
              className={cn("absolute inset-x-0 top-0", !plain && "border-b border-line", plain && row.index % 2 === 0 && "rounded-tile bg-sunken")}
              style={{ height: LIST_ROW_HEIGHT, transform: `translateY(${row.start - scrollMargin}px)` }}
            >
              {item ? (
                <button
                  type="button"
                  onClick={() => onOpen(item)}
                  aria-pressed={selection ? checked : undefined}
                  className={cn(
                    "grid h-full w-full items-center gap-4 text-start text-base transition-colors outline-none hover:bg-hover focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-(--pv-focus)",
                    plain && "rounded-tile",
                    checked && "bg-selected/12 hover:bg-selected/16",
                    columns,
                    inset,
                  )}
                >
                  <span className="flex min-w-0 items-center gap-3">
                    {selection ? <SelectMark checked={checked} /> : null}
                    <ItemArtwork item={item} bare className="size-8 shrink-0 rounded-[calc(var(--pv-radius-tile)*0.6)]" iconClassName="size-[1.125rem] text-icon" />
                    <span className="truncate text-fg">{item.title}</span>
                  </span>
                  <span className="truncate text-sm text-fg-muted max-sm:hidden">{t(`itemKinds.${item.kind}`)}</span>
                  <span className="text-end text-sm text-fg-muted">{item.sizeBytes !== null ? <Measure value={formatBytes(item.sizeBytes, locale)} /> : <span aria-hidden>—</span>}</span>
                  <span className="text-end text-sm text-fg-muted tabular-nums max-lg:hidden">
                    {item.modifiedAt ? formatDateTime(item.modifiedAt, locale) : <span aria-hidden>—</span>}
                  </span>
                </button>
              ) : (
                <div className={cn("flex h-full items-center gap-3", inset)}>
                  <Skeleton className="size-8 rounded-tile" />
                  <Skeleton className="h-3.5 w-1/3 rounded-full" />
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
