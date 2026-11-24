import { Button, buttonVariants, toast } from "@heroui/react";
import { useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams, useSearch } from "@tanstack/react-router";
import {
  ArrowLeft,
  BookOpen,
  Check,
  Clapperboard,
  Disc3,
  Download,
  Film,
  Heart,
  Layers,
  PencilLine,
  Play,
  RotateCcw,
  Share2,
  Star,
  Tv,
  User,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useRelatedWorks, type RelatedWork } from "@/api/queries/shares-collections";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import { useFeature } from "@/modules/access";
import { AddToCollectionDialog } from "@/ui/add-to-collection-dialog";
import { MetadataEditModal } from "@/ui/metadata-edit-modal";
import { ToolbarTitle } from "@/ui/page";
import { ShareDialog } from "@/ui/share-dialog";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { Cover } from "@/ui/work";
import { downloadOriginalVideo, imagePaths, type MediaInfo, type VideoEntry, type VideoPerson } from "../api";
import { MediaRail } from "../components/media-rail";
import { PosterCard } from "../components/poster-card";
import { VideoImage } from "../components/video-image";
import {
  codecLabel,
  formatBitrate,
  formatClock,
  formatEpisodeCode,
  formatRuntime,
  hdrLabel,
  personInitials,
  remainingMs,
  resolutionLabel,
  watchProgress,
} from "../format";
import {
  useMediaInfo,
  useToggleItemPlayed,
  useToggleWorkFavorite,
  useToggleWorkPlayed,
  useVideoEpisodes,
  useVideoWork,
  videoKeys,
} from "../queries";

function MediaSpecsStrip({ info }: { info: MediaInfo }) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "zh-CN";

  const videoBadges: string[] = [];
  if (info.video) {
    videoBadges.push(resolutionLabel(info.video));
    const hdr = hdrLabel(info.video.hdr);
    if (hdr) videoBadges.push(hdr);
    videoBadges.push(codecLabel(info.video.codec));
    if (info.video.bitDepth && info.video.bitDepth > 8) videoBadges.push(`${info.video.bitDepth}-bit`);
    if (info.video.frameRate) videoBadges.push(`${Number(info.video.frameRate.toFixed(2))} fps`);
  }

  const audioSummary = info.audio
    .slice(0, 3)
    .map((track) =>
      [
        track.title ?? track.language?.toUpperCase(),
        codecLabel(track.codec),
        track.channelLayout ?? (track.channels ? `${track.channels}ch` : null),
      ]
        .filter(Boolean)
        .join(" "),
    );

  const subSummary = info.subtitles
    .slice(0, 4)
    .map((s) => s.title ?? s.language?.toUpperCase() ?? s.codec.toUpperCase());

  const fileParts = [
    info.container?.toUpperCase(),
    formatBitrate(info.bitrateKbps),
    info.sizeBytes != null ? formatBytes(info.sizeBytes, locale) : null,
  ].filter(Boolean);

  if (!videoBadges.length && !audioSummary.length && !fileParts.length) return null;

  return (
    <section aria-label={t("videoApp.detail.mediaInfo")} className="grid gap-2.5">
      <div className="flex flex-wrap items-center gap-x-6 gap-y-2.5 rounded-tile border border-line bg-surface px-4 py-3 text-sm">
        {videoBadges.length > 0 ? (
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-xs font-mid text-fg-muted">{t("videoApp.detail.videoSpec")}</span>
            {videoBadges.map((badge) => (
              <span
                key={badge}
                className="rounded-chip bg-sunken px-2 py-0.5 text-xs font-strong text-fg tabular-nums"
              >
                {badge}
              </span>
            ))}
          </div>
        ) : null}

        {audioSummary.length > 0 ? (
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-xs font-mid text-fg-muted">{t("videoApp.detail.audioSpec")}</span>
            <span className="text-xs font-mid text-fg">{audioSummary.join(" / ")}</span>
          </div>
        ) : null}

        {subSummary.length > 0 ? (
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-xs font-mid text-fg-muted">{t("videoApp.detail.subtitleSpec")}</span>
            <span className="text-xs font-mid text-fg">{subSummary.join(" · ")}</span>
          </div>
        ) : null}

        {fileParts.length > 0 ? (
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-xs font-mid text-fg-muted">{t("videoApp.detail.fileSpec")}</span>
            <span data-num className="font-mono text-xs text-fg-muted tabular-nums">
              {fileParts.join(" · ")}
            </span>
          </div>
        ) : null}
      </div>
    </section>
  );
}

function EpisodeRow({
  episode,
  libraryId,
  workId,
}: {
  episode: VideoEntry;
  libraryId: string;
  workId: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const togglePlayed = useToggleItemPlayed();

  const progress = watchProgress(episode);
  const left = remainingMs(episode);
  const played = Boolean(episode.state?.played);
  const epNum = episode.episodeNumber != null ? String(episode.episodeNumber).padStart(2, "0") : null;
  const thumbPath = episode.thumbItemId
    ? imagePaths.thumb(episode.thumbItemId, 480)
    : episode.backdropItemId
      ? imagePaths.backdrop(episode.backdropItemId, 480)
      : episode.itemId
        ? imagePaths.thumb(episode.itemId, 480)
        : null;

  const playEpisode = () => {
    if (!episode.itemId) return;
    void navigate({
      to: "/video/play/$itemId",
      params: { itemId: episode.itemId },
      search: {
        libraryId,
        workId,
        season: episode.seasonNumber ?? undefined,
      },
    });
  };

  return (
    <li className="group flex flex-col gap-3 rounded-tile border border-line bg-surface p-3 transition-colors hover:bg-hover/60 sm:flex-row sm:items-center sm:gap-4">
      <button
        type="button"
        onClick={playEpisode}
        aria-label={`${t("videoApp.card.play")} ${episode.title}`}
        className="relative aspect-video w-full shrink-0 overflow-hidden rounded-control bg-sunken ring-1 ring-line/60 outline-none focus-visible:ring-2 focus-visible:ring-focus sm:w-48 md:w-52"
      >
        <VideoImage
          path={thumbPath}
          fallbackIcon={Tv}
          className="size-full"
          imgClassName="transition-transform duration-300 group-hover:scale-105"
        />
        <span className="pointer-events-none absolute inset-0 bg-black/20 opacity-0 transition-opacity group-hover:opacity-100" />
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-1/2 grid size-10 -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full bg-white/95 text-neutral-950 opacity-0 shadow-raised transition-opacity group-focus-visible:opacity-100 group-hover:opacity-100"
        >
          <Play className="ms-0.5 size-4 fill-current" />
        </span>
        {epNum ? (
          <span
            data-num
            className="pointer-events-none absolute bottom-2 left-2 rounded-chip bg-black/75 px-1.5 py-0.5 text-[0.7rem] font-strong text-white tabular-nums backdrop-blur-xs"
          >
            E{epNum}
          </span>
        ) : null}
        {played ? (
          <span
            aria-label={t("videoApp.card.played")}
            className="pointer-events-none absolute top-2 right-2 grid size-5 place-items-center rounded-full bg-ok text-white"
          >
            <Check className="size-3" strokeWidth={2.6} aria-hidden />
          </span>
        ) : null}
        {progress !== null ? (
          <span className="pointer-events-none absolute inset-x-0 bottom-0 h-1 bg-white/25">
            <span className="block h-full bg-white" style={{ width: `${Math.round(progress * 100)}%` }} />
          </span>
        ) : null}
      </button>

      <div className="grid min-w-0 flex-1 gap-1.5">
        <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
          <button
            type="button"
            onClick={playEpisode}
            className="truncate text-start text-base font-strong text-fg outline-none hover:text-heading hover:underline focus-visible:underline"
          >
            {epNum ? `${epNum} · ` : ""}
            {episode.title}
          </button>
          <div className="flex items-center gap-2 text-xs text-fg-muted tabular-nums">
            {episode.runtimeMs ? <span>{formatRuntime(episode.runtimeMs, t, true)}</span> : null}
            {left ? (
              <>
                <span aria-hidden>·</span>
                <span>{t("videoApp.card.remaining", { time: formatRuntime(left, t, true) })}</span>
              </>
            ) : null}
          </div>
        </div>
        {episode.overview ? (
          <p className="line-clamp-2 text-sm text-fg-muted">{episode.overview}</p>
        ) : null}
      </div>

      <div className="flex shrink-0 items-center gap-1.5 self-end sm:self-center">
        <Button size="sm" variant="outline" onPress={playEpisode}>
          <Play className="size-3.5 fill-current" aria-hidden />
          {progress !== null ? t("videoApp.card.resume") : t("videoApp.card.play")}
        </Button>
        {episode.itemId ? (
          <Button
            size="sm"
            variant={played ? "secondary" : "ghost"}
            isIconOnly
            aria-pressed={played}
            aria-label={t(played ? "videoApp.card.markUnplayed" : "videoApp.card.markPlayed")}
            onPress={() => togglePlayed.mutate({ itemId: episode.itemId!, played: !played })}
          >
            <Check className={cn("size-4", played ? "text-ok" : "text-fg-muted")} aria-hidden />
          </Button>
        ) : null}
      </div>
    </li>
  );
}

function CastRail({ libraryId, people }: { libraryId: string; people: VideoPerson[] }) {
  const { t } = useTranslation();
  if (people.length === 0) return null;

  const roleText = (person: VideoPerson) => {
    if (person.role) return person.role;
    if (person.kind === "director") return t("videoApp.detail.director");
    if (person.kind === "writer") return t("videoApp.detail.writer");
    if (person.kind === "producer") return t("videoApp.detail.producer");
    return null;
  };

  return (
    <MediaRail title={t("videoApp.detail.castAndCrew")}>
      {people.map((person, index) => (
        <Link
          key={`${person.kind}-${person.name}-${index}`}
          to="/video/$libraryId"
          params={{ libraryId }}
          search={{ person: person.name }}
          className="group flex w-28 shrink-0 snap-start flex-col gap-2 rounded-tile outline-none focus-visible:ring-2 focus-visible:ring-(--pv-focus) sm:w-32"
        >
          <VideoImage
            path={person.imageUrl ? imagePaths.person(person.imageUrl, 240) : null}
            alt={person.name}
            fallbackIcon={User}
            fallbackText={personInitials(person.name)}
            className="aspect-[5/6] w-full rounded-tile ring-1 ring-line/70 transition-shadow group-hover:shadow-raised"
            imgClassName="transition-transform duration-300 group-hover:scale-[1.04]"
          />
          <div className="grid min-w-0 gap-0.5 px-0.5">
            <span className="truncate text-sm font-strong text-fg group-hover:text-heading group-hover:underline">
              {person.name}
            </span>
            {roleText(person) ? (
              <span className="truncate text-xs text-fg-muted">{roleText(person)}</span>
            ) : null}
          </div>
        </Link>
      ))}
    </MediaRail>
  );
}

export function VideoDetailPage() {
  const { libraryId, workId } = useParams({ from: "/_app/video/$libraryId/works/$workId" });
  const search = useSearch({ from: "/_app/video/$libraryId/works/$workId" });
  return <VideoDetailView key={`${libraryId}/${workId}`} libraryId={libraryId} workId={workId} initialSeason={search.season} />;
}

function VideoDetailView({
  libraryId,
  workId,
  initialSeason,
}: {
  libraryId: string;
  workId: string;
  initialSeason?: number;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const isAdmin = useIsAdmin();

  const detailQuery = useVideoWork(libraryId, workId);
  const togglePlayed = useToggleWorkPlayed();
  const toggleFavorite = useToggleWorkFavorite();

  const detail = detailQuery.data;
  const work = detail?.work;
  const seasons = useMemo(() => detail?.seasons ?? [], [detail]);
  const files = useMemo(() => detail?.files ?? [], [detail]);

  const defaultSeason = useMemo(() => {
    if (initialSeason != null && seasons.some((s) => s.seasonNumber === initialSeason)) {
      return initialSeason;
    }
    if (detail?.nextUp?.seasonNumber != null) return detail.nextUp.seasonNumber;
    return seasons[0]?.seasonNumber ?? null;
  }, [initialSeason, seasons, detail]);

  const [pickedSeason, setPickedSeason] = useState<number | null>(null);
  const [pickedFileId, setPickedFileId] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const [metadataOpen, setMetadataOpen] = useState(false);
  const [collectionOpen, setCollectionOpen] = useState(false);
  const sharesEnabled = useFeature("shares");
  const collectionsEnabled = useFeature("collections");
  const relatedQuery = useRelatedWorks(work?.title, "video", collectionsEnabled);
  const relatedWorks = relatedQuery.data?.works ?? [];
  const selectedSeason = pickedSeason ?? defaultSeason;
  const selectedFileId = pickedFileId ?? files[0]?.itemId ?? work?.itemId ?? null;

  const episodesQuery = useVideoEpisodes(
    libraryId,
    workId,
    selectedSeason,
    work?.kind === "series" && selectedSeason !== null,
  );

  const primaryItemId =
    work?.kind === "movie"
      ? (selectedFileId ?? work.itemId)
      : (detail?.nextUp?.itemId ?? episodesQuery.data?.[0]?.itemId ?? null);

  const mediaInfoQuery = useMediaInfo(primaryItemId, Boolean(primaryItemId));

  if (detailQuery.isPending) {
    return (
      <div className="pv-page-frame mx-auto w-full max-w-(--pv-layout-max) px-(--pv-layout-gutter) pt-(--pv-layout-top) pb-(--pv-layout-bottom)">
        <LoadingRows rows={6} />
      </div>
    );
  }

  if (detailQuery.isError) {
    return (
      <div className="pv-page-frame mx-auto w-full max-w-(--pv-layout-max) px-(--pv-layout-gutter) pt-(--pv-layout-top) pb-(--pv-layout-bottom)">
        <ErrorState error={detailQuery.error} onRetry={() => void detailQuery.refetch()} />
      </div>
    );
  }

  if (!detail || !work) {
    return (
      <div className="pv-page-frame mx-auto w-full max-w-(--pv-layout-max) px-(--pv-layout-gutter) pt-(--pv-layout-top) pb-(--pv-layout-bottom)">
        <EmptyState
          icon={Film}
          title={t("videoApp.detail.notFound")}
          detail={t("videoApp.detail.notFoundDetail")}
          action={
            <Link
              to="/video/$libraryId"
              params={{ libraryId }}
              className={buttonVariants({ variant: "outline", size: "sm" })}
            >
              {t("videoApp.detail.back")}
            </Link>
          }
        />
      </div>
    );
  }

  const played = Boolean(work.state?.played);
  const favorite = Boolean(work.state?.favorite);
  const fallbackThumbId = work.thumbItemId ?? primaryItemId;
  const backdropPath = work.backdropItemId
    ? imagePaths.backdrop(work.backdropItemId, 1920)
    : fallbackThumbId
      ? imagePaths.thumb(fallbackThumbId, 960)
      : null;
  const posterPath = work.posterItemId
    ? imagePaths.poster(work.posterItemId, "work", 480)
    : fallbackThumbId
      ? imagePaths.thumb(fallbackThumbId, 480)
      : null;

  const isSeries = work.kind === "series";
  const nextUp = detail.nextUp;
  const movieHasResume = Boolean(!isSeries && !played && (work.state?.positionMs ?? 0) > 0);
  const nextUpHasResume = Boolean(nextUp && !nextUp.state?.played && (nextUp.state?.positionMs ?? 0) > 0);
  const nextUpCode = formatEpisodeCode(nextUp?.seasonNumber, nextUp?.episodeNumber);

  const playTarget = (fromStart = false) => {
    const targetItem = primaryItemId;
    if (!targetItem) return;
    void navigate({
      to: "/video/play/$itemId",
      params: { itemId: targetItem },
      search: {
        libraryId,
        workId: work.id,
        season: isSeries ? (nextUp?.seasonNumber ?? selectedSeason ?? undefined) : undefined,
        fromStart: fromStart || undefined,
      },
    });
  };

  const playButtonLabel = isSeries
    ? nextUp && nextUpCode
      ? nextUpHasResume && nextUp.state
        ? t("videoApp.detail.resumeEpisode", { code: nextUpCode, time: formatClock(nextUp.state.positionMs) })
        : t("videoApp.detail.playEpisode", { code: nextUpCode })
      : t("videoApp.detail.play")
    : movieHasResume && work.state
      ? t("videoApp.detail.resumeAt", { time: formatClock(work.state.positionMs) })
      : t("videoApp.detail.play");

  const runtimeStr = formatRuntime(work.runtimeMs, t, false);

  return (
    <div className="pv-enter flex w-full flex-col pb-16">
      <ToolbarTitle title={work.title} />
      {/* Full-bleed cinema hero with high-contrast directional scrims */}
      <section className="relative isolate w-full overflow-hidden bg-neutral-950 text-white">
        <div className="absolute inset-0 -z-10">
          <VideoImage
            path={backdropPath}
            alt={work.title}
            eager
            className="size-full bg-neutral-950"
            imgClassName="size-full object-cover object-center"
          />
        </div>
        <div
          className="relative w-full"
          style={{
            backgroundImage:
              "linear-gradient(90deg, rgba(6,8,12,0.92) 0%, rgba(6,8,12,0.78) 54%, rgba(6,8,12,0.45) 100%), linear-gradient(0deg, rgba(6,8,12,0.95) 0%, rgba(6,8,12,0.45) 55%, rgba(6,8,12,0.35) 100%)",
          }}
        >
          <div className="mx-auto flex w-full max-w-(--pv-layout-max) flex-col gap-6 px-(--pv-layout-gutter) pt-5 pb-8 sm:pt-7 sm:pb-10">
            <div>
              <Link
                to="/video/$libraryId"
                params={{ libraryId }}
                className="inline-flex h-8 items-center gap-1.5 rounded-control bg-white/12 px-3 text-sm font-mid text-white backdrop-blur-xs transition-colors hover:bg-white/22 focus-visible:outline-2 focus-visible:outline-white"
              >
                <ArrowLeft className="size-4" aria-hidden />
                {t("videoApp.detail.back")}
              </Link>
            </div>

            <div className="flex flex-col items-start gap-6 sm:flex-row sm:items-end sm:gap-8">
              {/* Crisp 2:3 Poster */}
              <div className="w-36 shrink-0 overflow-hidden rounded-tile shadow-raised ring-1 ring-white/20 sm:w-48 lg:w-56">
                <VideoImage
                  path={posterPath}
                  alt={work.title}
                  eager
                  fallbackIcon={isSeries ? Tv : Clapperboard}
                  className="aspect-[2/3] w-full bg-neutral-900"
                />
              </div>

              {/* Metadata & Primary Actions */}
              <div className="grid min-w-0 flex-1 gap-3">
                <div className="grid gap-1">
                  <h1 className="font-heading text-2xl leading-tight font-title text-white sm:text-4xl">
                    {work.title}
                  </h1>
                  {work.originalTitle && work.originalTitle !== work.title ? (
                    <p className="text-sm font-mid text-white/75 sm:text-base">{work.originalTitle}</p>
                  ) : null}
                </div>

                {/* Metadata row */}
                <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1.5 text-sm text-white/85">
                  {work.year != null ? <span data-num className="font-mid">{work.year}</span> : null}
                  {runtimeStr ? (
                    <>
                      <span aria-hidden>·</span>
                      <span>{runtimeStr}</span>
                    </>
                  ) : null}
                  {work.communityRating != null ? (
                    <>
                      <span aria-hidden>·</span>
                      <span className="inline-flex items-center gap-1 font-strong text-white tabular-nums">
                        <Star className="size-4 fill-amber-400 text-amber-400" aria-hidden />
                        {work.communityRating.toFixed(1)}
                      </span>
                    </>
                  ) : null}
                  {work.officialRating ? (
                    <span className="rounded-sm border border-white/35 px-1.5 py-0.5 text-xs leading-none font-strong tracking-wide text-white">
                      {work.officialRating}
                    </span>
                  ) : null}
                  {detail.studios.length > 0 ? (
                    <>
                      <span aria-hidden>·</span>
                      <span className="text-white/75">{detail.studios.join(" / ")}</span>
                    </>
                  ) : null}
                </div>

                {/* Genre pills */}
                {work.genres.length > 0 ? (
                  <div className="flex flex-wrap items-center gap-1.5">
                    {work.genres.map((genre) => (
                      <Link
                        key={genre}
                        to="/video/$libraryId"
                        params={{ libraryId }}
                        search={{ genre }}
                        className="rounded-chip bg-white/14 px-2.5 py-0.5 text-xs font-mid text-white/95 backdrop-blur-xs transition-colors hover:bg-white/24"
                      >
                        {genre}
                      </Link>
                    ))}
                  </div>
                ) : null}

                {/* Tagline */}
                {work.tagline ? (
                  <p className="text-sm font-mid text-amber-200/95 italic sm:text-base">
                    “{work.tagline}”
                  </p>
                ) : null}

                {/* Overview */}
                {work.overview ? (
                  <p className="max-w-3xl text-sm leading-relaxed text-white/85 sm:text-base">
                    {work.overview}
                  </p>
                ) : null}

                {/* Actions */}
                <div className="mt-1 flex flex-wrap items-center gap-2.5">
                  {primaryItemId ? (
                    <button
                      type="button"
                      onClick={() => playTarget(false)}
                      className="inline-flex h-10 items-center gap-2 rounded-control bg-white px-5 text-sm font-strong text-neutral-950 shadow-raised transition-colors hover:bg-neutral-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                    >
                      <Play className="size-4 fill-current" aria-hidden />
                      {playButtonLabel}
                    </button>
                  ) : null}

                  {movieHasResume || nextUpHasResume ? (
                    <button
                      type="button"
                      onClick={() => playTarget(true)}
                      className="inline-flex h-10 items-center gap-2 rounded-control border border-white/25 bg-white/14 px-4 text-sm font-mid text-white backdrop-blur-xs transition-colors hover:bg-white/24 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                    >
                      <RotateCcw className="size-4" aria-hidden />
                      {t("videoApp.detail.restart")}
                    </button>
                  ) : null}

                  {files.length > 1 ? (
                    <select
                      aria-label={t("videoApp.detail.version")}
                      value={selectedFileId ?? files[0]!.itemId}
                      onChange={(e) => setPickedFileId(e.target.value)}
                      className="h-10 rounded-control border border-white/25 bg-neutral-900/90 px-3 text-sm text-white focus-visible:outline-2 focus-visible:outline-white"
                    >
                      {files.map((file) => (
                        <option key={file.itemId} value={file.itemId}>
                          {file.title}
                        </option>
                      ))}
                    </select>
                  ) : null}

                  <button
                    type="button"
                    aria-pressed={played}
                    onClick={() => togglePlayed.mutate({ libraryId, workId: work.id, played: !played })}
                    className={cn(
                      "inline-flex h-10 items-center gap-1.5 rounded-control border px-3.5 text-sm font-mid backdrop-blur-xs transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white",
                      played
                        ? "border-emerald-400/50 bg-emerald-500/25 text-emerald-100 hover:bg-emerald-500/35"
                        : "border-white/25 bg-white/14 text-white hover:bg-white/24",
                    )}
                  >
                    <Check className="size-4" aria-hidden />
                    {t(played ? "videoApp.detail.played" : "videoApp.detail.markPlayed")}
                  </button>

                  <button
                    type="button"
                    aria-pressed={favorite}
                    onClick={() => toggleFavorite.mutate({ libraryId, workId: work.id, favorite: !favorite })}
                    className={cn(
                      "inline-flex h-10 items-center gap-1.5 rounded-control border px-3.5 text-sm font-mid backdrop-blur-xs transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white",
                      favorite
                        ? "border-rose-400/50 bg-rose-500/25 text-rose-100 hover:bg-rose-500/35"
                        : "border-white/25 bg-white/14 text-white hover:bg-white/24",
                    )}
                  >
                    <Heart className={cn("size-4", favorite && "fill-current")} aria-hidden />
                    {t(favorite ? "videoApp.detail.favorite" : "videoApp.detail.addFavorite")}
                  </button>

                  {sharesEnabled ? (
                    <button
                      type="button"
                      onClick={() => setShareOpen(true)}
                      className="inline-flex h-10 items-center gap-1.5 rounded-control border border-white/25 bg-white/14 px-3.5 text-sm font-mid text-white backdrop-blur-xs transition-colors hover:bg-white/24 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                    >
                      <Share2 className="size-4" aria-hidden />
                      {t("sharesApp.button")}
                    </button>
                  ) : null}

                  {collectionsEnabled ? (
                    <button
                      type="button"
                      onClick={() => setCollectionOpen(true)}
                      className="inline-flex h-10 items-center gap-1.5 rounded-control border border-white/25 bg-white/14 px-3.5 text-sm font-mid text-white backdrop-blur-xs transition-colors hover:bg-white/24 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                    >
                      <Layers className="size-4" aria-hidden />
                      {t("collectionsApp.addToCollection")}
                    </button>
                  ) : null}

                  {isAdmin && primaryItemId ? (
                    <button
                      type="button"
                      onClick={() => setMetadataOpen(true)}
                      className="inline-flex h-10 items-center gap-1.5 rounded-control border border-white/25 bg-white/14 px-3.5 text-sm font-mid text-white backdrop-blur-xs transition-colors hover:bg-white/24 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                    >
                      <PencilLine className="size-4" aria-hidden />
                      {t("metadataModal.trigger")}
                    </button>
                  ) : null}

                  {!isSeries && primaryItemId ? (
                    <button
                      type="button"
                      disabled={downloading}
                      onClick={() => {
                        if (downloading) return;
                        setDownloading(true);
                        void downloadOriginalVideo(primaryItemId)
                          .catch((err) => toast.danger(errorText(t, err)))
                          .finally(() => setDownloading(false));
                      }}
                      className="inline-flex h-10 items-center gap-1.5 rounded-control border border-white/25 bg-white/14 px-3.5 text-sm font-mid text-white backdrop-blur-xs transition-colors hover:bg-white/24 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white disabled:opacity-60"
                    >
                      <Download className="size-4" aria-hidden />
                      {t("videoApp.detail.download")}
                    </button>
                  ) : null}
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Main Content below Hero */}
      <div className="pv-page-frame mx-auto flex w-full max-w-(--pv-layout-max) flex-col gap-8 px-(--pv-layout-gutter) pt-7 max-md:gap-6">
        {mediaInfoQuery.data ? <MediaSpecsStrip info={mediaInfoQuery.data} /> : null}

        {/* Seasons & Episodes */}
        {isSeries && seasons.length > 0 ? (
          <section aria-label={t("videoApp.detail.seasons")} className="grid gap-4">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="me-2 font-heading text-lg font-title text-heading">
                {t("videoApp.detail.seasons")}
              </h2>
              <div role="tablist" aria-label={t("videoApp.detail.seasons")} className="flex flex-wrap items-center gap-1.5">
                {seasons.map((season) => {
                  const sNum = season.seasonNumber ?? 1;
                  const active = selectedSeason === sNum;
                  const unplayed = season.state?.unplayedCount ?? 0;
                  const label =
                    sNum === 0
                      ? t("videoApp.detail.specials")
                      : t("videoApp.detail.season", { number: sNum });
                  return (
                    <button
                      key={season.id}
                      type="button"
                      role="tab"
                      aria-selected={active}
                      onClick={() => setPickedSeason(sNum)}
                      className={cn(
                        "inline-flex h-8 items-center gap-1.5 rounded-control px-3 text-sm font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                        active
                          ? "bg-action font-strong text-action-fg"
                          : "border border-line bg-surface text-fg-muted hover:bg-hover hover:text-fg",
                      )}
                    >
                      <span>{label}</span>
                      {unplayed > 0 ? (
                        <span
                          data-num
                          className={cn(
                            "rounded-full px-1.5 py-0.5 text-[0.7rem] leading-none font-strong tabular-nums",
                            active ? "bg-black/25 text-action-fg" : "bg-sunken text-fg-muted",
                          )}
                        >
                          {unplayed}
                        </span>
                      ) : null}
                    </button>
                  );
                })}
              </div>
            </div>

            {episodesQuery.isPending ? (
              <LoadingRows rows={4} />
            ) : episodesQuery.isError ? (
              <ErrorState compact error={episodesQuery.error} onRetry={() => void episodesQuery.refetch()} />
            ) : (
              <ol className="grid gap-2.5">
                {(episodesQuery.data ?? []).map((episode) => (
                  <EpisodeRow
                    key={episode.id}
                    episode={episode}
                    libraryId={libraryId}
                    workId={work.id}
                  />
                ))}
              </ol>
            )}
          </section>
        ) : null}

        {/* Cast & Crew */}
        <CastRail libraryId={libraryId} people={detail.people} />

        {/* Cross-Media Related Works (Soundtracks, Books, Comics) */}
        {relatedWorks.length > 0 ? (
          <MediaRail title={t("collectionsApp.relatedRailTitle")} count={relatedWorks.length}>
            {relatedWorks.map((rel: RelatedWork) => {
              const Icon = rel.entryKind === "music_album" ? Disc3 : BookOpen;
              const openRelated = () => {
                if (rel.entryKind === "music_album") {
                  void navigate({
                    to: "/music/albums/$libraryId/$albumId",
                    params: { libraryId: rel.libraryId, albumId: rel.targetId },
                  });
                } else if (rel.entryKind === "book") {
                  void navigate({
                    to: "/reading/book/$itemId",
                    params: { itemId: rel.targetId },
                  });
                } else if (rel.entryKind === "comic") {
                  void navigate({
                    to: "/reading/comic/$itemId",
                    params: { itemId: rel.targetId },
                  });
                }
              };
              return (
                <button
                  key={`${rel.entryKind}:${rel.libraryId}:${rel.targetId}`}
                  type="button"
                  onClick={openRelated}
                  className="group flex w-36 shrink-0 snap-start flex-col gap-2 rounded-tile text-start outline-none focus-visible:ring-2 focus-visible:ring-(--pv-focus) sm:w-40"
                >
                  <Cover
                    artworkId={rel.artworkId}
                    title={rel.title}
                    icon={Icon}
                    decorative
                    className="aspect-square w-full rounded-tile ring-1 ring-line/70 transition-shadow group-hover:shadow-raised"
                  />
                  <div className="grid min-w-0 gap-0.5 px-0.5">
                    <span className="truncate text-sm font-strong text-fg group-hover:text-heading group-hover:underline">
                      {rel.title}
                    </span>
                    <span className="truncate text-xs text-fg-muted">
                      {[t(`collectionsApp.entryKinds.${rel.entryKind}`), rel.subtitle]
                        .filter(Boolean)
                        .join(" · ")}
                    </span>
                  </div>
                </button>
              );
            })}
          </MediaRail>
        ) : null}

        {/* Similar Titles */}
        {detail.similar.length > 0 ? (
          <MediaRail title={t("videoApp.detail.similar")} count={detail.similar.length}>
            {detail.similar.map((sim) => (
              <PosterCard
                key={sim.id}
                entry={sim}
                className="w-36 shrink-0 snap-start sm:w-40 lg:w-44"
              />
            ))}
          </MediaRail>
        ) : null}
      </div>

      <ShareDialog
        isOpen={shareOpen}
        onClose={() => setShareOpen(false)}
        targetKind="video_work"
        targetId={work.id}
        libraryId={libraryId}
        defaultTitle={work.title}
      />

      <AddToCollectionDialog
        isOpen={collectionOpen}
        onClose={() => setCollectionOpen(false)}
        item={{
          entryKind: "video",
          targetId: work.id,
          libraryId,
          title: work.title,
          subtitle: work.year != null ? String(work.year) : null,
          artworkId: work.posterItemId ? `vp_${work.posterItemId}` : null,
        }}
      />

      {isAdmin && primaryItemId ? (
        <MetadataEditModal
          isOpen={metadataOpen}
          onClose={() => setMetadataOpen(false)}
          itemId={primaryItemId}
          workKey={work.id}
          defaultMediaType={isSeries ? "tv" : "movie"}
          initialValues={{
            title: work.title,
            originalTitle: work.originalTitle,
            year: work.year,
            rating: work.communityRating,
            genres: work.genres,
            overview: work.overview,
          }}
          onSaved={() => {
            void queryClient.invalidateQueries({ queryKey: videoKeys.work(libraryId, workId) });
            void queryClient.invalidateQueries({ queryKey: videoKeys.all });
          }}
          onReset={() => {
            void queryClient.invalidateQueries({ queryKey: videoKeys.work(libraryId, workId) });
            void queryClient.invalidateQueries({ queryKey: videoKeys.all });
          }}
        />
      ) : null}
    </div>
  );
}
