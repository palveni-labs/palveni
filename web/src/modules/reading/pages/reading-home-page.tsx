import { Link, useNavigate } from "@tanstack/react-router";
import { BookOpen, ChevronRight, FolderOpen, Layers, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { readingImagePaths, type ReadingEntry, type ReadingSeries } from "../api";
import { ReadingCard } from "../components/reading-card";
import { ReadingImage } from "../components/reading-image";
import { useReadingHome, useReadingLibraries } from "../queries";

function ShelfSection({
  title,
  moreTo,
  moreLabel,
  entries,
}: {
  title: string;
  moreTo?: "/reading/books" | "/reading/comics";
  moreLabel?: string;
  entries: ReadingEntry[];
}) {
  if (entries.length === 0) return null;
  return (
    <section aria-label={title} className="grid gap-3">
      <div className="flex items-center justify-between gap-2">
        <h2 className="font-heading text-lg font-title text-heading">{title}</h2>
        {moreTo && moreLabel ? (
          <Link
            to={moreTo}
            className="inline-flex items-center gap-0.5 text-sm font-mid text-fg-muted transition-colors hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <span>{moreLabel}</span>
            <ChevronRight className="size-4" aria-hidden />
          </Link>
        ) : null}
      </div>
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8">
        {entries.map((entry) => (
          <ReadingCard key={entry.itemId} entry={entry} />
        ))}
      </div>
    </section>
  );
}

function SeriesSection({ series }: { series: ReadingSeries[] }) {
  const { t } = useTranslation();
  if (series.length === 0) return null;

  return (
    <section aria-label={t("readingApp.home.series")} className="grid gap-3">
      <h2 className="font-heading text-lg font-title text-heading">{t("readingApp.home.series")}</h2>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
        {series.map((s) => (
          <Link
            key={`${s.libraryId}:${s.kind}:${s.seriesKey}`}
            to={s.kind === "comic" ? "/reading/comics" : "/reading/books"}
            search={{ library: s.libraryId, series: s.seriesName }}
            className="group flex items-center gap-3.5 rounded-panel bg-surface p-3 ring-1 ring-line transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <ReadingImage
              path={s.hasCover ? readingImagePaths.cover(s.coverItemId, 240) : null}
              alt={s.seriesName}
              fallbackIcon={s.kind === "comic" ? Layers : BookOpen}
              fallbackTitle={s.seriesName}
              className="aspect-[2/3] w-14 shrink-0 rounded-tile shadow-art ring-1 ring-line/50"
            />
            <div className="grid min-w-0 flex-1 gap-1">
              <span className="truncate font-strong text-fg group-hover:underline">{s.seriesName}</span>
              <span className="text-xs text-fg-muted tabular-nums">
                {s.completedCount > 0
                  ? t("readingApp.home.seriesProgress", { done: s.completedCount, total: s.itemCount })
                  : t("readingApp.home.seriesCount", { count: s.itemCount })}
              </span>
            </div>
            <ChevronRight className="size-4 shrink-0 text-fg-subtle" aria-hidden />
          </Link>
        ))}
      </div>
    </section>
  );
}

export function ReadingHomePage({ libraryId }: { libraryId?: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { libraries, isLoading: libsLoading } = useReadingLibraries();
  const activeLib = libraries.some((l) => l.id === libraryId) ? libraryId : undefined;
  const homeQuery = useReadingHome(activeLib);

  const selectLibrary = (nextId?: string) => {
    void navigate({
      to: "/reading",
      search: nextId ? { library: nextId } : {},
    });
  };

  const data = homeQuery.data;
  const isEmpty =
    data &&
    data.continueReading.length === 0 &&
    data.recentBooks.length === 0 &&
    data.recentComics.length === 0 &&
    data.series.length === 0;

  return (
    <div className="grid gap-6 pb-8">
      <PageHeader
        title={t("readingApp.title")}
        summary={t("readingApp.home.summary")}
        actions={
          <Link
            to="/reading/search"
            className="inline-flex h-8 items-center gap-1.5 rounded-control border border-line bg-surface px-3 text-sm font-mid text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <Search className="size-4" aria-hidden />
            <span>{t("readingApp.nav.search")}</span>
          </Link>
        }
      />

      {/* Library switcher */}
      {libraries.length > 1 ? (
        <div role="tablist" aria-label={t("readingApp.home.libraries")} className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            role="tab"
            aria-selected={!activeLib}
            onClick={() => selectLibrary(undefined)}
            className={cn(
              "h-8 rounded-chip px-3 text-sm font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              !activeLib
                ? "bg-action text-action-fg font-strong"
                : "bg-surface text-fg-muted ring-1 ring-line hover:text-fg",
            )}
          >
            {t("readingApp.home.allLibraries")}
          </button>
          {libraries.map((lib) => {
            const selected = activeLib === lib.id;
            return (
              <button
                key={lib.id}
                type="button"
                role="tab"
                aria-selected={selected}
                onClick={() => selectLibrary(lib.id)}
                className={cn(
                  "h-8 rounded-chip px-3 text-sm font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  selected
                    ? "bg-action text-action-fg font-strong"
                    : "bg-surface text-fg-muted ring-1 ring-line hover:text-fg",
                )}
              >
                {lib.name}
              </button>
            );
          })}
        </div>
      ) : null}

      {homeQuery.isLoading || libsLoading ? (
        <LoadingRows rows={5} />
      ) : homeQuery.error ? (
        <ErrorState error={homeQuery.error} onRetry={() => void homeQuery.refetch()} />
      ) : isEmpty ? (
        <EmptyState
          icon={BookOpen}
          title={t("readingApp.home.emptyTitle")}
          detail={t("readingApp.home.emptyDetail")}
        />
      ) : data ? (
        <div className="grid gap-8">
          <ShelfSection
            title={t("readingApp.home.continueReading")}
            entries={data.continueReading}
          />
          <ShelfSection
            title={t("readingApp.home.recentBooks")}
            moreTo="/reading/books"
            moreLabel={t("readingApp.home.viewAllBooks")}
            entries={data.recentBooks}
          />
          <ShelfSection
            title={t("readingApp.home.recentComics")}
            moreTo="/reading/comics"
            moreLabel={t("readingApp.home.viewAllComics")}
            entries={data.recentComics}
          />
          <SeriesSection series={data.series} />

          {libraries.length > 0 ? (
            <section aria-label={t("readingApp.home.libraries")} className="grid gap-3">
              <h2 className="font-heading text-lg font-title text-heading">{t("readingApp.home.libraries")}</h2>
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {libraries.map((lib) => (
                  <div
                    key={lib.id}
                    className="flex items-center justify-between gap-3 rounded-panel bg-surface p-3.5 ring-1 ring-line"
                  >
                    <Link
                      to={lib.kind === "comic" ? "/reading/comics" : "/reading/books"}
                      search={{ library: lib.id }}
                      className="flex min-w-0 flex-1 items-center gap-3 outline-none focus-visible:underline"
                    >
                      <span className="grid size-9 shrink-0 place-items-center rounded-tile bg-sunken text-fg-muted">
                        {lib.kind === "comic" ? (
                          <Layers className="size-4" aria-hidden />
                        ) : (
                          <BookOpen className="size-4" aria-hidden />
                        )}
                      </span>
                      <span className="grid min-w-0">
                        <span className="truncate font-strong text-fg">{lib.name}</span>
                        <span className="text-xs text-fg-muted tabular-nums">
                          {t("readingApp.shelf.totalCount", { count: lib.itemCount })}
                        </span>
                      </span>
                    </Link>
                    <Link
                      to="/library/$libraryId"
                      params={{ libraryId: lib.id }}
                      search={{ view: "files" }}
                      aria-label={`${lib.name} · ${t("readingApp.shelf.filesView")}`}
                      className="inline-flex h-7 shrink-0 items-center gap-1 rounded-control px-2 text-xs font-mid text-fg-muted transition-colors hover:bg-sunken hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                    >
                      <FolderOpen className="size-3.5" aria-hidden />
                      <span className="max-sm:sr-only">{t("readingApp.shelf.filesView")}</span>
                    </Link>
                  </div>
                ))}
              </div>
            </section>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
