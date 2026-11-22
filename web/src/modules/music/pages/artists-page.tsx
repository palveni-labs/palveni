import { Button, Dropdown, Input, Label, Spinner, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowDown, ArrowUp, Check, ChevronDown, Heart, LayoutGrid, List, RefreshCw, UsersRound } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useScrollContainer } from "@/lib/scroll-container";
import { readLocal, writeLocal } from "@/lib/storage";
import { useElementWidth, useOffsetWithin } from "@/lib/use-element-size";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { musicApi, type ArtistSort, type MusicArtist, type Order } from "../api";
import { ArtistCard, ArtistListRow } from "../components/artist-card";
import { MusicLibraryGuard, MusicLibrarySwitcher } from "../library-context";
import { musicKeys } from "../queries";
import { observeVirtualRect } from "../virtual";

export interface ArtistsSearchParams {
  sort?: ArtistSort;
  order?: Order;
  favorite?: boolean;
  q?: string;
}

const VIEW_KEY = "palveni.music.artists.view";
const SORT_OPTIONS: ArtistSort[] = ["name", "albums", "tracks", "plays", "random"];

function VirtualArtistGrid({
  artists,
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
}: {
  artists: MusicArtist[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
}) {
  const scrollRef = useScrollContainer();
  const containerRef = useRef<HTMLDivElement>(null);
  const measuredWidth = useElementWidth(containerRef);
  const width = measuredWidth > 0 ? measuredWidth : 1024;
  const scrollMargin = useOffsetWithin(containerRef, scrollRef);

  const gap = width < 640 ? 16 : 24;
  const minTile = width < 480 ? 140 : 164;
  const columns = Math.max(2, Math.floor((width + gap) / (minTile + gap)));
  const tileWidth = (width - gap * (columns - 1)) / columns;
  const rowHeight = Math.ceil(tileWidth + 56 + gap);
  const rowCount = Math.ceil(artists.length / columns);

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
        const rowItems = artists.slice(start, start + columns);
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
            {rowItems.map((artist) => (
              <ArtistCard key={artist.id} artist={artist} />
            ))}
          </div>
        );
      })}
    </div>
  );
}

function VirtualArtistList({
  artists,
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
}: {
  artists: MusicArtist[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
}) {
  const scrollRef = useScrollContainer();
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollMargin = useOffsetWithin(containerRef, scrollRef);

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: artists.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 58,
    overscan: 8,
    scrollMargin,
    observeElementRect: observeVirtualRect,
  });

  const virtualRows = virtualizer.getVirtualItems();
  const lastIndex = virtualRows.at(-1)?.index;
  useEffect(() => {
    if (lastIndex !== undefined && lastIndex >= artists.length - 10 && hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [lastIndex, artists.length, hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <div className="overflow-hidden rounded-panel border border-line bg-surface">
      <div ref={containerRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
        {virtualRows.map((row) => {
          const artist = artists[row.index]!;
          return (
            <div
              key={row.key}
              className="absolute inset-x-0 top-0 border-b border-line last:border-b-0"
              style={{ height: 58, transform: `translateY(${row.start - scrollMargin}px)` }}
            >
              <ArtistListRow artist={artist} />
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ArtistsContent({ libraryId, search }: { libraryId: string; search: ArtistsSearchParams }) {
  const { t } = useTranslation();
  const navigate = useNavigate({ from: "/music/artists/" });
  const [view, setView] = useState<"grid" | "list">(() => (readLocal(VIEW_KEY) === "list" ? "list" : "grid"));
  const [seed, setSeed] = useState(1);
  const [draft, setDraft] = useState<{ base: string; value: string }>(() => ({ base: search.q ?? "", value: search.q ?? "" }));
  const queryInput = draft.base === (search.q ?? "") ? draft.value : (search.q ?? "");
  const setQueryInput = (value: string) => setDraft({ base: search.q ?? "", value });

  const sort: ArtistSort = search.sort ?? "name";
  const defaultDesc = sort !== "name";
  const order: Order = search.order ?? (defaultDesc ? "desc" : "asc");

  const updateSearch = (patch: Partial<ArtistsSearchParams>) => {
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
      if (trimmed !== (search.q || undefined)) updateSearch({ q: trimmed });
    }, 220);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryInput]);

  const params = {
    sort,
    order: sort === "random" ? undefined : order,
    favorite: search.favorite ? true : undefined,
    q: search.q,
    seed: sort === "random" ? seed : undefined,
    limit: 60,
  };

  const artistsQuery = useInfiniteQuery({
    queryKey: musicKeys.artists(libraryId, params),
    queryFn: ({ pageParam, signal }) => musicApi.artists(libraryId, { ...params, cursor: pageParam }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const artists = artistsQuery.data?.pages.flatMap((page) => page.artists) ?? [];
  const total = artistsQuery.data?.pages[0]?.total;

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          value={queryInput}
          onChange={(event) => setQueryInput(event.target.value)}
          aria-label={t("musicApp.artists.searchPlaceholder")}
          placeholder={t("musicApp.artists.searchPlaceholder")}
          className="h-(--pv-control-h-sm) w-52 max-sm:w-full"
        />

        <ToggleButton
          size="sm"
          isSelected={Boolean(search.favorite)}
          onChange={(on) => updateSearch({ favorite: on ? true : undefined })}
          className="rounded-control"
        >
          <Heart className={cn("size-3.5", search.favorite && "fill-failed text-failed")} aria-hidden />
          {t("musicApp.albums.favoritesOnly")}
        </ToggleButton>

        <span className="flex-1" />

        {total !== undefined ? (
          <span data-num className="text-xs text-fg-muted tabular-nums max-sm:hidden">
            {t("musicApp.artists.count", { count: total })}
          </span>
        ) : null}

        <div className="flex items-center gap-1">
          <Dropdown>
            <Dropdown.Trigger
              aria-label={t("musicApp.artists.sort.label")}
              className="inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border border-line-control bg-surface px-2.5 text-sm font-mid text-fg outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
            >
              <span>{t(`musicApp.artists.sort.${sort}`)}</span>
              <ChevronDown className="size-3.5 text-fg-muted" aria-hidden />
            </Dropdown.Trigger>
            <Dropdown.Popover placement="bottom end" className="min-w-40">
              <Dropdown.Menu
                aria-label={t("musicApp.artists.sort.label")}
                onAction={(key) => {
                  const nextSort = String(key) as ArtistSort;
                  updateSearch({ sort: nextSort, order: nextSort === "random" ? undefined : nextSort === "name" ? "asc" : "desc" });
                }}
              >
                {SORT_OPTIONS.map((opt) => (
                  <Dropdown.Item key={opt} id={opt} textValue={t(`musicApp.artists.sort.${opt}`)}>
                    <Label className="flex-1">{t(`musicApp.artists.sort.${opt}`)}</Label>
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

      {artistsQuery.isPending ? (
        <LoadingRows rows={6} />
      ) : artistsQuery.isError && artists.length === 0 ? (
        <ErrorState error={artistsQuery.error} onRetry={() => void artistsQuery.refetch()} />
      ) : artists.length === 0 ? (
        <EmptyState icon={UsersRound} title={t("musicApp.artists.empty")} />
      ) : view === "grid" ? (
        <VirtualArtistGrid
          artists={artists}
          hasNextPage={artistsQuery.hasNextPage}
          isFetchingNextPage={artistsQuery.isFetchingNextPage}
          fetchNextPage={() => void artistsQuery.fetchNextPage()}
        />
      ) : (
        <VirtualArtistList
          artists={artists}
          hasNextPage={artistsQuery.hasNextPage}
          isFetchingNextPage={artistsQuery.isFetchingNextPage}
          fetchNextPage={() => void artistsQuery.fetchNextPage()}
        />
      )}

      {artistsQuery.isFetchingNextPage ? (
        <div role="status" className="flex justify-center py-3">
          <Spinner size="sm" />
        </div>
      ) : null}
    </div>
  );
}

export function MusicArtistsPage() {
  const { t } = useTranslation();
  const search = useSearch({ from: "/_app/music/artists/" });
  return (
    <PageFrame>
      <PageHeader title={t("musicApp.artists.title")} />
      <MusicLibraryGuard>{(library) => <ArtistsContent libraryId={library.id} search={search} />}</MusicLibraryGuard>
    </PageFrame>
  );
}
