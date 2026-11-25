import { Button } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ArrowDownAZ, ArrowUpAZ, BookOpen, FolderOpen, Heart, Layers, Search, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { FieldInput, Select } from "@/ui/controls";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import type { ReadingKind, ReadingListQuery, ReadingSort, ReadingSortDir } from "../api";
import { ReadingCard } from "../components/reading-card";
import { useReadingAcrossLibraries, useReadingLibraries, useReadingLibraryList } from "../queries";

const BOOK_FORMATS = ["epub", "pdf", "txt", "md", "mobi", "azw3", "fb2", "djvu"];
const COMIC_FORMATS = ["cbz", "zip", "cbt", "tar", "pdf", "cbr", "rar", "7z"];

export type ShelfMode = "books" | "comics" | "favorites" | "search";

export function ReadingShelfPage({
  mode,
  initialLibraryId,
  initialSeries,
  initialQuery,
  onQueryChange,
}: {
  mode: ShelfMode;
  initialLibraryId?: string;
  initialSeries?: string;
  initialQuery?: string;
  onQueryChange?: (q: string) => void;
}) {
  const { t } = useTranslation();
  const fixedKind: ReadingKind | undefined =
    mode === "books" ? "book" : mode === "comics" ? "comic" : undefined;
  const { libraries, isLoading: libsLoading } = useReadingLibraries(fixedKind);

  const [selectedLibrary, setSelectedLibrary] = useState<string>(initialLibraryId ?? "");
  const [kindFilter, setKindFilter] = useState<"" | ReadingKind>("");
  const [formatFilter, setFormatFilter] = useState<string>("");
  const [statusFilter, setStatusFilter] = useState<"all" | "reading" | "completed">("all");
  const [seriesFilter, setSeriesFilter] = useState<string>(initialSeries ?? "");
  const [sort, setSort] = useState<ReadingSort>(initialSeries ? "series" : "added");
  const [dir, setDir] = useState<ReadingSortDir>(initialSeries ? "asc" : "desc");
  const [searchInput, setSearchInput] = useState<string>(initialQuery ?? "");

  const effectiveLibrary = libraries.some((l) => l.id === selectedLibrary) ? selectedLibrary : "";
  const effectiveKind: ReadingKind | undefined = fixedKind ?? (kindFilter || undefined);
  const trimmedQ = searchInput.trim();

  const query = useMemo<ReadingListQuery>(
    () => ({
      kind: effectiveKind,
      series: seriesFilter || undefined,
      format: formatFilter || undefined,
      favorite: mode === "favorites" ? true : undefined,
      completed:
        statusFilter === "completed" ? true : statusFilter === "reading" ? false : undefined,
      q: trimmedQ || undefined,
      sort,
      dir,
    }),
    [effectiveKind, seriesFilter, formatFilter, mode, statusFilter, trimmedQ, sort, dir],
  );

  const shouldRunSearch = mode !== "search" || trimmedQ.length > 0;

  const singleQuery = useReadingLibraryList(
    effectiveLibrary && shouldRunSearch ? effectiveLibrary : undefined,
    query,
  );
  const multiQuery = useReadingAcrossLibraries(
    libraries,
    query,
    !effectiveLibrary && shouldRunSearch,
  );

  const entries = effectiveLibrary
    ? (singleQuery.data?.pages.flatMap((p) => p.entries) ?? [])
    : (multiQuery.data?.entries ?? []);
  const total = effectiveLibrary
    ? (singleQuery.data?.pages[0]?.total ?? 0)
    : (multiQuery.data?.total ?? 0);
  const isLoading =
    libsLoading || (shouldRunSearch && (effectiveLibrary ? singleQuery.isLoading : multiQuery.isLoading));
  const error = effectiveLibrary ? singleQuery.error : multiQuery.error;

  const formats =
    effectiveKind === "book"
      ? BOOK_FORMATS
      : effectiveKind === "comic"
        ? COMIC_FORMATS
        : [...new Set([...BOOK_FORMATS, ...COMIC_FORMATS])];

  const hasActiveFilters =
    Boolean(formatFilter) ||
    statusFilter !== "all" ||
    Boolean(seriesFilter) ||
    (mode !== "search" && Boolean(trimmedQ)) ||
    Boolean(kindFilter);

  const clearFilters = () => {
    setFormatFilter("");
    setStatusFilter("all");
    setSeriesFilter("");
    setKindFilter("");
    if (mode !== "search") setSearchInput("");
  };

  const title =
    mode === "books"
      ? t("readingApp.shelf.booksTitle")
      : mode === "comics"
        ? t("readingApp.shelf.comicsTitle")
        : mode === "favorites"
          ? t("readingApp.shelf.favoritesTitle")
          : t("readingApp.search.title");

  return (
    <PageFrame className="gap-5">
      <PageHeader
        title={title}
        summary={shouldRunSearch && !isLoading ? t("readingApp.shelf.totalCount", { count: total }) : undefined}
        actions={
          effectiveLibrary ? (
            <Link
              to="/library/$libraryId"
              params={{ libraryId: effectiveLibrary }}
              search={{ view: "files" }}
              className="inline-flex h-8 items-center gap-1.5 rounded-control border border-line bg-surface px-3 text-xs font-mid text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <FolderOpen className="size-3.5" aria-hidden />
              <span>{t("readingApp.shelf.filesView")}</span>
            </Link>
          ) : undefined
        }
      />

      {/* Filter & Sort Controls */}
      <div className="flex flex-wrap items-center gap-2.5">
        <div className="relative min-w-[14rem] flex-1 sm:max-w-sm">
          <Search
            className="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-fg-subtle"
            aria-hidden
          />
          <FieldInput
            type="search"
            value={searchInput}
            onChange={(e) => {
              setSearchInput(e.target.value);
              onQueryChange?.(e.target.value);
            }}
            placeholder={t("readingApp.search.placeholder")}
            aria-label={t("readingApp.search.placeholder")}
            className="w-full ps-9"
          />
        </div>

        {libraries.length > 1 ? (
          <Select
            value={effectiveLibrary}
            onChange={(e) => setSelectedLibrary(e.target.value)}
            aria-label={t("readingApp.shelf.allLibraries")}
          >
            <option value="">{t("readingApp.shelf.allLibraries")}</option>
            {libraries.map((lib) => (
              <option key={lib.id} value={lib.id}>
                {lib.name}
              </option>
            ))}
          </Select>
        ) : null}

        {!fixedKind ? (
          <Select
            value={kindFilter}
            onChange={(e) => setKindFilter(e.target.value as "" | ReadingKind)}
            aria-label={t("readingApp.search.allKinds")}
          >
            <option value="">{t("readingApp.search.allKinds")}</option>
            <option value="book">{t("readingApp.search.onlyBooks")}</option>
            <option value="comic">{t("readingApp.search.onlyComics")}</option>
          </Select>
        ) : null}

        <Select
          value={formatFilter}
          onChange={(e) => setFormatFilter(e.target.value)}
          aria-label={t("readingApp.shelf.allFormats")}
        >
          <option value="">{t("readingApp.shelf.allFormats")}</option>
          {formats.map((fmt) => (
            <option key={fmt} value={fmt}>
              {fmt.toUpperCase()}
            </option>
          ))}
        </Select>

        <Select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value as "all" | "reading" | "completed")}
          aria-label={t("readingApp.shelf.allStatus")}
        >
          <option value="all">{t("readingApp.shelf.allStatus")}</option>
          <option value="reading">{t("readingApp.shelf.statusReading")}</option>
          <option value="completed">{t("readingApp.shelf.statusCompleted")}</option>
        </Select>

        <Select
          value={sort}
          onChange={(e) => {
            const next = e.target.value as ReadingSort;
            setSort(next);
            setDir(next === "added" ? "desc" : "asc");
          }}
          aria-label={t("readingApp.shelf.sortTitle")}
        >
          <option value="added">{t("readingApp.shelf.sortAdded")}</option>
          <option value="title">{t("readingApp.shelf.sortTitle")}</option>
          <option value="series">{t("readingApp.shelf.sortSeries")}</option>
        </Select>

        <button
          type="button"
          onClick={() => setDir((d) => (d === "asc" ? "desc" : "asc"))}
          aria-label={dir === "asc" ? t("readingApp.shelf.sortAsc") : t("readingApp.shelf.sortDesc")}
          className="inline-flex size-(--pv-control-h) items-center justify-center rounded-field border border-line-control bg-surface text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        >
          {dir === "asc" ? <ArrowUpAZ className="size-4" aria-hidden /> : <ArrowDownAZ className="size-4" aria-hidden />}
        </button>

        {seriesFilter ? (
          <button
            type="button"
            onClick={() => setSeriesFilter("")}
            className="inline-flex h-8 items-center gap-1 rounded-chip bg-sunken px-2.5 text-xs font-strong text-fg ring-1 ring-line hover:bg-hover"
          >
            <span>{t("readingApp.shelf.seriesBadge", { series: seriesFilter })}</span>
            <X className="size-3.5" aria-hidden />
          </button>
        ) : null}

        {hasActiveFilters ? (
          <Button size="sm" variant="ghost" onPress={clearFilters}>
            {t("readingApp.shelf.clearFilters")}
          </Button>
        ) : null}
      </div>

      {/* Shelf Grid */}
      {mode === "search" && !shouldRunSearch ? (
        <EmptyState
          icon={Search}
          title={t("readingApp.search.promptTitle")}
          detail={t("readingApp.search.promptDetail")}
        />
      ) : isLoading ? (
        <LoadingRows rows={6} />
      ) : error ? (
        <ErrorState
          error={error}
          onRetry={() => void (effectiveLibrary ? singleQuery.refetch() : multiQuery.refetch())}
        />
      ) : entries.length === 0 ? (
        <EmptyState
          icon={mode === "favorites" ? Heart : fixedKind === "comic" ? Layers : BookOpen}
          title={
            mode === "favorites"
              ? t("readingApp.shelf.emptyFavorites")
              : mode === "search"
                ? t("readingApp.search.noResults", { query: trimmedQ })
                : t("readingApp.shelf.emptyTitle")
          }
          detail={mode === "favorites" ? t("readingApp.shelf.emptyFavoritesDetail") : undefined}
        />
      ) : (
        <div className="grid gap-6">
          <div className="pv-grid-poster">
            {entries.map((entry) => (
              <ReadingCard key={entry.itemId} entry={entry} />
            ))}
          </div>

          {effectiveLibrary && singleQuery.hasNextPage ? (
            <div className="flex justify-center">
              <Button
                variant="outline"
                isPending={singleQuery.isFetchingNextPage}
                onPress={() => void singleQuery.fetchNextPage()}
              >
                {t("readingApp.shelf.loadMore")}
              </Button>
            </div>
          ) : null}
        </div>
      )}
    </PageFrame>
  );
}
