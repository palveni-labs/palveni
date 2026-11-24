import { Dropdown, Label } from "@heroui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Check, Clapperboard, Eye, EyeOff, Heart, Info, MoreHorizontal, Play, Star, Tv } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { imagePaths, type VideoEntry } from "../api";
import { formatEpisodeCode, formatRuntime, remainingMs, watchProgress } from "../format";
import { useToggleItemPlayed, useToggleWorkFavorite, useToggleWorkPlayed } from "../queries";
import { VideoImage } from "./video-image";

function CardMenu({
  entry,
  onPlay,
  isEpisode = false,
}: {
  entry: VideoEntry;
  onPlay?: () => void;
  isEpisode?: boolean;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const toggleWorkPlayed = useToggleWorkPlayed();
  const toggleWorkFavorite = useToggleWorkFavorite();
  const toggleItemPlayed = useToggleItemPlayed();

  const workId = entry.seriesId ?? entry.id;
  const played = Boolean(entry.state?.played);
  const favorite = Boolean(entry.state?.favorite);
  const hasResume = Boolean(!played && (entry.state?.positionMs ?? 0) > 0);

  return (
    <Dropdown>
      <Dropdown.Trigger
        aria-label={t("videoApp.card.moreActions")}
        className="grid size-8 place-items-center rounded-full bg-black/75 text-white backdrop-blur-xs ring-1 ring-white/15 outline-none transition-colors hover:bg-black/90 focus-visible:ring-2 focus-visible:ring-white"
      >
        <MoreHorizontal className="size-4" aria-hidden />
      </Dropdown.Trigger>
      <Dropdown.Popover placement="bottom end" className="min-w-44">
        <Dropdown.Menu
          aria-label={entry.title}
          onAction={(key) => {
            const action = String(key);
            if (action === "play" && onPlay) {
              onPlay();
            } else if (action === "details") {
              void navigate({
                to: "/video/$libraryId/works/$workId",
                params: { libraryId: entry.libraryId, workId },
                search: entry.seasonNumber != null ? { season: entry.seasonNumber } : {},
              });
            } else if (action === "played") {
              if (isEpisode && entry.itemId) {
                toggleItemPlayed.mutate({ itemId: entry.itemId, played: !played });
              } else {
                toggleWorkPlayed.mutate({ libraryId: entry.libraryId, workId, played: !played });
              }
            } else if (action === "favorite") {
              toggleWorkFavorite.mutate({ libraryId: entry.libraryId, workId, favorite: !favorite });
            }
          }}
        >
          {onPlay ? (
            <Dropdown.Item id="play" textValue={t(hasResume ? "videoApp.card.resume" : "videoApp.card.play")}>
              <Play className="size-4 text-fg-muted" aria-hidden />
              <Label>{t(hasResume ? "videoApp.card.resume" : "videoApp.card.play")}</Label>
            </Dropdown.Item>
          ) : null}
          <Dropdown.Item id="details" textValue={t(isEpisode ? "videoApp.card.goToSeries" : "videoApp.card.details")}>
            <Info className="size-4 text-fg-muted" aria-hidden />
            <Label>{t(isEpisode ? "videoApp.card.goToSeries" : "videoApp.card.details")}</Label>
          </Dropdown.Item>
          <Dropdown.Item id="played" textValue={t(played ? "videoApp.card.markUnplayed" : "videoApp.card.markPlayed")}>
            {played ? <EyeOff className="size-4 text-fg-muted" aria-hidden /> : <Eye className="size-4 text-fg-muted" aria-hidden />}
            <Label>{t(played ? "videoApp.card.markUnplayed" : "videoApp.card.markPlayed")}</Label>
          </Dropdown.Item>
          {!isEpisode ? (
            <Dropdown.Item id="favorite" textValue={t(favorite ? "videoApp.card.unfavorite" : "videoApp.card.favorite")}>
              <Heart className={cn("size-4 text-fg-muted", favorite && "fill-current")} aria-hidden />
              <Label>{t(favorite ? "videoApp.card.unfavorite" : "videoApp.card.favorite")}</Label>
            </Dropdown.Item>
          ) : null}
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}

/** 2:3 Portrait Poster Card for movies and series. */
export function PosterCard({
  entry,
  className,
}: {
  entry: VideoEntry;
  className?: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const progress = watchProgress(entry);
  const played = Boolean(entry.state?.played);
  const unplayedCount = entry.kind === "series" ? (entry.state?.unplayedCount ?? null) : null;
  const favorite = Boolean(entry.state?.favorite);
  const posterPath = entry.posterItemId
    ? imagePaths.poster(entry.posterItemId, "work", 480)
    : entry.thumbItemId
      ? imagePaths.thumb(entry.thumbItemId, 480)
      : entry.itemId
        ? imagePaths.thumb(entry.itemId, 480)
        : null;
  const Icon = entry.kind === "series" ? Tv : Clapperboard;

  const workId = entry.seriesId ?? entry.id;

  const playNow = () => {
    if (entry.kind === "movie" && entry.itemId) {
      void navigate({
        to: "/video/play/$itemId",
        params: { itemId: entry.itemId },
        search: { libraryId: entry.libraryId, workId: entry.id },
      });
    } else {
      void navigate({
        to: "/video/$libraryId/works/$workId",
        params: { libraryId: entry.libraryId, workId },
      });
    }
  };

  const subtitleParts: string[] = [];
  if (entry.year != null) subtitleParts.push(String(entry.year));
  if (entry.communityRating != null) subtitleParts.push(entry.communityRating.toFixed(1));
  else if (entry.kind === "series" && entry.fileCount > 0) {
    subtitleParts.push(t("videoApp.card.episodesCount", { count: entry.fileCount }));
  }

  return (
    <article className={cn("group relative flex min-w-0 flex-col gap-2.5", className)}>
      <div className="relative aspect-[2/3] w-full overflow-hidden rounded-2xl bg-sunken shadow-art ring-1 ring-white/12 ring-inset transition-all duration-200 ease-out group-hover:-translate-y-1.5 group-hover:shadow-overlay group-hover:ring-white/25">
        <VideoImage
          path={posterPath}
          alt={entry.title}
          fallbackIcon={Icon}
          className="size-full"
          imgClassName="transition-transform duration-300 ease-out group-hover:scale-[1.03]"
        />

        {/* Stretched click area over poster (keyboard focus lands once on the title link below) */}
        <Link
          to="/video/$libraryId/works/$workId"
          params={{ libraryId: entry.libraryId, workId }}
          tabIndex={-1}
          aria-hidden="true"
          className="absolute inset-0 z-[1] rounded-tile outline-none"
        />

        {/* Subtle hover scrim + quick play button */}
        <div className="pointer-events-none absolute inset-0 z-[2] bg-gradient-to-t from-black/70 via-black/15 to-black/25 opacity-0 transition-opacity duration-200 group-focus-within:opacity-100 group-hover:opacity-100" />

        <button
          type="button"
          onClick={playNow}
          aria-label={`${t("videoApp.card.play")} ${entry.title}`}
          className="absolute top-1/2 left-1/2 z-[3] grid size-12 -translate-x-1/2 -translate-y-1/2 scale-95 place-items-center rounded-full bg-white/95 text-neutral-950 opacity-0 shadow-raised transition-all duration-200 group-focus-within:scale-100 group-focus-within:opacity-100 group-hover:scale-100 group-hover:opacity-100 hover:bg-white focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-white"
        >
          <Play className="ms-0.5 size-5 fill-current" aria-hidden />
        </button>

        {/* Top-left floating rating & favorite pills */}
        <div className="pointer-events-none absolute top-2 left-2 z-[2] flex items-center gap-1.5">
          {entry.communityRating != null ? (
            <span
              data-num
              className="inline-flex items-center gap-1 rounded-full bg-black/68 px-2 py-0.5 text-[0.7rem] leading-tight font-strong text-amber-300 tabular-nums ring-1 ring-white/15 backdrop-blur-md"
            >
              <Star className="size-3 fill-amber-400 text-amber-400" aria-hidden />
              {entry.communityRating.toFixed(1)}
            </span>
          ) : null}
          {favorite ? (
            <span
              aria-label={t("videoApp.detail.favorite")}
              className="grid size-6 place-items-center rounded-full bg-black/68 text-rose-400 ring-1 ring-white/15 backdrop-blur-md"
            >
              <Heart className="size-3.5 fill-current" aria-hidden />
            </span>
          ) : null}
        </div>

        {/* Top-right played / unplayed count badge */}
        <div className="pointer-events-none absolute top-2 right-2 z-[2] flex items-center gap-1">
          {unplayedCount !== null && unplayedCount > 0 ? (
            <span
              data-num
              aria-label={t("videoApp.card.unplayedCount", { count: unplayedCount })}
              className="grid min-w-6 place-items-center rounded-full bg-neutral-950/85 px-1.5 py-0.5 text-[0.7rem] leading-tight font-strong text-white tabular-nums backdrop-blur-xs ring-1 ring-white/20"
            >
              {unplayedCount}
            </span>
          ) : played ? (
            <span
              aria-label={t("videoApp.card.played")}
              className="grid size-6 place-items-center rounded-full bg-ok text-white shadow-sm ring-1 ring-white/25"
            >
              <Check className="size-3.5" strokeWidth={2.6} aria-hidden />
            </span>
          ) : null}
        </div>

        {/* Bottom-right quick menu */}
        <div className="absolute right-2 bottom-2.5 z-[3] opacity-0 transition-opacity duration-150 group-focus-within:opacity-100 group-hover:opacity-100">
          <CardMenu entry={entry} onPlay={playNow} />
        </div>

        {/* Bottom watch progress bar */}
        {progress !== null ? (
          <div className="pointer-events-none absolute inset-x-0 bottom-0 z-[2] h-1 bg-black/55">
            <div className="h-full bg-progress-done" style={{ width: `${Math.round(progress * 100)}%` }} />
          </div>
        ) : null}
      </div>

      {/* Crisp minimal footer */}
      <div className="grid min-w-0 gap-0.5 px-0.5">
        <Link
          to="/video/$libraryId/works/$workId"
          params={{ libraryId: entry.libraryId, workId }}
          className="truncate text-sm font-strong text-fg outline-none hover:text-heading focus-visible:underline"
        >
          {entry.title}
        </Link>
        {subtitleParts.length > 0 ? (
          <div className="flex items-center gap-1.5 truncate text-xs text-fg-muted tabular-nums">
            {entry.year != null ? <span>{entry.year}</span> : null}
            {entry.year != null && entry.communityRating != null ? <span aria-hidden>·</span> : null}
            {entry.communityRating != null ? (
              <span className="inline-flex items-center gap-0.5">
                <Star className="size-3 fill-amber-500 text-amber-500" aria-hidden />
                {entry.communityRating.toFixed(1)}
              </span>
            ) : entry.kind === "series" && entry.fileCount > 0 ? (
              <>
                {entry.year != null ? <span aria-hidden>·</span> : null}
                <span>{t("videoApp.card.episodesCount", { count: entry.fileCount })}</span>
              </>
            ) : null}
          </div>
        ) : null}
      </div>
    </article>
  );
}

/** 16:9 Landscape Card for Continue Watching, Next Up, and Latest Episodes. */
export function VideoWideCard({
  entry,
  variant = "continue",
  className,
}: {
  entry: VideoEntry;
  variant?: "continue" | "nextUp" | "latestEpisode";
  className?: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isEpisode = entry.kind === "episode";
  const workId = entry.seriesId ?? entry.id;
  const progress = watchProgress(entry);
  const left = remainingMs(entry);
  const epCode = formatEpisodeCode(entry.seasonNumber, entry.episodeNumber);
  const played = Boolean(entry.state?.played);

  const imagePath = entry.thumbItemId
    ? imagePaths.thumb(entry.thumbItemId, 640)
    : entry.backdropItemId
      ? imagePaths.backdrop(entry.backdropItemId, 640)
      : entry.posterItemId
        ? imagePaths.poster(entry.posterItemId, "work", 480)
        : entry.itemId
          ? imagePaths.thumb(entry.itemId, 640)
          : null;

  const playNow = () => {
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

  const primaryTitle = isEpisode && entry.seriesTitle ? entry.seriesTitle : entry.title;
  const leftStr = left ? formatRuntime(left, t, true) : null;
  const secondaryLine = isEpisode
    ? [epCode, entry.title].filter(Boolean).join(" · ")
    : leftStr && variant === "continue"
      ? t("videoApp.card.remaining", { time: leftStr })
      : [entry.year, formatRuntime(entry.runtimeMs, t, true)].filter(Boolean).join(" · ");

  const pillText =
    variant === "latestEpisode" && entry.fileCount > 1
      ? t("videoApp.card.newEpisodes", { count: entry.fileCount })
      : epCode ?? formatRuntime(entry.runtimeMs, t, true);

  return (
    <article className={cn("group relative flex min-w-0 flex-col gap-2.5", className)}>
      <div className="relative aspect-video w-full overflow-hidden rounded-tile bg-sunken shadow-art ring-1 ring-white/10 ring-inset transition-all duration-200 ease-out group-hover:-translate-y-1 group-hover:shadow-overlay">
        <VideoImage
          path={imagePath}
          alt={primaryTitle}
          fallbackIcon={isEpisode ? Tv : Clapperboard}
          className="size-full"
          imgClassName="transition-transform duration-300 ease-out group-hover:scale-[1.03]"
        />

        <div className="pointer-events-none absolute inset-0 z-[1] bg-gradient-to-t from-black/75 via-black/15 to-transparent" />

        {/* Stretched click plays immediately (or goes to work if no itemId) */}
        <button
          type="button"
          onClick={playNow}
          aria-label={`${t(variant === "continue" ? "videoApp.card.resume" : "videoApp.card.play")} ${primaryTitle}`}
          className="absolute inset-0 z-[2] rounded-tile outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-inset"
        />

        {/* Center play ring on hover */}
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-1/2 z-[3] grid size-11 -translate-x-1/2 -translate-y-1/2 scale-95 place-items-center rounded-full bg-white/95 text-neutral-950 opacity-0 shadow-raised transition-all duration-200 group-focus-within:scale-100 group-focus-within:opacity-100 group-hover:scale-100 group-hover:opacity-100"
        >
          <Play className="ms-0.5 size-5 fill-current" />
        </span>

        {/* Top-right quick menu */}
        <div className="absolute top-2 right-2 z-[4] opacity-0 transition-opacity duration-150 group-focus-within:opacity-100 group-hover:opacity-100">
          <CardMenu entry={entry} onPlay={playNow} isEpisode={isEpisode} />
        </div>

        {/* Bottom-left badge (episode code / new count / runtime) */}
        {pillText ? (
          <span className="pointer-events-none absolute bottom-2.5 left-2.5 z-[3] rounded-chip bg-black/75 px-2 py-0.5 text-[0.72rem] font-strong text-white tabular-nums backdrop-blur-xs ring-1 ring-white/15">
            {pillText}
          </span>
        ) : null}

        {/* Bottom-right remaining time or played check */}
        {variant === "continue" && leftStr && isEpisode ? (
          <span className="pointer-events-none absolute right-2.5 bottom-2.5 z-[3] rounded-chip bg-black/70 px-1.5 py-0.5 text-[0.7rem] text-white/90 tabular-nums backdrop-blur-xs">
            {t("videoApp.card.remaining", { time: leftStr })}
          </span>
        ) : played ? (
          <span
            aria-label={t("videoApp.card.played")}
            className="pointer-events-none absolute right-2.5 bottom-2.5 z-[3] grid size-5 place-items-center rounded-full bg-ok text-white"
          >
            <Check className="size-3" strokeWidth={2.6} aria-hidden />
          </span>
        ) : null}

        {/* Watch progress bar */}
        {progress !== null ? (
          <div className="pointer-events-none absolute inset-x-0 bottom-0 z-[3] h-1 bg-white/25">
            <div className="h-full bg-progress-done" style={{ width: `${Math.round(progress * 100)}%` }} />
          </div>
        ) : null}
      </div>

      {/* Footer */}
      <div className="grid min-w-0 gap-0.5 px-0.5">
        <Link
          to="/video/$libraryId/works/$workId"
          params={{ libraryId: entry.libraryId, workId }}
          search={entry.seasonNumber != null ? { season: entry.seasonNumber } : {}}
          className="truncate text-sm font-strong text-fg outline-none hover:text-heading focus-visible:underline"
        >
          {primaryTitle}
        </Link>
        {secondaryLine ? (
          <span className="truncate text-xs text-fg-muted">{secondaryLine}</span>
        ) : null}
      </div>
    </article>
  );
}
