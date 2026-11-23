import { Button, buttonVariants, Input, Popover } from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import {
  BookAudio,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Disc3,
  FolderPlus,
  Gauge,
  Headphones,
  ListTree,
  Moon,
  Play,
  Podcast,
  RotateCcw,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { isApiError } from "@/api/errors";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import type { Library } from "@/api/types";
import { cn } from "@/lib/cn";
import { formatDuration } from "@/lib/format";
import { readLocal, writeLocal } from "@/lib/storage";
import { useFeature } from "@/modules/access";
import {
  formatPlaybackRate,
  PLAYBACK_RATES,
  playerStore,
  SLEEP_TIMER_MINUTES,
  usePlayer,
  type PlayableTrack,
} from "@/player/store";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { musicApi, type MediaState, type SpokenEntry, type SpokenRecording } from "../api";
import { MusicArtwork } from "../artwork";
import { formatSeconds } from "../format";
import { useMusicLibraries } from "../library-store";
import { musicKeys } from "../queries";

const SPOKEN_LIBRARY_KEY = "palveni.music.spokenLibraryId";
const SPOKEN_GENRE_PATTERN = /^(audiobooks?|audio\s*books?|spoken(\s*word)?|podcasts?|有声书|有声读物|评书|广播剧|播客)$/i;

type SpokenKind = "audiobook" | "podcast";

function isCollectionEntry(entry: SpokenEntry): entry is Extract<SpokenEntry, { kind: "book" | "show" }> {
  return entry.kind === "book" || entry.kind === "show";
}

function isRecordingEntry(entry: SpokenEntry): entry is Extract<SpokenEntry, { kind: "chapter" | "episode" }> {
  return entry.kind === "chapter" || entry.kind === "episode";
}

function toSpokenTrack(
  recording: SpokenRecording,
  libraryId: string,
  spokenKind: SpokenKind,
  collectionTitle?: string,
  state?: MediaState | null,
): PlayableTrack {
  return {
    id: recording.id,
    libraryId,
    title: recording.title,
    artist: recording.artist,
    albumArtist: recording.artist,
    album: collectionTitle ?? null,
    albumId: null,
    artistId: null,
    trackNumber: recording.number ?? null,
    discNumber: recording.discNumber ?? null,
    year: null,
    genres: [],
    durationMs: recording.durationMs ?? null,
    artworkItemId: recording.id,
    codec: null,
    bitrateKbps: null,
    sampleRateHz: null,
    bitDepth: null,
    channels: null,
    sizeBytes: 0,
    addedAt: "1970-01-01T00:00:00Z",
    state: {
      favorite: state?.favorite ?? false,
      stars: state?.stars ?? 0,
      playCount: state?.playCount ?? 0,
      lastPlayedAt: state?.lastPlayedAt ?? null,
    },
    spokenKind,
  };
}

function SpokenPlayerQuickControls() {
  const { t } = useTranslation();
  const player = usePlayer();
  const sleepActive = player.sleepUntilMs !== null || player.sleepEndOfTrack;
  const sleepBadgeText =
    player.sleepRemainingMinutes !== null
      ? t("musicApp.player.sleepRemainingBadge", { minutes: player.sleepRemainingMinutes })
      : player.sleepEndOfTrack
        ? t("musicApp.player.sleepEndBadge")
        : null;

  return (
    <div className="flex flex-wrap items-center gap-2">
      <div
        role="group"
        aria-label={t("musicApp.player.speed")}
        className="inline-flex items-center gap-0.5 rounded-control border border-line-control bg-surface p-0.5"
      >
        <Gauge className="ms-1.5 me-1 size-3.5 text-fg-muted" aria-hidden />
        {PLAYBACK_RATES.map((rate) => (
          <button
            key={rate}
            type="button"
            aria-pressed={player.playbackRate === rate}
            onClick={() => playerStore.setPlaybackRate(rate)}
            className={cn(
              "rounded-[calc(var(--pv-radius-control)-2px)] px-2 py-1 text-xs font-mid tabular-nums transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              player.playbackRate === rate ? "bg-action text-action-fg font-strong" : "text-fg-muted hover:bg-hover hover:text-fg",
            )}
          >
            {formatPlaybackRate(rate)}
          </button>
        ))}
      </div>

      <Popover>
        <Popover.Trigger
          aria-label={t("musicApp.player.sleepTimer")}
          className={cn(
            "inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border px-2.5 text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
            sleepActive
              ? "border-line-strong bg-selected/15 font-strong text-fg"
              : "border-line-control bg-surface text-fg-muted hover:bg-hover hover:text-fg",
          )}
        >
          <Moon className="size-3.5" aria-hidden />
          <span>{sleepBadgeText ?? t("musicApp.player.sleepTimer")}</span>
        </Popover.Trigger>
        <Popover.Content placement="bottom end" className="w-60 p-3">
          <Popover.Dialog aria-label={t("musicApp.player.sleepTimer")} className="grid gap-2.5 outline-none">
            <div className="grid gap-0.5">
              <p className="text-xs font-strong text-fg">{t("musicApp.player.sleepTimer")}</p>
              {player.sleepRemainingMinutes !== null ? (
                <p className="text-[0.7rem] text-fg-muted tabular-nums">
                  {t("musicApp.player.sleepRemainingStatus", { minutes: player.sleepRemainingMinutes })}
                </p>
              ) : player.sleepEndOfTrack ? (
                <p className="text-[0.7rem] text-fg-muted">{t("musicApp.player.sleepEndStatus")}</p>
              ) : null}
            </div>
            <div className="grid grid-cols-2 gap-1">
              <button
                type="button"
                aria-pressed={!sleepActive}
                onClick={() => playerStore.setSleepTimer("off")}
                className={cn(
                  "rounded-control px-2.5 py-1.5 text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  !sleepActive ? "bg-action text-action-fg font-strong" : "bg-sunken text-fg hover:bg-hover",
                )}
              >
                {t("musicApp.player.sleepOff")}
              </button>
              {SLEEP_TIMER_MINUTES.map((minutes) => {
                const selected =
                  player.sleepUntilMs !== null &&
                  player.sleepRemainingMinutes !== null &&
                  Math.abs(player.sleepRemainingMinutes - minutes) < 1;
                return (
                  <button
                    key={minutes}
                    type="button"
                    aria-pressed={selected}
                    onClick={() => playerStore.setSleepTimer(minutes)}
                    className={cn(
                      "rounded-control px-2.5 py-1.5 text-xs font-mid tabular-nums transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                      selected ? "bg-action text-action-fg font-strong" : "bg-sunken text-fg hover:bg-hover",
                    )}
                  >
                    {t("musicApp.player.sleepMinutes", { minutes })}
                  </button>
                );
              })}
              <button
                type="button"
                aria-pressed={player.sleepEndOfTrack}
                onClick={() => playerStore.setSleepTimer("end_of_track")}
                className={cn(
                  "col-span-2 rounded-control px-2.5 py-1.5 text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  player.sleepEndOfTrack ? "bg-action text-action-fg font-strong" : "bg-sunken text-fg hover:bg-hover",
                )}
              >
                {t("musicApp.player.sleepEndOfTrack")}
              </button>
            </div>
          </Popover.Dialog>
        </Popover.Content>
      </Popover>
    </div>
  );
}

function MusicSpokenGenreBanner() {
  const { t } = useTranslation();
  const { library: musicLibrary } = useMusicLibraries();
  const genresQuery = useQuery({
    queryKey: musicKeys.genres(musicLibrary?.id ?? ""),
    queryFn: ({ signal }) => musicApi.genres(musicLibrary!.id, signal),
    enabled: Boolean(musicLibrary),
    staleTime: 60_000,
  });

  const matchedGenres = useMemo(
    () => (genresQuery.data?.genres ?? []).filter((g) => SPOKEN_GENRE_PATTERN.test(g.name.trim())),
    [genresQuery.data],
  );

  if (matchedGenres.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center justify-between gap-3 rounded-panel border border-line bg-surface p-3.5">
      <div className="flex min-w-0 items-start gap-3">
        <span className="grid size-9 shrink-0 place-items-center rounded-tile bg-sunken text-fg">
          <Disc3 className="size-4" aria-hidden />
        </span>
        <div className="grid min-w-0 gap-0.5">
          <p className="text-sm font-strong text-fg">{t("musicApp.audiobooks.musicGenreBannerTitle")}</p>
          <p className="text-xs text-fg-muted">
            {t("musicApp.audiobooks.musicGenreBannerDetail", {
              genre: matchedGenres[0]!.name,
              count: matchedGenres[0]!.albumCount,
            })}
          </p>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {matchedGenres.map((genre) => (
          <Link
            key={genre.name}
            to="/music/albums"
            search={{ genre: genre.name }}
            className={buttonVariants({ variant: "outline", size: "sm" })}
          >
            {t("musicApp.audiobooks.openGenreAlbums", { genre: genre.name })}
          </Link>
        ))}
      </div>
    </div>
  );
}

function RecordingRow({
  recording,
  index,
  allTracks,
  libraryKind,
  autoLoadChapters,
}: {
  recording: SpokenRecording;
  index: number;
  allTracks: PlayableTrack[];
  libraryKind: SpokenKind;
  autoLoadChapters: boolean;
}) {
  const { t } = useTranslation();
  const player = usePlayer();
  const spokenChaptersEnabled = useFeature("spokenChapters");
  const [chaptersExpanded, setChaptersExpanded] = useState(autoLoadChapters);

  const stateQuery = useQuery({
    queryKey: ["music", "spoken-state", recording.id],
    queryFn: async ({ signal }): Promise<MediaState | null> => {
      try {
        return await musicApi.state(recording.id, signal);
      } catch (error) {
        if (isApiError(error) && error.status === 404) return null;
        throw error;
      }
    },
    staleTime: 15_000,
  });

  const shouldQueryChapters = libraryKind === "audiobook" && spokenChaptersEnabled && (autoLoadChapters || chaptersExpanded);
  const chaptersQuery = useQuery({
    queryKey: ["items", "spoken-chapters", recording.id],
    queryFn: ({ signal }) => musicApi.chapters(recording.id, signal),
    enabled: shouldQueryChapters,
    staleTime: 5 * 60_000,
    retry: false,
  });

  const mediaState = stateQuery.data;
  const savedPositionMs = mediaState && !mediaState.completed ? mediaState.positionMs : 0;
  const isCurrent = player.currentTrack?.id === recording.id;
  const liveSec = isCurrent ? player.currentTimeSec : savedPositionMs / 1000;
  const durationSec = (recording.durationMs ?? 0) / 1000;
  const progressPct = durationSec > 0 && liveSec > 0 ? Math.min(100, (liveSec / durationSec) * 100) : 0;
  const embeddedChapters = chaptersQuery.data?.chapters ?? [];

  const startAt = (startPositionSec: number) => {
    const hydratedTracks = allTracks.map((track) =>
      track.id === recording.id && mediaState
        ? {
            ...track,
            state: {
              favorite: mediaState.favorite,
              stars: mediaState.stars,
              playCount: mediaState.playCount,
              lastPlayedAt: mediaState.lastPlayedAt,
            },
          }
        : track,
    );
    if (isCurrent) {
      playerStore.seek(startPositionSec);
      if (!player.playing) playerStore.play();
      return;
    }
    playerStore.playQueue(hydratedTracks, index, { shuffle: false, startPositionSec });
  };

  return (
    <li className="grid gap-2 px-4 py-3.5 transition-colors hover:bg-hover/60">
      <div className="flex flex-wrap items-center gap-3">
        <span data-num className="w-7 shrink-0 text-sm font-mid text-fg-muted tabular-nums">
          {recording.number ?? index + 1}
        </span>

        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className={cn("break-words font-strong", isCurrent ? "text-link" : "text-fg")}>
              {recording.title}
            </span>
            {mediaState?.completed ? (
              <span className="inline-flex items-center gap-1 rounded-chip bg-sunken px-2 py-0.5 text-[0.7rem] font-mid text-fg-muted">
                <CheckCircle2 className="size-3" aria-hidden />
                {t("musicApp.audiobooks.completedBadge")}
              </span>
            ) : savedPositionMs > 0 ? (
              <span data-num className="inline-flex items-center rounded-chip bg-selected/15 px-2 py-0.5 text-[0.7rem] font-strong text-fg tabular-nums">
                {t("musicApp.audiobooks.inProgressBadge", { position: formatSeconds(savedPositionMs / 1000) })}
              </span>
            ) : null}
          </div>

          <div className="mt-0.5 flex flex-wrap items-center gap-2 text-xs text-fg-muted">
            {recording.artist ? <span>{recording.artist}</span> : null}
            {recording.durationMs != null ? (
              <span data-num className="tabular-nums">{formatDuration(recording.durationMs)}</span>
            ) : null}
          </div>

          {progressPct > 0 ? (
            <div aria-hidden className="mt-2 h-1 w-full max-w-sm overflow-hidden rounded-full bg-progress-rest">
              <div className="h-full bg-action transition-all" style={{ width: `${progressPct}%` }} />
            </div>
          ) : null}
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {libraryKind === "audiobook" && spokenChaptersEnabled && (!autoLoadChapters || embeddedChapters.length > 0) ? (
            <Button
              size="sm"
              variant="ghost"
              aria-expanded={chaptersExpanded}
              aria-label={chaptersExpanded ? t("musicApp.audiobooks.hideChapters") : t("musicApp.audiobooks.showChapters")}
              onPress={() => setChaptersExpanded((prev) => !prev)}
            >
              <ListTree className="size-3.5" aria-hidden />
              {embeddedChapters.length > 0
                ? t("musicApp.audiobooks.embeddedChapters", { count: embeddedChapters.length })
                : t("musicApp.audiobooks.chapters")}
            </Button>
          ) : null}

          {savedPositionMs > 0 ? (
            <>
              <Button
                size="sm"
                variant="primary"
                aria-label={`${t("musicApp.audiobooks.resumeAt", { position: formatSeconds(savedPositionMs / 1000) })} · ${recording.title}`}
                onPress={() => startAt(savedPositionMs / 1000)}
              >
                <Play className="size-3.5 fill-current" aria-hidden />
                {t("musicApp.audiobooks.resumeAt", { position: formatSeconds(savedPositionMs / 1000) })}
              </Button>
              <Button
                size="sm"
                variant="outline"
                aria-label={`${t("musicApp.audiobooks.playFromStart")} · ${recording.title}`}
                onPress={() => startAt(0)}
              >
                <RotateCcw className="size-3.5" aria-hidden />
                {t("musicApp.audiobooks.playFromStart")}
              </Button>
            </>
          ) : (
            <Button
              size="sm"
              variant={isCurrent && player.playing ? "primary" : "outline"}
              aria-label={`${t("musicApp.actions.play")} · ${recording.title}`}
              onPress={() => startAt(0)}
            >
              <Play className="size-3.5 fill-current" aria-hidden />
              {t("musicApp.actions.play")}
            </Button>
          )}
        </div>
      </div>

      {chaptersExpanded && embeddedChapters.length > 0 ? (
        <div className="ms-10 mt-1 grid gap-1 rounded-tile border border-line bg-sunken/55 p-2 max-sm:ms-0">
          <p className="px-2 py-0.5 text-xs font-strong text-fg-muted">
            {t("musicApp.audiobooks.embeddedChapters", { count: embeddedChapters.length })}
          </p>
          <ul className="grid gap-0.5 sm:grid-cols-2">
            {embeddedChapters.map((chapter, chapterIndex) => {
              const chapterTitle = chapter.title ?? t("musicApp.audiobooks.numberedChapter", { number: chapterIndex + 1 });
              const startSec = chapter.startMs / 1000;
              return (
                <li key={`${chapter.startMs}-${chapterIndex}`}>
                  <button
                    type="button"
                    aria-label={t("musicApp.audiobooks.jumpChapter", { title: chapterTitle })}
                    onClick={() => startAt(startSec)}
                    className="flex w-full items-center justify-between gap-2 rounded-control px-2.5 py-1.5 text-start text-xs transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                  >
                    <span className="truncate font-mid text-fg">{chapterTitle}</span>
                    <span data-num className="shrink-0 text-fg-muted tabular-nums">
                      {formatSeconds(startSec)}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}
    </li>
  );
}

function CollectionDetailView({
  library,
  collectionId,
  collectionTitle,
  onBack,
}: {
  library: Library;
  collectionId: string;
  collectionTitle: string;
  onBack: () => void;
}) {
  const { t } = useTranslation();
  const kind: SpokenKind = library.kind === "podcast" ? "podcast" : "audiobook";
  const Icon = kind === "podcast" ? Podcast : Headphones;

  const query = useInfiniteQuery({
    queryKey: ["music", "spoken", library.id, collectionId],
    queryFn: ({ pageParam, signal }) =>
      musicApi.spoken(library.id, { collection: collectionId, cursor: pageParam, limit: 100 }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const recordings = useMemo(
    () => (query.data?.pages.flatMap((p) => p.entries) ?? []).filter(isRecordingEntry),
    [query.data],
  );

  const allTracks = useMemo(
    () => recordings.map((rec) => toSpokenTrack(rec, library.id, kind, collectionTitle)),
    [recordings, library.id, kind, collectionTitle],
  );

  const leadId = recordings[0]?.id;
  const unitTitle = kind === "podcast" ? t("musicApp.audiobooks.episodes") : t("musicApp.audiobooks.chapters");

  return (
    <div className="grid gap-5">
      <nav aria-label={t("a11y.breadcrumb")} className="flex items-center gap-2 text-sm text-fg-muted">
        <button
          type="button"
          onClick={onBack}
          className="inline-flex items-center gap-1 rounded-control px-2 py-1 font-mid text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        >
          <ChevronLeft className="size-4" aria-hidden />
          {t("musicApp.audiobooks.backToShelf")}
        </button>
        <ChevronRight className="size-4 shrink-0" aria-hidden />
        <span aria-current="page" className="truncate text-fg">{collectionTitle}</span>
      </nav>

      <div className="flex flex-wrap items-center gap-5 rounded-panel border border-line bg-surface p-5">
        {leadId ? (
          <MusicArtwork itemId={leadId} size={240} className="size-24 shrink-0 rounded-tile shadow-panel sm:size-28" />
        ) : (
          <div className="grid size-24 shrink-0 place-items-center rounded-tile bg-sunken text-fg-muted sm:size-28">
            <Icon className="size-9" aria-hidden />
          </div>
        )}
        <div className="grid min-w-0 flex-1 gap-2">
          <div className="flex flex-wrap items-center gap-2 text-xs font-mid text-fg-muted">
            <span className="inline-flex items-center gap-1 rounded-chip bg-sunken px-2 py-0.5">
              <Icon className="size-3" aria-hidden />
              {kind === "podcast" ? t("musicApp.audiobooks.kindPodcast") : t("musicApp.audiobooks.kindAudiobook")}
            </span>
            <span>{library.name}</span>
          </div>
          <h2 className="font-heading text-xl font-title text-heading sm:text-2xl">{collectionTitle}</h2>
          <div className="flex flex-wrap items-center gap-2 pt-1">
            <Button
              size="sm"
              variant="primary"
              isDisabled={allTracks.length === 0}
              onPress={() => playerStore.playQueue(allTracks, 0, { shuffle: false, startPositionSec: 0 })}
            >
              <Play className="size-4 fill-current" aria-hidden />
              {t("musicApp.audiobooks.playAll")}
            </Button>
          </div>
        </div>
      </div>

      <section aria-label={unitTitle} className="grid gap-3">
        <h3 className="font-heading text-base font-title text-heading">{unitTitle}</h3>
        {query.isPending ? (
          <LoadingRows rows={5} />
        ) : query.isError && recordings.length === 0 ? (
          <ErrorState error={query.error} onRetry={() => void query.refetch()} />
        ) : recordings.length === 0 ? (
          <EmptyState icon={Icon} title={t("musicApp.audiobooks.emptyCollection")} />
        ) : (
          <ul className="divide-y divide-line overflow-hidden rounded-panel border border-line bg-surface">
            {recordings.map((recording, index) => (
              <RecordingRow
                key={recording.id}
                recording={recording}
                index={index}
                allTracks={allTracks}
                libraryKind={kind}
                autoLoadChapters={recordings.length <= 5}
              />
            ))}
          </ul>
        )}

        {query.hasNextPage ? (
          <div>
            <Button
              size="sm"
              variant="outline"
              isDisabled={query.isFetchingNextPage}
              onPress={() => void query.fetchNextPage()}
            >
              {t("musicApp.audiobooks.loadMore")}
            </Button>
          </div>
        ) : null}
      </section>
    </div>
  );
}

function ShelfView({
  library,
  onOpenCollection,
}: {
  library: Library;
  onOpenCollection: (collectionId: string, title: string) => void;
}) {
  const { t } = useTranslation();
  const [q, setQ] = useState("");
  const kind: SpokenKind = library.kind === "podcast" ? "podcast" : "audiobook";
  const Icon = kind === "podcast" ? Podcast : BookAudio;
  const shelfTitle = kind === "podcast" ? t("musicApp.audiobooks.showsShelf") : t("musicApp.audiobooks.booksShelf");

  const query = useInfiniteQuery({
    queryKey: ["music", "spoken", library.id, null],
    queryFn: ({ pageParam, signal }) => musicApi.spoken(library.id, { cursor: pageParam, limit: 60 }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const entries = useMemo(() => query.data?.pages.flatMap((p) => p.entries) ?? [], [query.data]);
  const needle = q.trim().toLowerCase();

  const collections = useMemo(
    () => entries.filter(isCollectionEntry).filter((c) => !needle || c.title.toLowerCase().includes(needle)),
    [entries, needle],
  );
  const looseRecordings = useMemo(
    () => entries.filter(isRecordingEntry).filter((r) => !needle || r.title.toLowerCase().includes(needle)),
    [entries, needle],
  );
  const looseTracks = useMemo(
    () => looseRecordings.map((r) => toSpokenTrack(r, library.id, kind, library.name)),
    [looseRecordings, library.id, kind, library.name],
  );

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          value={q}
          onChange={(event) => setQ(event.target.value)}
          aria-label={t("musicApp.audiobooks.searchPlaceholder")}
          placeholder={t("musicApp.audiobooks.searchPlaceholder")}
          className="h-(--pv-control-h-sm) w-60 max-sm:w-full"
        />
        <span className="flex-1" />
        {collections.length > 0 ? (
          <span data-num className="text-xs text-fg-muted tabular-nums max-sm:hidden">
            {t("musicApp.audiobooks.collectionsCount", { count: collections.length })}
          </span>
        ) : null}
      </div>

      {query.isPending ? (
        <LoadingRows rows={6} />
      ) : query.isError && entries.length === 0 ? (
        <ErrorState error={query.error} onRetry={() => void query.refetch()} />
      ) : collections.length === 0 && looseRecordings.length === 0 ? (
        <EmptyState
          icon={Icon}
          title={needle ? t("musicApp.audiobooks.emptySearch") : t("musicApp.audiobooks.emptyCollection")}
        />
      ) : (
        <div className="grid gap-5">
          {collections.length > 0 ? (
            <section aria-label={shelfTitle} className="grid gap-3">
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {collections.map((entry) => (
                  <button
                    key={entry.id}
                    type="button"
                    onClick={() => onOpenCollection(entry.id, entry.title)}
                    className="group flex min-w-0 items-center gap-3.5 rounded-panel border border-line bg-surface p-4 text-start transition-colors hover:border-line-strong hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                  >
                    <span className="grid size-12 shrink-0 place-items-center rounded-tile bg-sunken text-fg-muted transition-colors group-hover:text-fg">
                      <Icon className="size-6" aria-hidden />
                    </span>
                    <span className="grid min-w-0 flex-1 gap-0.5">
                      <span className="line-clamp-2 font-heading text-base font-title text-heading">{entry.title}</span>
                      <span className="text-xs text-fg-muted">
                        {entry.kind === "show" ? t("musicApp.audiobooks.kindPodcast") : t("musicApp.audiobooks.kindAudiobook")}
                      </span>
                    </span>
                    <ChevronRight className="size-4 shrink-0 text-fg-muted transition-transform group-hover:translate-x-0.5" aria-hidden />
                  </button>
                ))}
              </div>
            </section>
          ) : null}

          {looseRecordings.length > 0 ? (
            <ul className="divide-y divide-line overflow-hidden rounded-panel border border-line bg-surface">
              {looseRecordings.map((recording, index) => (
                <RecordingRow
                  key={recording.id}
                  recording={recording}
                  index={index}
                  allTracks={looseTracks}
                  libraryKind={kind}
                  autoLoadChapters={looseRecordings.length <= 5}
                />
              ))}
            </ul>
          ) : null}
        </div>
      )}

      {query.hasNextPage ? (
        <div>
          <Button
            size="sm"
            variant="outline"
            isDisabled={query.isFetchingNextPage}
            onPress={() => void query.fetchNextPage()}
          >
            {t("musicApp.audiobooks.loadMore")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

export function MusicAudiobooksPage() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const navigate = useNavigate();
  const search = useSearch({ from: "/_app/music/audiobooks" });
  const librariesQuery = useLibraries();
  const [persistedId, setPersistedId] = useState<string | null>(() => readLocal(SPOKEN_LIBRARY_KEY));

  const spokenLibraries = useMemo(
    () => (librariesQuery.data?.libraries ?? []).filter((lib) => lib.kind === "audiobook" || lib.kind === "podcast"),
    [librariesQuery.data],
  );

  const activeLibrary = useMemo(() => {
    if (spokenLibraries.length === 0) return null;
    if (search.library) {
      const bySearch = spokenLibraries.find((lib) => lib.id === search.library);
      if (bySearch) return bySearch;
    }
    if (persistedId) {
      const bySaved = spokenLibraries.find((lib) => lib.id === persistedId);
      if (bySaved) return bySaved;
    }
    return spokenLibraries[0] ?? null;
  }, [spokenLibraries, search.library, persistedId]);

  const selectSpokenLibrary = (libraryId: string) => {
    setPersistedId(libraryId);
    writeLocal(SPOKEN_LIBRARY_KEY, libraryId);
    void navigate({
      to: "/music/audiobooks",
      search: { library: libraryId },
    });
  };

  return (
    <PageFrame>
      <PageHeader title={t("musicApp.audiobooks.title")} />

      <div className="grid gap-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          {spokenLibraries.length > 1 && activeLibrary ? (
            <div
              role="group"
              aria-label={t("musicApp.audiobooks.switchLibrary")}
              className="flex flex-wrap items-center gap-1.5"
            >
              {spokenLibraries.map((lib) => {
                const selected = lib.id === activeLibrary.id;
                const LibIcon = lib.kind === "podcast" ? Podcast : BookAudio;
                return (
                  <button
                    key={lib.id}
                    type="button"
                    aria-pressed={selected}
                    onClick={() => selectSpokenLibrary(lib.id)}
                    className={cn(
                      "inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border px-3 text-sm font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                      selected
                        ? "border-action bg-action text-action-fg font-strong"
                        : "border-line-control bg-surface text-fg hover:bg-hover",
                    )}
                  >
                    <LibIcon className="size-3.5" aria-hidden />
                    <span>{lib.name}</span>
                  </button>
                );
              })}
            </div>
          ) : (
            <span />
          )}

          <SpokenPlayerQuickControls />
        </div>

        <MusicSpokenGenreBanner />

        {librariesQuery.isPending ? (
          <LoadingRows rows={6} />
        ) : !activeLibrary ? (
          <EmptyState
            icon={BookAudio}
            title={t("musicApp.audiobooks.empty")}
            detail={isAdmin ? t("musicApp.audiobooks.emptyAdmin") : t("musicApp.audiobooks.emptyUser")}
            action={
              isAdmin ? (
                <Link
                  to="/system/libraries"
                  search={{ create: true }}
                  className={buttonVariants({ variant: "primary", size: "sm" })}
                >
                  <FolderPlus aria-hidden />
                  {t("musicApp.audiobooks.manageLibraries")}
                </Link>
              ) : null
            }
          />
        ) : search.collection ? (
          <CollectionDetailView
            key={`${activeLibrary.id}:${search.collection}`}
            library={activeLibrary}
            collectionId={search.collection}
            collectionTitle={search.title || t("musicApp.audiobooks.chapters")}
            onBack={() =>
              void navigate({
                to: "/music/audiobooks",
                search: { library: activeLibrary.id },
              })
            }
          />
        ) : (
          <ShelfView
            key={activeLibrary.id}
            library={activeLibrary}
            onOpenCollection={(collectionId, title) =>
              void navigate({
                to: "/music/audiobooks",
                search: { library: activeLibrary.id, collection: collectionId, title },
              })
            }
          />
        )}
      </div>
    </PageFrame>
  );
}
