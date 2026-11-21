import { Button } from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronRight, Clapperboard, Play, Tv } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { request, requestBlob } from "@/api/client";
import type { components } from "@/api/generated";
import { queryKeys } from "@/api/query-keys";
import { useScrollContainer } from "@/lib/scroll-container";
import { useElementWidth, useOffsetWithin } from "@/lib/use-element-size";
import { Section } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { gridLayout } from "./grid-layout";
import { OriginalPlayer } from "./original-player";

type VideoEntry = components["schemas"]["VideoEntry"];
type VideoPage = components["schemas"]["VideoPage"];

function Poster({
  entry,
  libraryId,
}: {
  entry: VideoEntry;
  libraryId: string;
}) {
  const imgRef = useRef<HTMLImageElement>(null);
  const level =
    entry.kind === "season" || entry.kind === "episode" ? entry.kind : "work";
  const image = useQuery({
    queryKey: [
      ...queryKeys.items.libraryLists(libraryId),
      "poster",
      entry.posterItemId,
      level,
    ],
    queryFn: ({ signal }) =>
      requestBlob(`/items/${entry.posterItemId}/poster?level=${level}`, {
        signal,
      }),
    enabled: Boolean(entry.posterItemId),
    retry: false,
    staleTime: 60_000,
    gcTime: 15_000,
  });
  useEffect(() => {
    const element = imgRef.current;
    if (!element || !image.data) return;
    const url = URL.createObjectURL(image.data);
    element.hidden = false;
    element.src = url;
    return () => {
      element.removeAttribute("src");
      URL.revokeObjectURL(url);
    };
  }, [image.data]);
  const Icon = entry.kind === "movie" ? Clapperboard : Tv;
  return (
    <span className="relative grid aspect-[2/3] w-full place-items-center overflow-hidden rounded-tile bg-sunken text-icon ring-1 ring-line ring-inset">
      <Icon aria-hidden className="size-8" />
      {image.data && (
        <img
          ref={imgRef}
          alt=""
          decoding="async"
          className="absolute inset-0 size-full object-cover"
          onError={(event) => {
            event.currentTarget.hidden = true;
          }}
        />
      )}
    </span>
  );
}

function VideoWall({
  libraryId,
  entries,
  onOpen,
  titleOf,
}: {
  libraryId: string;
  entries: VideoEntry[];
  onOpen: (entry: VideoEntry) => void;
  titleOf: (entry: VideoEntry) => string;
}) {
  const { t } = useTranslation();
  const scroll = useScrollContainer();
  const container = useRef<HTMLDivElement>(null);
  const width = useElementWidth(container);
  const scrollMargin = useOffsetWithin(container, scroll);
  const { columns, gap, rowHeight } = gridLayout(width, 2 / 3);
  // eslint-disable-next-line react-hooks/incompatible-library -- the grid renders the visible virtual rows
  const grid = useVirtualizer({
    count: Math.ceil(entries.length / columns),
    getScrollElement: () => scroll.current,
    estimateSize: () => rowHeight,
    scrollMargin,
    overscan: 1,
  });
  useEffect(() => {
    grid.measure();
  }, [grid, rowHeight, columns]);
  return (
    <div
      ref={container}
      role="list"
      className="relative w-full"
      style={{ height: grid.getTotalSize() }}
    >
      {grid.getVirtualItems().map((row) => (
        <div
          key={row.key}
          role="presentation"
          className="absolute inset-x-0 top-0 grid"
          style={{
            gridTemplateColumns: `repeat(${columns},minmax(0,1fr))`,
            gap,
            transform: `translateY(${row.start - scrollMargin}px)`,
          }}
        >
          {entries
            .slice(row.index * columns, (row.index + 1) * columns)
            .map((entry) => (
              <div role="listitem" key={entry.id} className="min-w-0">
                <button
                  type="button"
                  title={titleOf(entry)}
                  onClick={() => onOpen(entry)}
                  className="group flex w-full min-w-0 flex-col gap-2 rounded-tile text-start outline-none focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-(--pv-focus)"
                >
                  <Poster entry={entry} libraryId={libraryId} />
                  <span className="grid w-full gap-0.5 px-0.5">
                    <span className="truncate text-sm font-mid text-fg">
                      {titleOf(entry)}
                    </span>
                    <span className="truncate text-xs text-fg-muted">
                      {entry.kind === "series" || entry.kind === "season"
                        ? t("video.entries", { count: entry.fileCount })
                        : [
                            entry.year,
                            entry.kind === "episode"
                              ? entry.episodeNumber != null
                                ? t("video.episode", {
                                    number: entry.episodeNumber,
                                  })
                                : t("video.series")
                              : t("video.movie"),
                          ]
                            .filter(Boolean)
                            .join(" · ")}
                    </span>
                  </span>
                </button>
              </div>
            ))}
        </div>
      ))}
    </div>
  );
}

function VideoDetails({
  libraryId,
  entry,
  back,
}: {
  libraryId: string;
  entry: VideoEntry;
  back: () => void;
}) {
  const { t } = useTranslation();
  const [playing, setPlaying] = useState(false);
  return (
    <Section title={entry.title} unframed>
      <div className="flex flex-wrap items-start gap-6">
        <div className="w-32 shrink-0 sm:w-44">
          <Poster entry={entry} libraryId={libraryId} />
        </div>
        <div className="grid min-w-0 flex-1 gap-3">
          {entry.year !== null && (
            <p className="text-sm text-fg-muted">{entry.year}</p>
          )}
          {entry.overview && (
            <p className="max-w-prose whitespace-pre-line text-fg">
              {entry.overview}
            </p>
          )}
          <div className="flex flex-wrap items-center gap-2">
            {!playing && entry.itemId && (
              <Button size="sm" onPress={() => setPlaying(true)}>
                <Play aria-hidden className="size-4" />
                {t("video.preview")}
              </Button>
            )}
            <Button size="sm" variant="outline" onPress={back}>
              {t("common.back")}
            </Button>
          </div>
        </div>
      </div>
      {playing && entry.itemId && (
        <div className="mt-4">
          <OriginalPlayer id={entry.itemId} title={entry.title} video />
        </div>
      )}
    </Section>
  );
}

export function VideoBrowse({ libraryId }: { libraryId: string }) {
  const { series, seriesName, season, order } = useSearch({
    from: "/_app/library/$libraryId",
  });
  return (
    <VideoLibraryView
      key={`${libraryId}/${series ?? ""}/${season ?? ""}`}
      libraryId={libraryId}
      series={series}
      seriesName={seriesName}
      season={season}
      order={order}
    />
  );
}

function VideoLibraryView({
  libraryId,
  series,
  seriesName,
  season,
  order,
}: {
  libraryId: string;
  series?: string;
  seriesName?: string;
  season?: number;
  order?: "title";
}) {
  const { t } = useTranslation();
  const navigate = useNavigate({ from: "/library/$libraryId" });
  const [selected, setSelected] = useState<VideoEntry | null>(null);
  const listOrder = series ? undefined : (order ?? "poster");
  const listing = useInfiniteQuery({
    queryKey: [
      ...queryKeys.items.libraryLists(libraryId),
      "works",
      series,
      season,
      listOrder,
    ],
    queryFn: ({ pageParam, signal }) =>
      request<VideoPage>(`/libraries/${libraryId}/videos`, {
        query: { series, season, order: listOrder, cursor: pageParam },
        signal,
      }),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });
  const entries = listing.data?.pages.flatMap((page) => page.entries) ?? [];
  const seasonTitle = (number: number) =>
    number === 0 ? t("video.specials") : t("video.season", { number });
  const titleOf = (entry: VideoEntry) =>
    entry.kind === "season" && entry.seasonNumber != null
      ? seasonTitle(entry.seasonNumber)
      : entry.title;
  const title =
    season != null
      ? t("video.episodes")
      : series
        ? t("video.seasons")
        : t("video.works");
  const open = (entry: VideoEntry) => {
    if (entry.kind === "series") {
      void navigate({
        search: { view: "works", order, series: entry.id, seriesName: entry.title },
      });
    } else if (entry.kind === "season" && entry.seasonNumber != null) {
      void navigate({
        search: {
          view: "works",
          order,
          series,
          seriesName,
          season: entry.seasonNumber,
        },
      });
    } else setSelected(entry);
  };
  return (
    <>
      {series && (
        <nav
          aria-label={t("a11y.breadcrumb")}
          className="flex flex-wrap items-center gap-2 text-sm text-fg-muted"
        >
          <Link
            to="/library/$libraryId"
            params={{ libraryId }}
            search={{ view: "works", order }}
            onClick={() => setSelected(null)}
          >
            {t("video.works")}
          </Link>
          <ChevronRight aria-hidden className="size-4" />
          <Link
            to="/library/$libraryId"
            params={{ libraryId }}
            search={{ view: "works", order, series, seriesName }}
            onClick={() => setSelected(null)}
          >
            {seriesName ?? t("video.series")}
          </Link>
          {season != null && (
            <>
              <ChevronRight aria-hidden className="size-4" />
              <span aria-current="page">{seasonTitle(season)}</span>
            </>
          )}
        </nav>
      )}
      {selected ? (
        <VideoDetails
          key={selected.id}
          libraryId={libraryId}
          entry={selected}
          back={() => setSelected(null)}
        />
      ) : (
        <Section
          title={title}
          bare={Boolean(series)}
          unframed
          actions={!series && (
            <select
              aria-label={t("video.sort")}
              value={listOrder}
              onChange={(event) => void navigate({
                search: { view: "works", order: event.target.value === "title" ? "title" : undefined },
              })}
              className="rounded-control border border-line bg-surface px-2 py-1 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <option value="poster">{t("video.posterFirst")}</option>
              <option value="title">{t("video.byTitle")}</option>
            </select>
          )}
        >
          {listing.isPending ? (
            <LoadingRows />
          ) : listing.isError && entries.length === 0 ? (
            <ErrorState
              error={listing.error}
              onRetry={() => void listing.refetch()}
            />
          ) : entries.length === 0 ? (
            <EmptyState
              icon={Clapperboard}
              title={t("video.empty")}
              detail={t("video.scanHint")}
            />
          ) : (
            <VideoWall
              libraryId={libraryId}
              entries={entries}
              titleOf={titleOf}
              onOpen={open}
            />
          )}
          {listing.isFetchNextPageError && (
            <ErrorState
              compact
              error={listing.error}
              onRetry={() => void listing.fetchNextPage()}
            />
          )}
          {listing.hasNextPage && !listing.isFetchNextPageError && (
            <Button
              size="sm"
              variant="outline"
              onPress={() => void listing.fetchNextPage()}
              isDisabled={listing.isFetchingNextPage}
            >
              {t("music.more")}
            </Button>
          )}
        </Section>
      )}
    </>
  );
}
