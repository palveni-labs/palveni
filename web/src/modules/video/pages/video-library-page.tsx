import { Button, Spinner, buttonVariants } from "@heroui/react";
import { Link, useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowDown,
  ArrowUp,
  Film,
  FolderOpen,
  Heart,
  Search,
  Shuffle,
  SlidersHorizontal,
  Tag,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLibraries, useLibrary } from "@/api/queries/libraries";
import { cn } from "@/lib/cn";
import { useScrollContainer } from "@/lib/scroll-container";
import { useElementWidth, useOffsetWithin } from "@/lib/use-element-size";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import type { VideoEntry, VideoListQuery, VideoOrder } from "../api";
import { PosterCard } from "../components/poster-card";
import { useVideoGenres, useVideoList } from "../queries";

export type VideoLibraryTab = "all" | "movie" | "series" | "genres";

export interface VideoLibrarySearch {
  tab?: VideoLibraryTab;
  order?: VideoOrder;
  dir?: "asc" | "desc";
  q?: string;
  genre?: string;
  yearFrom?: number;
  yearTo?: number;
  played?: "played" | "unplayed";
  favorite?: boolean;
  startsWith?: string;
  seed?: number;
}

const ALPHABET = ["#", ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ"] as const;
const SORT_ORDERS: VideoOrder[] = ["added", "title", "year", "rating", "runtime", "poster", "random"];

function posterGridMetrics(width: number) {
  const gap = width < 640 ? 12 : 16;
  const columns =
    width < 440 ? 2 : width < 640 ? 3 : width < 860 ? 4 : width < 1120 ? 5 : width < 1360 ? 6 : 7;
  const cellWidth = Math.max(120, (width - gap * (columns - 1)) / columns);
  const posterHeight = Math.round(cellWidth * 1.5);
  const footerHeight = 52;
  return { columns, gap, rowHeight: posterHeight + footerHeight + gap };
}

function VirtualPosterGrid({
  entries,
  hasNextPage,
  isFetchingNextPage,
  onLoadMore,
}: {
  entries: VideoEntry[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onLoadMore: () => void;
}) {
  const scroll = useScrollContainer();
  const container = useRef<HTMLDivElement>(null);
  const width = useElementWidth(container);
  const scrollMargin = useOffsetWithin(container, scroll);
  const effectiveWidth = width > 0 ? width : 960;
  const { columns, gap, rowHeight } = posterGridMetrics(effectiveWidth);
  const rowCount = Math.ceil(entries.length / columns);

  // eslint-disable-next-line react-hooks/incompatible-library -- renders only visible virtual rows
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scroll.current,
    estimateSize: () => rowHeight,
    scrollMargin,
    overscan: 2,
  });

  useEffect(() => {
    virtualizer.measure();
  }, [virtualizer, rowHeight, columns]);

  const virtualRows = virtualizer.getVirtualItems();
  const lastRowIndex = virtualRows.at(-1)?.index ?? -1;

  useEffect(() => {
    if (hasNextPage && !isFetchingNextPage && lastRowIndex >= rowCount - 2 && rowCount > 0) {
      onLoadMore();
    }
  }, [hasNextPage, isFetchingNextPage, lastRowIndex, rowCount, onLoadMore]);

  return (
    <div
      ref={container}
      role="list"
      className="relative w-full"
      style={{ height: virtualizer.getTotalSize() }}
    >
      {virtualRows.map((row) => (
        <div
          key={row.key}
          role="presentation"
          className="absolute inset-x-0 top-0 grid"
          style={{
            gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
            gap,
            transform: `translateY(${row.start - scrollMargin}px)`,
          }}
        >
          {entries.slice(row.index * columns, (row.index + 1) * columns).map((entry) => (
            <div key={entry.id} role="listitem" className="min-w-0">
              <PosterCard entry={entry} />
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

function AlphabetBar({
  active,
  onSelect,
  orientation,
}: {
  active: string | undefined;
  onSelect: (letter: string | undefined) => void;
  orientation: "vertical" | "horizontal";
}) {
  const { t } = useTranslation();
  if (orientation === "horizontal") {
    return (
      <nav
        aria-label={t("videoApp.library.alphabet")}
        className="no-scrollbar -mx-1 flex items-center gap-0.5 overflow-x-auto px-1 py-0.5 md:hidden"
      >
        <button
          type="button"
          onClick={() => onSelect(undefined)}
          aria-pressed={!active}
          className={cn(
            "shrink-0 rounded-control px-2 py-1 text-xs font-mid transition-colors",
            !active ? "bg-action text-action-fg font-strong" : "text-fg-muted hover:bg-hover hover:text-fg",
          )}
        >
          {t("videoApp.library.alphabetAll")}
        </button>
        {ALPHABET.map((letter) => {
          const selected = active === letter;
          return (
            <button
              key={letter}
              type="button"
              onClick={() => onSelect(selected ? undefined : letter)}
              aria-pressed={selected}
              className={cn(
                "grid size-7 shrink-0 place-items-center rounded-control text-xs font-mid transition-colors",
                selected ? "bg-action text-action-fg font-strong" : "text-fg-muted hover:bg-hover hover:text-fg",
              )}
            >
              {letter}
            </button>
          );
        })}
      </nav>
    );
  }

  return (
    <nav
      aria-label={t("videoApp.library.alphabet")}
      className="sticky top-4 hidden max-h-[calc(100svh-8rem)] shrink-0 flex-col items-center gap-0.5 overflow-y-auto rounded-tile border border-line bg-surface/85 px-1 py-1.5 backdrop-blur-xs md:flex"
    >
      {ALPHABET.map((letter) => {
        const selected = active === letter;
        return (
          <button
            key={letter}
            type="button"
            onClick={() => onSelect(selected ? undefined : letter)}
            aria-pressed={selected}
            className={cn(
              "grid size-6 place-items-center rounded-control text-[0.7rem] font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              selected ? "bg-action font-strong text-action-fg" : "text-fg-muted hover:bg-hover hover:text-fg",
            )}
          >
            {letter}
          </button>
        );
      })}
    </nav>
  );
}

export function VideoLibraryPage() {
  const { t } = useTranslation();
  const { libraryId } = useParams({ from: "/_app/video/$libraryId/" });
  const search = useSearch({ from: "/_app/video/$libraryId/" });
  const navigate = useNavigate({ from: "/video/$libraryId/" });
  const libraryQuery = useLibrary(libraryId);
  const librariesQuery = useLibraries();
  const videoLibraries = useMemo(
    () => (librariesQuery.data?.libraries ?? []).filter((l) => l.kind === "video"),
    [librariesQuery.data],
  );

  const tab: VideoLibraryTab = search.tab ?? "all";
  const order: VideoOrder = search.order ?? "added";
  const dir: "asc" | "desc" =
    search.dir ?? (order === "title" || order === "poster" ? "asc" : "desc");
  const [showYearFilter, setShowYearFilter] = useState(Boolean(search.yearFrom || search.yearTo));

  const updateSearch = (patch: Partial<VideoLibrarySearch>) => {
    void navigate({
      search: (prev) => {
        const next: VideoLibrarySearch = { ...prev, ...patch };
        if (next.tab === "all") delete next.tab;
        if (!next.q) delete next.q;
        if (!next.genre) delete next.genre;
        if (!next.played) delete next.played;
        if (!next.favorite) delete next.favorite;
        if (!next.startsWith) delete next.startsWith;
        if (!next.yearFrom) delete next.yearFrom;
        if (!next.yearTo) delete next.yearTo;
        if (next.order !== "random") delete next.seed;
        return next;
      },
      replace: true,
    });
  };

  const listQueryParams: VideoListQuery = useMemo(
    () => ({
      kind: tab === "movie" || tab === "series" ? tab : undefined,
      q: search.q,
      genre: search.genre,
      yearFrom: search.yearFrom,
      yearTo: search.yearTo,
      played: search.played === "played" ? true : search.played === "unplayed" ? false : undefined,
      favorite: search.favorite ? true : undefined,
      order,
      dir,
      seed: order === "random" ? (search.seed ?? 42) : undefined,
      startsWith: search.startsWith,
    }),
    [tab, search.q, search.genre, search.yearFrom, search.yearTo, search.played, search.favorite, order, dir, search.seed, search.startsWith],
  );

  const listQuery = useVideoList(libraryId, listQueryParams, tab !== "genres");
  const genresQuery = useVideoGenres(libraryId);

  const entries = useMemo(
    () => listQuery.data?.pages.flatMap((page) => page.entries) ?? [],
    [listQuery.data],
  );
  const total = listQuery.data?.pages[0]?.total ?? entries.length;
  const genres = genresQuery.data?.genres ?? [];

  const hasFilters = Boolean(
    search.q || search.genre || search.played || search.favorite || search.startsWith || search.yearFrom || search.yearTo,
  );

  const library = libraryQuery.data;

  const headerActions = (
    <div className="flex items-center gap-2">
      {videoLibraries.length > 1 ? (
        <select
          aria-label={t("videoApp.home.libraries")}
          value={libraryId}
          onChange={(e) => {
            void navigate({ to: "/video/$libraryId", params: { libraryId: e.target.value }, search: {} });
          }}
          className="h-8 rounded-control border border-line bg-surface px-2.5 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        >
          {videoLibraries.map((lib) => (
            <option key={lib.id} value={lib.id}>
              {lib.name}
            </option>
          ))}
        </select>
      ) : null}
      <Link
        to="/library/$libraryId"
        params={{ libraryId }}
        search={{ view: "files" }}
        className={buttonVariants({ variant: "outline", size: "sm" })}
      >
        <FolderOpen className="size-4" aria-hidden />
        {t("videoApp.library.filesView")}
      </Link>
    </div>
  );

  return (
    <div className="pv-enter mx-auto flex w-full max-w-[88rem] flex-col gap-5 px-(--pv-gutter) pt-5 pb-14 max-md:px-4 max-md:pt-4">
      <PageHeader
        title={library?.name ?? t("videoApp.title")}
        summary={
          tab !== "genres" && listQuery.isSuccess
            ? t("videoApp.library.count", { count: total })
            : undefined
        }
        actions={headerActions}
      />

      {/* Tabs + search + sort/filter controls */}
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <nav aria-label={t("videoApp.library.tabs.all")} className="pv-segmented w-fit max-w-full">
            {(["all", "movie", "series", "genres"] as const).map((key) => {
              const current = tab === key;
              return (
                <button
                  key={key}
                  type="button"
                  aria-current={current ? "page" : undefined}
                  onClick={() => updateSearch({ tab: key })}
                  className={cn(
                    "flex h-[calc(var(--pv-control-h)-4px)] items-center rounded-control px-3 text-sm transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                    current ? "bg-raised font-strong text-fg shadow-raised" : "font-mid text-fg-muted hover:text-fg",
                  )}
                >
                  {t(`videoApp.library.tabs.${key}`)}
                </button>
              );
            })}
          </nav>

          {tab !== "genres" ? (
            <form
              role="search"
              className="relative flex min-w-[12rem] flex-1 items-center sm:max-w-xs"
              onSubmit={(e) => {
                e.preventDefault();
                const raw = new FormData(e.currentTarget).get("q");
                const trimmed = typeof raw === "string" ? raw.trim() : "";
                updateSearch({ q: trimmed || undefined });
              }}
            >
              <Search className="pointer-events-none absolute left-2.5 size-4 text-fg-subtle" aria-hidden />
              <input
                key={search.q ?? ""}
                type="search"
                name="q"
                defaultValue={search.q ?? ""}
                onChange={(e) => {
                  if (!e.target.value.trim() && search.q) updateSearch({ q: undefined });
                }}
                aria-label={t("videoApp.library.searchPlaceholder")}
                placeholder={t("videoApp.library.searchPlaceholder")}
                className="h-(--pv-control-h) w-full rounded-field border border-line bg-surface ps-8 pe-3 text-sm text-fg placeholder:text-fg-subtle focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              />
            </form>
          ) : null}

          {tab !== "genres" ? (
            <div className="ms-auto flex flex-wrap items-center gap-2 max-sm:ms-0 max-sm:w-full">
              {/* Sort selector + direction */}
              <div className="flex items-center gap-1">
                <select
                  aria-label={t("videoApp.library.sortLabel")}
                  value={order}
                  onChange={(e) => {
                    const nextOrder = e.target.value as VideoOrder;
                    updateSearch({
                      order: nextOrder,
                      dir: nextOrder === "title" || nextOrder === "poster" ? "asc" : "desc",
                      seed: nextOrder === "random" ? Date.now() % 10_000 : undefined,
                    });
                  }}
                  className="h-(--pv-control-h-sm) rounded-control border border-line bg-surface px-2.5 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                >
                  {SORT_ORDERS.map((o) => (
                    <option key={o} value={o}>
                      {t(`videoApp.library.sort.${o}`)}
                    </option>
                  ))}
                </select>
                {order === "random" ? (
                  <Button
                    size="sm"
                    variant="outline"
                    isIconOnly
                    aria-label={t("videoApp.library.reshuffle")}
                    onPress={() => updateSearch({ order: "random", seed: Math.floor(Math.random() * 100_000) + 1 })}
                  >
                    <Shuffle className="size-3.5" aria-hidden />
                  </Button>
                ) : (
                  <Button
                    size="sm"
                    variant="outline"
                    isIconOnly
                    aria-label={t(dir === "asc" ? "videoApp.library.sortAsc" : "videoApp.library.sortDesc")}
                    onPress={() => updateSearch({ dir: dir === "asc" ? "desc" : "asc" })}
                  >
                    {dir === "asc" ? <ArrowUp className="size-3.5" aria-hidden /> : <ArrowDown className="size-3.5" aria-hidden />}
                  </Button>
                )}
              </div>
            </div>
          ) : null}
        </div>

        {/* Secondary filter strip */}
        {tab !== "genres" ? (
          <div className="flex flex-wrap items-center gap-2">
            {/* Played / Unplayed */}
            <div className="inline-flex items-center rounded-control border border-line bg-surface p-0.5">
              {([
                [undefined, "allStatus"],
                ["unplayed", "unplayed"],
                ["played", "played"],
              ] as const).map(([val, key]) => {
                const selected = search.played === val;
                return (
                  <button
                    key={key}
                    type="button"
                    aria-pressed={selected}
                    onClick={() => updateSearch({ played: val })}
                    className={cn(
                      "rounded-[calc(var(--pv-radius-control)-2px)] px-2.5 py-1 text-xs font-mid transition-colors",
                      selected ? "bg-action text-action-fg font-strong" : "text-fg-muted hover:text-fg",
                    )}
                  >
                    {t(`videoApp.library.filter.${key}`)}
                  </button>
                );
              })}
            </div>

            {/* Favorite toggle */}
            <button
              type="button"
              aria-pressed={Boolean(search.favorite)}
              onClick={() => updateSearch({ favorite: search.favorite ? undefined : true })}
              className={cn(
                "inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border px-2.5 text-xs font-mid transition-colors",
                search.favorite
                  ? "border-action bg-action text-action-fg font-strong"
                  : "border-line bg-surface text-fg-muted hover:bg-hover hover:text-fg",
              )}
            >
              <Heart className={cn("size-3.5", search.favorite && "fill-current")} aria-hidden />
              {t("videoApp.library.filter.favorite")}
            </button>

            {/* Genre select */}
            {genres.length > 0 ? (
              <select
                aria-label={t("videoApp.library.filter.allGenres")}
                value={search.genre ?? ""}
                onChange={(e) => updateSearch({ genre: e.target.value || undefined })}
                className="h-(--pv-control-h-sm) rounded-control border border-line bg-surface px-2.5 text-xs text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <option value="">{t("videoApp.library.filter.allGenres")}</option>
                {genres.map((g) => (
                  <option key={g.name} value={g.name}>
                    {g.name} ({g.count})
                  </option>
                ))}
              </select>
            ) : null}

            {/* Year filter toggle / inputs */}
            <button
              type="button"
              aria-expanded={showYearFilter}
              onClick={() => setShowYearFilter((v) => !v)}
              className={cn(
                "inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border px-2.5 text-xs font-mid transition-colors",
                search.yearFrom || search.yearTo
                  ? "border-action bg-action text-action-fg font-strong"
                  : "border-line bg-surface text-fg-muted hover:bg-hover hover:text-fg",
              )}
            >
              <SlidersHorizontal className="size-3.5" aria-hidden />
              {search.yearFrom || search.yearTo
                ? `${search.yearFrom ?? "…"}–${search.yearTo ?? "…"}`
                : t("videoApp.library.filter.year")}
            </button>

            {showYearFilter ? (
              <div className="flex items-center gap-1.5">
                <input
                  type="number"
                  min={1900}
                  max={2100}
                  placeholder={t("videoApp.library.filter.yearFrom")}
                  aria-label={t("videoApp.library.filter.yearFrom")}
                  value={search.yearFrom ?? ""}
                  onChange={(e) => {
                    const n = Number(e.target.value);
                    updateSearch({ yearFrom: Number.isInteger(n) && n >= 1900 ? n : undefined });
                  }}
                  className="h-(--pv-control-h-sm) w-20 rounded-field border border-line bg-surface px-2 text-xs text-fg tabular-nums"
                />
                <span className="text-xs text-fg-muted">–</span>
                <input
                  type="number"
                  min={1900}
                  max={2100}
                  placeholder={t("videoApp.library.filter.yearTo")}
                  aria-label={t("videoApp.library.filter.yearTo")}
                  value={search.yearTo ?? ""}
                  onChange={(e) => {
                    const n = Number(e.target.value);
                    updateSearch({ yearTo: Number.isInteger(n) && n >= 1900 ? n : undefined });
                  }}
                  className="h-(--pv-control-h-sm) w-20 rounded-field border border-line bg-surface px-2 text-xs text-fg tabular-nums"
                />
              </div>
            ) : null}

            {hasFilters ? (
              <button
                type="button"
                onClick={() =>
                  updateSearch({
                    q: undefined,
                    genre: undefined,
                    played: undefined,
                    favorite: undefined,
                    startsWith: undefined,
                    yearFrom: undefined,
                    yearTo: undefined,
                  })
                }
                className="inline-flex h-(--pv-control-h-sm) items-center gap-1 rounded-control px-2 text-xs font-mid text-fg-muted hover:bg-hover hover:text-fg"
              >
                <X className="size-3.5" aria-hidden />
                {t("videoApp.library.filter.clear")}
              </button>
            ) : null}
          </div>
        ) : null}
      </div>

      {/* Genres view */}
      {tab === "genres" ? (
        genresQuery.isPending ? (
          <LoadingRows rows={4} />
        ) : genresQuery.isError ? (
          <ErrorState error={genresQuery.error} onRetry={() => void genresQuery.refetch()} />
        ) : genres.length === 0 ? (
          <EmptyState icon={Tag} title={t("videoApp.library.emptyLibrary")} />
        ) : (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
            {genres.map((genre) => (
              <button
                key={genre.name}
                type="button"
                onClick={() => updateSearch({ tab: "all", genre: genre.name })}
                className="group flex flex-col items-start justify-between gap-4 rounded-tile border border-line bg-surface p-4 text-start shadow-panel transition-colors hover:border-line-strong hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <Tag className="size-5 text-fg-muted transition-colors group-hover:text-fg" aria-hidden />
                <div className="grid gap-0.5">
                  <span className="font-heading text-base font-title text-heading">{genre.name}</span>
                  <span data-num className="text-xs text-fg-muted tabular-nums">
                    {t("videoApp.library.genreCount", { count: genre.count })}
                  </span>
                </div>
              </button>
            ))}
          </div>
        )
      ) : (
        <>
          <AlphabetBar
            active={search.startsWith}
            onSelect={(letter) => updateSearch({ startsWith: letter })}
            orientation="horizontal"
          />

          <div className="flex items-start gap-4">
            <div className="min-w-0 flex-1">
              {listQuery.isPending ? (
                <LoadingRows rows={6} />
              ) : listQuery.isError && entries.length === 0 ? (
                <ErrorState error={listQuery.error} onRetry={() => void listQuery.refetch()} />
              ) : entries.length === 0 ? (
                <EmptyState
                  icon={Film}
                  title={hasFilters ? t("videoApp.library.emptyFiltered") : t("videoApp.library.emptyLibrary")}
                  action={
                    hasFilters ? (
                      <Button
                        size="sm"
                        variant="outline"
                        onPress={() =>
                          updateSearch({
                            q: undefined,
                            genre: undefined,
                            played: undefined,
                            favorite: undefined,
                            startsWith: undefined,
                            yearFrom: undefined,
                            yearTo: undefined,
                          })
                        }
                      >
                        {t("videoApp.library.filter.clear")}
                      </Button>
                    ) : null
                  }
                />
              ) : (
                <>
                  <VirtualPosterGrid
                    entries={entries}
                    hasNextPage={Boolean(listQuery.hasNextPage)}
                    isFetchingNextPage={listQuery.isFetchingNextPage}
                    onLoadMore={() => void listQuery.fetchNextPage()}
                  />
                  {listQuery.isFetchingNextPage ? (
                    <div className="mt-6 flex justify-center">
                      <Spinner size="sm" />
                    </div>
                  ) : listQuery.hasNextPage ? (
                    <div className="mt-6 flex justify-center">
                      <Button size="sm" variant="outline" onPress={() => void listQuery.fetchNextPage()}>
                        {t("videoApp.library.loadMore")}
                      </Button>
                    </div>
                  ) : null}
                </>
              )}
            </div>

            <AlphabetBar
              active={search.startsWith}
              onSelect={(letter) => updateSearch({ startsWith: letter })}
              orientation="vertical"
            />
          </div>
        </>
      )}
    </div>
  );
}
