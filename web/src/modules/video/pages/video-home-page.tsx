import { buttonVariants } from "@heroui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Clapperboard, Film, Heart, Info, Play, Search, Star, Tv } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import type { Library } from "@/api/types";
import { cn } from "@/lib/cn";
import { formatNumber } from "@/lib/format";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { imagePaths, type VideoEntry } from "../api";
import { MediaRail } from "../components/media-rail";
import { PosterCard, VideoWideCard } from "../components/poster-card";
import { VideoImage } from "../components/video-image";
import { formatClock, formatEpisodeCode, formatRuntime } from "../format";
import { useToggleWorkFavorite, useVideoHome } from "../queries";

function FeaturedHero({ items }: { items: VideoEntry[] }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const toggleFavorite = useToggleWorkFavorite();
  const [activeIndex, setActiveIndex] = useState(0);

  const safeIndex = items.length > 0 ? activeIndex % items.length : 0;
  const entry = items[safeIndex];
  if (!entry) return null;

  const workId = entry.seriesId ?? entry.id;
  const isEpisode = entry.kind === "episode";
  const hasResume = Boolean(!entry.state?.played && (entry.state?.positionMs ?? 0) > 0);
  const favorite = Boolean(entry.state?.favorite);
  const epCode = formatEpisodeCode(entry.seasonNumber, entry.episodeNumber);
  const title = isEpisode && entry.seriesTitle ? entry.seriesTitle : entry.title;
  const backdropPath = entry.backdropItemId
    ? imagePaths.backdrop(entry.backdropItemId, 1920)
    : entry.posterItemId
      ? imagePaths.poster(entry.posterItemId, "work", 960)
      : entry.thumbItemId
        ? imagePaths.thumb(entry.thumbItemId, 960)
        : entry.itemId
          ? imagePaths.thumb(entry.itemId, 960)
          : null;

  const playAction = () => {
    if (entry.itemId) {
      void navigate({
        to: "/video/play/$itemId",
        params: { itemId: entry.itemId },
        search: {
          libraryId: entry.libraryId,
          workId,
          season: entry.seasonNumber ?? undefined,
        },
      });
    } else {
      void navigate({
        to: "/video/$libraryId/works/$workId",
        params: { libraryId: entry.libraryId, workId },
      });
    }
  };

  const runtimeText = formatRuntime(entry.runtimeMs, t, true);

  return (
    <section
      aria-label={t("videoApp.home.featured")}
      className="group/hero relative isolate overflow-hidden rounded-[1.25rem] bg-neutral-950 text-white shadow-overlay ring-1 ring-white/12 ring-inset"
    >
      <div className="absolute inset-0 -z-10">
        <VideoImage
          path={backdropPath}
          alt={title}
          eager
          className="size-full bg-neutral-950"
          imgClassName="size-full object-cover object-center transition-transform duration-700 ease-out group-hover/hero:scale-[1.02]"
        />
      </div>

      {/* Multi-stop cinematic scrims ensure high text contrast on bright artwork */}
      <div
        className="relative flex min-h-[22rem] flex-col justify-end p-6 sm:min-h-[26rem] sm:p-9 lg:min-h-[29rem] lg:p-11"
        style={{
          backgroundImage:
            "linear-gradient(95deg, rgba(8,11,17,0.94) 0%, rgba(8,11,17,0.72) 44%, rgba(8,11,17,0.16) 100%), linear-gradient(0deg, rgba(8,11,17,0.92) 0%, rgba(8,11,17,0.22) 52%, rgba(8,11,17,0.18) 100%)",
        }}
      >
        <div className="grid max-w-xl gap-3.5">
          <div className="flex flex-wrap items-center gap-2 text-xs font-mid text-white/85">
            {epCode ? (
              <span className="rounded-full bg-white/16 px-2.5 py-0.5 font-strong text-white backdrop-blur-md">
                {epCode}
              </span>
            ) : null}
            {entry.year != null ? <span data-num>{entry.year}</span> : null}
            {runtimeText ? (
              <>
                <span aria-hidden>·</span>
                <span>{runtimeText}</span>
              </>
            ) : null}
            {entry.communityRating != null ? (
              <>
                <span aria-hidden>·</span>
                <span className="inline-flex items-center gap-1 font-strong text-white tabular-nums">
                  <Star className="size-3.5 fill-amber-400 text-amber-400" aria-hidden />
                  {entry.communityRating.toFixed(1)}
                </span>
              </>
            ) : null}
            {entry.officialRating ? (
              <span className="rounded-sm border border-white/30 bg-white/8 px-1.5 py-0.5 text-[0.6875rem] leading-none font-strong tracking-wide text-white/95 backdrop-blur-xs">
                {entry.officialRating}
              </span>
            ) : null}
          </div>

          <h2 className="font-heading text-3xl leading-[1.1] font-title tracking-tight text-white sm:text-5xl">
            {title}
          </h2>

          {entry.genres.length > 0 ? (
            <div className="flex flex-wrap items-center gap-1.5">
              {entry.genres.slice(0, 4).map((genre) => (
                <span
                  key={genre}
                  className="rounded-full border border-white/12 bg-white/10 px-2.5 py-0.5 text-xs text-white/90 backdrop-blur-md"
                >
                  {genre}
                </span>
              ))}
            </div>
          ) : null}

          {entry.overview ? (
            <p className="line-clamp-2 max-w-prose text-sm leading-relaxed text-white/80 sm:line-clamp-3 sm:text-base">
              {entry.overview}
            </p>
          ) : null}

          <div className="mt-1.5 flex flex-wrap items-center gap-3">
            <button
              type="button"
              onClick={playAction}
              className="inline-flex h-11 items-center gap-2 rounded-full bg-white px-6 text-sm font-strong text-neutral-950 shadow-raised transition-all duration-150 hover:scale-[1.02] hover:bg-white/92 active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
            >
              <Play className="size-4 fill-current" aria-hidden />
              {hasResume && entry.state
                ? t("videoApp.detail.resumeAt", { time: formatClock(entry.state.positionMs) })
                : t("videoApp.detail.play")}
            </button>

            <Link
              to="/video/$libraryId/works/$workId"
              params={{ libraryId: entry.libraryId, workId }}
              search={entry.seasonNumber != null ? { season: entry.seasonNumber } : {}}
              className="inline-flex h-11 items-center gap-2 rounded-full border border-white/22 bg-white/12 px-5 text-sm font-mid text-white backdrop-blur-md transition-colors hover:bg-white/22 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
            >
              <Info className="size-4" aria-hidden />
              {t("videoApp.card.details")}
            </Link>

            {!isEpisode ? (
              <button
                type="button"
                aria-pressed={favorite}
                aria-label={t(favorite ? "videoApp.card.unfavorite" : "videoApp.card.favorite")}
                onClick={() =>
                  toggleFavorite.mutate({ libraryId: entry.libraryId, workId, favorite: !favorite })
                }
                className={cn(
                  "grid size-11 place-items-center rounded-full border border-white/22 bg-white/12 text-white backdrop-blur-md transition-colors hover:bg-white/22 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white",
                  favorite && "border-rose-400/60 bg-rose-500/25 text-rose-200",
                )}
              >
                <Heart className={cn("size-4", favorite && "fill-current")} aria-hidden />
              </button>
            ) : null}
          </div>
        </div>

        {items.length > 1 ? (
          <div
            role="tablist"
            aria-label={t("videoApp.home.switchFeatured")}
            className="mt-5 flex items-center gap-1.5 sm:absolute sm:right-8 sm:bottom-8 sm:mt-0"
          >
            {items.map((item, idx) => {
              const selected = idx === safeIndex;
              return (
                <button
                  key={item.id}
                  type="button"
                  role="tab"
                  aria-selected={selected}
                  aria-label={item.seriesTitle ?? item.title}
                  onClick={() => setActiveIndex(idx)}
                  className={cn(
                    "h-2 rounded-full transition-all focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white",
                    selected ? "w-7 bg-white" : "w-2 bg-white/40 hover:bg-white/70",
                  )}
                />
              );
            })}
          </div>
        ) : null}
      </div>
    </section>
  );
}

function LibraryTiles({
  libraries,
  previewsByLibrary,
}: {
  libraries: Library[];
  previewsByLibrary: Map<string, { itemId: string; kind: "backdrop" | "thumb" }>;
}) {
  const { t, i18n } = useTranslation();
  if (libraries.length === 0) return null;
  const locale = i18n.resolvedLanguage ?? "zh-CN";

  return (
    <section aria-label={t("videoApp.home.libraries")} className="grid gap-3">
      <h2 className="font-heading text-lg font-title text-heading">{t("videoApp.home.libraries")}</h2>
      <div className="pv-grid-card">
        {libraries.map((lib) => {
          const preview = previewsByLibrary.get(lib.id);
          const backdrop = preview
            ? preview.kind === "backdrop"
              ? imagePaths.backdrop(preview.itemId, 480)
              : imagePaths.thumb(preview.itemId, 480)
            : null;
          return (
            <Link
              key={lib.id}
              to="/video/$libraryId"
              params={{ libraryId: lib.id }}
              className="group relative isolate flex h-28 items-end overflow-hidden rounded-panel bg-neutral-900 p-5 text-white shadow-art ring-1 ring-white/12 ring-inset outline-none transition-all duration-200 hover:-translate-y-1 hover:shadow-overlay focus-visible:ring-2 focus-visible:ring-focus"
            >
              <div className="absolute inset-0 -z-10">
                <VideoImage
                  path={backdrop}
                  fallbackIcon={Film}
                  className="size-full bg-neutral-900"
                  imgClassName="size-full object-cover transition-transform duration-300 group-hover:scale-105"
                />
              </div>
              <div
                className="pointer-events-none absolute inset-0 -z-10"
                style={{
                  backgroundImage:
                    "linear-gradient(90deg, rgba(8,10,14,0.88) 0%, rgba(8,10,14,0.52) 60%, rgba(8,10,14,0.28) 100%)",
                }}
              />
              <div className="flex min-w-0 flex-1 items-center justify-between gap-3">
                <div className="grid min-w-0 gap-0.5">
                  <span className="truncate font-heading text-lg font-title text-white">{lib.name}</span>
                  <span data-num className="text-xs text-white/80 tabular-nums">
                    {formatNumber(lib.itemCount, locale)} {t("units.video")}
                  </span>
                </div>
                <span className="grid size-9 shrink-0 place-items-center rounded-full bg-white/14 text-white backdrop-blur-xs">
                  {lib.name.includes("剧") || lib.name.toLowerCase().includes("tv") ? (
                    <Tv className="size-4" aria-hidden />
                  ) : (
                    <Clapperboard className="size-4" aria-hidden />
                  )}
                </span>
              </div>
            </Link>
          );
        })}
      </div>
    </section>
  );
}

export function VideoHomePage() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const librariesQuery = useLibraries();
  const videoLibraries = useMemo(
    () => (librariesQuery.data?.libraries ?? []).filter((lib) => lib.kind === "video"),
    [librariesQuery.data],
  );
  const homeQuery = useVideoHome();

  const home = homeQuery.data;

  // Pick up to 5 featured works (preferring backdrops/posters, falling back to grabbed video thumbs)
  const featured = useMemo(() => {
    if (!home) return [];
    const candidates = [...home.latestMovies, ...home.continueWatching, ...home.nextUp];
    const seen = new Set<string>();
    const list: VideoEntry[] = [];
    for (const candidate of candidates) {
      const workKey = candidate.seriesId ?? candidate.id;
      if (seen.has(workKey) || (!candidate.backdropItemId && !candidate.posterItemId)) continue;
      seen.add(workKey);
      list.push(candidate);
      if (list.length >= 5) return list;
    }
    for (const candidate of candidates) {
      const workKey = candidate.seriesId ?? candidate.id;
      if (seen.has(workKey) || (!candidate.thumbItemId && !candidate.itemId)) continue;
      seen.add(workKey);
      list.push(candidate);
      if (list.length >= 5) break;
    }
    return list;
  }, [home]);

  const previewsByLibrary = useMemo(() => {
    const map = new Map<string, { itemId: string; kind: "backdrop" | "thumb" }>();
    if (!home) return map;
    const all = [...home.latestMovies, ...home.latestEpisodes, ...home.continueWatching, ...home.nextUp];
    for (const entry of all) {
      if (!map.has(entry.libraryId) && entry.backdropItemId) {
        map.set(entry.libraryId, { itemId: entry.backdropItemId, kind: "backdrop" });
      }
    }
    for (const entry of all) {
      const thumbId = entry.thumbItemId ?? entry.itemId;
      if (!map.has(entry.libraryId) && thumbId) {
        map.set(entry.libraryId, { itemId: thumbId, kind: "thumb" });
      }
    }
    return map;
  }, [home]);

  const primaryMoviesLib = home?.latestMovies[0]?.libraryId ?? videoLibraries[0]?.id;
  const primarySeriesLib = home?.latestEpisodes[0]?.libraryId ?? videoLibraries[1]?.id ?? videoLibraries[0]?.id;

  const isEmpty =
    home &&
    home.continueWatching.length === 0 &&
    home.nextUp.length === 0 &&
    home.latestMovies.length === 0 &&
    home.latestEpisodes.length === 0;

  return (
    <div className="pv-enter pv-page-frame mx-auto flex w-full max-w-(--pv-layout-max) flex-col gap-7 px-(--pv-layout-gutter) pt-(--pv-layout-top) pb-(--pv-layout-bottom) max-md:gap-6">
      <PageHeader
        title={t("videoApp.title")}
        actions={
          videoLibraries.length > 0 ? (
            <Link
              to="/video/search"
              className="inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border border-line bg-surface px-3 text-sm font-mid text-fg-muted transition-colors hover:bg-hover hover:text-fg"
            >
              <Search className="size-3.5" aria-hidden />
              {t("videoApp.nav.search")}
            </Link>
          ) : undefined
        }
      />

      {librariesQuery.isPending || homeQuery.isPending ? (
        <LoadingRows rows={6} />
      ) : homeQuery.isError ? (
        <ErrorState error={homeQuery.error} onRetry={() => void homeQuery.refetch()} />
      ) : videoLibraries.length === 0 ? (
        <EmptyState
          icon={Film}
          title={t("videoApp.home.noLibraries")}
          detail={t("videoApp.home.noLibrariesDetail")}
          action={
            isAdmin ? (
              <Link to="/system/libraries" className={buttonVariants({ variant: "primary", size: "sm" })}>
                {t("videoApp.home.manageLibraries")}
              </Link>
            ) : null
          }
        />
      ) : isEmpty ? (
        <>
          <LibraryTiles libraries={videoLibraries} previewsByLibrary={previewsByLibrary} />
          <EmptyState
            icon={Film}
            title={t("videoApp.home.empty")}
            detail={t("videoApp.home.emptyDetail")}
          />
        </>
      ) : (
        <>
          {featured.length > 0 ? <FeaturedHero items={featured} /> : null}

          {home && home.continueWatching.length > 0 ? (
            <MediaRail title={t("videoApp.home.continueWatching")} count={home.continueWatching.length}>
              {home.continueWatching.map((entry) => (
                <VideoWideCard
                  key={entry.id}
                  entry={entry}
                  variant="continue"
                  className="w-64 shrink-0 snap-start sm:w-72 lg:w-80"
                />
              ))}
            </MediaRail>
          ) : null}

          {home && home.nextUp.length > 0 ? (
            <MediaRail title={t("videoApp.home.nextUp")} count={home.nextUp.length}>
              {home.nextUp.map((entry) => (
                <VideoWideCard
                  key={entry.id}
                  entry={entry}
                  variant="nextUp"
                  className="w-64 shrink-0 snap-start sm:w-72 lg:w-80"
                />
              ))}
            </MediaRail>
          ) : null}

          <LibraryTiles libraries={videoLibraries} previewsByLibrary={previewsByLibrary} />

          {home && home.latestMovies.length > 0 ? (
            <MediaRail
              title={t("videoApp.home.latestMovies")}
              count={home.latestMovies.length}
              moreTo={primaryMoviesLib ? "/video/$libraryId" : undefined}
              moreParams={primaryMoviesLib ? { libraryId: primaryMoviesLib } : undefined}
              moreSearch={primaryMoviesLib ? { tab: "movie", order: "added", dir: "desc" } : undefined}
            >
              {home.latestMovies.map((entry) => (
                <PosterCard
                  key={entry.id}
                  entry={entry}
                  className="w-36 shrink-0 snap-start sm:w-40 lg:w-44"
                />
              ))}
            </MediaRail>
          ) : null}

          {home && home.latestEpisodes.length > 0 ? (
            <MediaRail
              title={t("videoApp.home.latestEpisodes")}
              count={home.latestEpisodes.length}
              moreTo={primarySeriesLib ? "/video/$libraryId" : undefined}
              moreParams={primarySeriesLib ? { libraryId: primarySeriesLib } : undefined}
              moreSearch={primarySeriesLib ? { tab: "series", order: "added", dir: "desc" } : undefined}
            >
              {home.latestEpisodes.map((entry) => (
                <VideoWideCard
                  key={entry.id}
                  entry={entry}
                  variant="latestEpisode"
                  className="w-64 shrink-0 snap-start sm:w-72 lg:w-80"
                />
              ))}
            </MediaRail>
          ) : null}
        </>
      )}
    </div>
  );
}
