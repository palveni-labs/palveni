import { Button, Dropdown, Input, Label, Spinner, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowDown, ArrowUp, Check, ChevronDown, Disc3, Heart, LayoutGrid, List, RefreshCw, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useScrollContainer } from "@/lib/scroll-container";
import { readLocal, writeLocal } from "@/lib/storage";
import { useElementWidth, useOffsetWithin } from "@/lib/use-element-size";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { musicApi, type AlbumSort, type MusicAlbum, type Order } from "../api";
import { AlbumCard, AlbumListRow } from "../components/album-card";
import { MusicLibraryGuard, MusicLibrarySwitcher } from "../library-context";
import { musicKeys } from "../queries";
import { observeVirtualRect } from "../virtual";

export interface AlbumsSearchParams {
  sort?: AlbumSort;
  order?: Order;
  genre?: string;
  yearFrom?: number;
  yearTo?: number;
  favorite?: boolean;
  q?: string;
}

const VIEW_KEY = "palveni.music.albums.view";
const SORT_OPTIONS: AlbumSort[] = ["title", "artist", "year", "added", "played", "plays", "rating", "random"];
const DECADES = [2020, 2010, 2000, 1990, 1980] as const;

function VirtualAlbumGrid({
  albums,
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
}: {
  albums: MusicAlbum[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
}) {
  const scrollRef = useScrollContainer();
  const containerRef = useRef<HTMLDivElement>(null);
  const measuredWidth = useElementWidth(containerRef);
  const width = measuredWidth > 0 ? measuredWidth : 1024;
  const scrollMargin = useOffsetWithin(containerRef, scrollRef);

  const gap = width < 640 ? 14 : 20;
  const minTile = width < 480 ? 150 : 176;
  const columns = Math.max(2, Math.floor((width + gap) / (minTile + gap)));
  const tileWidth = (width - gap * (columns - 1)) / columns;
  const rowHeight = Math.ceil(tileWidth + 56 + gap);
  const rowCount = Math.ceil(albums.length / columns);

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 3,
    scrollMargin,
    observeElementRect: observeVirtualRect,
  });

  useEffect(() => {
    virtualizer.measure();
  }, [virtualizer, rowHeight, columns]);

  const virtualRows = virtualizer.getVirtualItems();
  const lastIndex = virtualRows.at(-1)?.index;
  useEffect(() => {
    if (lastIndex !== undefined && lastIndex >= rowCount - 2 && hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [lastIndex, rowCount, hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <div ref={containerRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
      {virtualRows.map((row) => {
        const start = row.index * columns;
        const rowItems = albums.slice(start, start + columns);
        return (
          <div
            key={row.key}
            className="absolute inset-x-0 top-0 grid"
            style={{
              transform: `translateY(${row.start - scrollMargin}px)`,
              gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
              columnGap: gap,
            }}
          >
            {rowItems.map((album) => (
              <AlbumCard key={album.id} album={album} />
            ))}
          </div>
        );
      })}
    </div>
  );
}

function VirtualAlbumList({
  albums,
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
}: {
  albums: MusicAlbum[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
}) {
  const { t } = useTranslation();
  const scrollRef = useScrollContainer();
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollMargin = useOffsetWithin(containerRef, scrollRef);

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: albums.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 58,
    overscan: 8,
    scrollMargin,
    observeElementRect: observeVirtualRect,
  });

  const virtualRows = virtualizer.getVirtualItems();
  const lastIndex = virtualRows.at(-1)?.index;
  useEffect(() => {
    if (lastIndex !== undefined && lastIndex >= albums.length - 10 && hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [lastIndex, albums.length, hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <div className="overflow-hidden rounded-panel border border-line bg-surface">
      <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 border-b border-line px-3 py-2 text-xs font-mid text-fg-muted sm:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_4.5rem_5rem_4.5rem]">
        <span>{t("musicApp.albums.title")}</span>
        <span className="max-sm:hidden">{t("musicApp.songs.columns.artist")}</span>
        <span className="text-end max-sm:hidden">{t("musicApp.songs.columns.year")}</span>
        <span className="text-end max-sm:hidden">{t("musicApp.songs.columns.duration")}</span>
        <span className="sr-only">{t("musicApp.actions.more")}</span>
      </div>
      <div ref={containerRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
        {virtualRows.map((row) => {
          const album = albums[row.index]!;
          return (
            <div
              key={row.key}
              className="absolute inset-x-0 top-0 border-b border-line last:border-b-0"
              style={{ height: 58, transform: `translateY(${row.start - scrollMargin}px)` }}
            >
              <AlbumListRow album={album} />
            </div>
          );
        })}
      </div>
    </div>
  );
}

function AlbumsContent({ libraryId, search }: { libraryId: string; search: AlbumsSearchParams }) {
  const { t } = useTranslation();
  const navigate = useNavigate({ from: "/music/albums/" });
  const [view, setView] = useState<"grid" | "list">(() => (readLocal(VIEW_KEY) === "list" ? "list" : "grid"));
  const [seed, setSeed] = useState(1);
  const [draft, setDraft] = useState<{ base: string; value: string }>(() => ({ base: search.q ?? "", value: search.q ?? "" }));
  const queryInput = draft.base === (search.q ?? "") ? draft.value : (search.q ?? "");
  const setQueryInput = (value: string) => setDraft({ base: search.q ?? "", value });

  const sort: AlbumSort = search.sort ?? "title";
  const defaultDesc = sort === "added" || sort === "played" || sort === "plays" || sort === "rating";
  const order: Order = search.order ?? (defaultDesc ? "desc" : "asc");

  const updateSearch = (patch: Partial<AlbumsSearchParams>) => {
    void navigate({
      search: (prev) => {
        const next = { ...prev, ...patch };
        for (const [k, v] of Object.entries(next)) {
          if (v === undefined || v === "" || v === false) delete (next as Record<string, unknown>)[k];
        }
        return next;
      },
      replace: true,
    });
  };

  useEffect(() => {
    const timer = setTimeout(() => {
      const trimmed = queryInput.trim() || undefined;
      if (trimmed !== (search.q || undefined)) {
        updateSearch({ q: trimmed });
      }
    }, 220);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryInput]);

  const genresQuery = useQuery({
    queryKey: musicKeys.genres(libraryId),
    queryFn: ({ signal }) => musicApi.genres(libraryId, signal),
    staleTime: 60_000,
  });

  const listParams = {
    sort,
    order: sort === "random" ? undefined : order,
    genre: search.genre,
    yearFrom: search.yearFrom,
    yearTo: search.yearTo,
    favorite: search.favorite ? true : undefined,
    q: search.q,
    seed: sort === "random" ? seed : undefined,
    limit: 60,
  };

  const albumsQuery = useInfiniteQuery({
    queryKey: musicKeys.albums(libraryId, listParams),
    queryFn: ({ pageParam, signal }) => musicApi.albums(libraryId, { ...listParams, cursor: pageParam }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const albums = albumsQuery.data?.pages.flatMap((page) => page.albums) ?? [];
  const total = albumsQuery.data?.pages[0]?.total;
  const hasFilters = Boolean(search.genre || search.yearFrom || search.yearTo || search.favorite || search.q);

  const activeDecade =
    search.yearFrom && search.yearTo && search.yearTo === search.yearFrom + 9 ? search.yearFrom : null;

  return (
    <div className="grid gap-4">
      {/* Filter & sort bar */}
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          value={queryInput}
          onChange={(event) => setQueryInput(event.target.value)}
          aria-label={t("musicApp.albums.searchPlaceholder")}
          placeholder={t("musicApp.albums.searchPlaceholder")}
          className="h-(--pv-control-h-sm) w-52 max-sm:w-full"
        />

        {/* Genre filter */}
        <Dropdown>
          <Dropdown.Trigger
            aria-label={t("musicApp.genres.title")}
            className={cn(
              "inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border px-2.5 text-sm font-mid outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus",
              search.genre ? "border-selected bg-selected/10 text-fg" : "border-line-control bg-surface text-fg-muted",
            )}
          >
            <span className="max-w-28 truncate">{search.genre ?? t("musicApp.albums.allGenres")}</span>
            <ChevronDown className="size-3.5" aria-hidden />
          </Dropdown.Trigger>
          <Dropdown.Popover placement="bottom start" className="max-h-72 min-w-44 overflow-y-auto">
            <Dropdown.Menu
              aria-label={t("musicApp.genres.title")}
              onAction={(key) => {
                const value = String(key);
                updateSearch({ genre: value === "__all__" ? undefined : value });
              }}
            >
              <Dropdown.Item id="__all__" textValue={t("musicApp.albums.allGenres")}>
                <Label className="flex-1">{t("musicApp.albums.allGenres")}</Label>
                {!search.genre ? <Check className="size-3.5" aria-hidden /> : null}
              </Dropdown.Item>
              {(genresQuery.data?.genres ?? []).map((genre) => (
                <Dropdown.Item key={genre.name} id={genre.name} textValue={genre.name}>
                  <Label className="flex-1 truncate">{genre.name}</Label>
                  <span data-num className="text-xs text-fg-muted tabular-nums">{genre.albumCount}</span>
                  {search.genre === genre.name ? <Check className="size-3.5" aria-hidden /> : null}
                </Dropdown.Item>
              ))}
            </Dropdown.Menu>
          </Dropdown.Popover>
        </Dropdown>

        {/* Decade filter */}
        <Dropdown>
          <Dropdown.Trigger
            aria-label={t("musicApp.albums.allYears")}
            className={cn(
              "inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border px-2.5 text-sm font-mid outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus",
              activeDecade ? "border-selected bg-selected/10 text-fg" : "border-line-control bg-surface text-fg-muted",
            )}
          >
            <span>{activeDecade ? t("musicApp.albums.decade", { year: activeDecade }) : t("musicApp.albums.allYears")}</span>
            <ChevronDown className="size-3.5" aria-hidden />
          </Dropdown.Trigger>
          <Dropdown.Popover placement="bottom start" className="min-w-36">
            <Dropdown.Menu
              aria-label={t("musicApp.albums.allYears")}
              onAction={(key) => {
                const value = String(key);
                if (value === "__all__") updateSearch({ yearFrom: undefined, yearTo: undefined });
                else {
                  const decade = Number(value);
                  updateSearch({ yearFrom: decade, yearTo: decade + 9 });
                }
              }}
            >
              <Dropdown.Item id="__all__" textValue={t("musicApp.albums.allYears")}>
                <Label className="flex-1">{t("musicApp.albums.allYears")}</Label>
                {!activeDecade ? <Check className="size-3.5" aria-hidden /> : null}
              </Dropdown.Item>
              {DECADES.map((decade) => (
                <Dropdown.Item key={decade} id={String(decade)} textValue={t("musicApp.albums.decade", { year: decade })}>
                  <Label className="flex-1">{t("musicApp.albums.decade", { year: decade })}</Label>
                  {activeDecade === decade ? <Check className="size-3.5" aria-hidden /> : null}
                </Dropdown.Item>
              ))}
            </Dropdown.Menu>
          </Dropdown.Popover>
        </Dropdown>

        {/* Favorites toggle */}
        <ToggleButton
          size="sm"
          isSelected={Boolean(search.favorite)}
          onChange={(on) => updateSearch({ favorite: on ? true : undefined })}
          className="rounded-control"
        >
          <Heart className={cn("size-3.5", search.favorite && "fill-failed text-failed")} aria-hidden />
          {t("musicApp.albums.favoritesOnly")}
        </ToggleButton>

        {hasFilters ? (
          <Button
            size="sm"
            variant="ghost"
            onPress={() => {
              setQueryInput("");
              updateSearch({ genre: undefined, yearFrom: undefined, yearTo: undefined, favorite: undefined, q: undefined });
            }}
          >
            <X className="size-3.5" aria-hidden />
            {t("musicApp.albums.clearFilters")}
          </Button>
        ) : null}

        <span className="flex-1" />

        {total !== undefined ? (
          <span data-num className="text-xs text-fg-muted tabular-nums max-sm:hidden">
            {t("musicApp.albums.count", { count: total })}
          </span>
        ) : null}

        {/* Sort */}
        <div className="flex items-center gap-1">
          <Dropdown>
            <Dropdown.Trigger
              aria-label={t("musicApp.albums.sort.label")}
              className="inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border border-line-control bg-surface px-2.5 text-sm font-mid text-fg outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
            >
              <span>{t(`musicApp.albums.sort.${sort}`)}</span>
              <ChevronDown className="size-3.5 text-fg-muted" aria-hidden />
            </Dropdown.Trigger>
            <Dropdown.Popover placement="bottom end" className="min-w-40">
              <Dropdown.Menu
                aria-label={t("musicApp.albums.sort.label")}
                onAction={(key) => {
                  const nextSort = String(key) as AlbumSort;
                  const nextDefaultDesc = nextSort === "added" || nextSort === "played" || nextSort === "plays" || nextSort === "rating";
                  updateSearch({ sort: nextSort, order: nextSort === "random" ? undefined : nextDefaultDesc ? "desc" : "asc" });
                }}
              >
                {SORT_OPTIONS.map((opt) => (
                  <Dropdown.Item key={opt} id={opt} textValue={t(`musicApp.albums.sort.${opt}`)}>
                    <Label className="flex-1">{t(`musicApp.albums.sort.${opt}`)}</Label>
                    {sort === opt ? <Check className="size-3.5" aria-hidden /> : null}
                  </Dropdown.Item>
                ))}
              </Dropdown.Menu>
            </Dropdown.Popover>
          </Dropdown>

          {sort === "random" ? (
            <Button size="sm" variant="outline" isIconOnly aria-label={t("musicApp.home.refreshRandom")} onPress={() => setSeed((s) => s + 1)}>
              <RefreshCw className="size-3.5" aria-hidden />
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              isIconOnly
              aria-label={order === "asc" ? t("musicApp.actions.ascending") : t("musicApp.actions.descending")}
              onPress={() => updateSearch({ order: order === "asc" ? "desc" : "asc" })}
            >
              {order === "asc" ? <ArrowUp className="size-3.5" aria-hidden /> : <ArrowDown className="size-3.5" aria-hidden />}
            </Button>
          )}
        </div>

        {/* Grid / List toggle */}
        <ToggleButtonGroup
          aria-label={t("browse.view")}
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={[view]}
          onSelectionChange={(keys) => {
            const next = [...keys][0];
            if (next === "grid" || next === "list") {
              setView(next);
              writeLocal(VIEW_KEY, next);
            }
          }}
          className="pv-segmented"
        >
          <ToggleButton id="grid" aria-label={t("musicApp.actions.viewGrid")}>
            <LayoutGrid aria-hidden />
          </ToggleButton>
          <ToggleButton id="list" aria-label={t("musicApp.actions.viewList")}>
            <List aria-hidden />
          </ToggleButton>
        </ToggleButtonGroup>

        <MusicLibrarySwitcher />
      </div>

      {/* Results */}
      {albumsQuery.isPending ? (
        <LoadingRows rows={6} />
      ) : albumsQuery.isError && albums.length === 0 ? (
        <ErrorState error={albumsQuery.error} onRetry={() => void albumsQuery.refetch()} />
      ) : albums.length === 0 ? (
        <EmptyState icon={Disc3} title={t("musicApp.albums.empty")} />
      ) : view === "grid" ? (
        <VirtualAlbumGrid
          albums={albums}
          hasNextPage={albumsQuery.hasNextPage}
          isFetchingNextPage={albumsQuery.isFetchingNextPage}
          fetchNextPage={() => void albumsQuery.fetchNextPage()}
        />
      ) : (
        <VirtualAlbumList
          albums={albums}
          hasNextPage={albumsQuery.hasNextPage}
          isFetchingNextPage={albumsQuery.isFetchingNextPage}
          fetchNextPage={() => void albumsQuery.fetchNextPage()}
        />
      )}

      {albumsQuery.isFetchingNextPage ? (
        <div role="status" className="flex justify-center py-3">
          <Spinner size="sm" />
        </div>
      ) : null}
    </div>
  );
}

export function MusicAlbumsPage() {
  const { t } = useTranslation();
  const search = useSearch({ from: "/_app/music/albums/" });
  return (
    <PageFrame>
      <PageHeader title={t("musicApp.albums.title")} />
      <MusicLibraryGuard>{(library) => <AlbumsContent libraryId={library.id} search={search} />}</MusicLibraryGuard>
    </PageFrame>
  );
}
