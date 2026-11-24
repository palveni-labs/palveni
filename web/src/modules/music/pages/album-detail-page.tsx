import { Button, toast } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { BookOpen, Clapperboard, Disc3, Heart, Layers, ListEnd, ListPlus, ListStart, Play, Share2, Shuffle } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useRelatedWorks, type RelatedWork } from "@/api/queries/shares-collections";
import { cn } from "@/lib/cn";
import { useFeature } from "@/modules/access";
import { playerStore } from "@/player/store";
import { AddToCollectionDialog } from "@/ui/add-to-collection-dialog";
import { PageFrame } from "@/ui/frame";
import { ShareDialog } from "@/ui/share-dialog";
import { ErrorState, LoadingRows } from "@/ui/states";
import { Cover } from "@/ui/work";
import { musicApi, type MusicTrack } from "../api";
import { MusicArtwork } from "../artwork";
import { MusicHero } from "../components/music-hero";
import { PlaylistPickerDialog } from "../components/playlist-picker-dialog";
import { TrackRow } from "../components/track-row";
import { formatAudioSpecs, formatCollectionDuration } from "../format";
import { musicKeys, useToggleAlbumFavorite } from "../queries";

export function MusicAlbumDetailPage() {
  const { libraryId, albumId } = useParams({ from: "/_app/music/albums/$libraryId/$albumId" });
  const navigate = useNavigate();
  const { t, i18n } = useTranslation();
  const sharesEnabled = useFeature("shares");
  const collectionsEnabled = useFeature("collections");
  const toggleFav = useToggleAlbumFavorite();
  const [pickerIds, setPickerIds] = useState<string[] | null>(null);
  const [shareOpen, setShareOpen] = useState(false);
  const [collectionOpen, setCollectionOpen] = useState(false);

  const detailQuery = useQuery({
    queryKey: musicKeys.album(libraryId, albumId),
    queryFn: ({ signal }) => musicApi.album(libraryId, albumId, signal),
  });

  const relatedQuery = useRelatedWorks(detailQuery.data?.album.name, "music_album", collectionsEnabled);
  const relatedWorks = relatedQuery.data?.works ?? [];

  const tracks = useMemo(() => detailQuery.data?.tracks ?? [], [detailQuery.data]);
  const discGroups = useMemo(() => {
    const groups = new Map<number, MusicTrack[]>();
    for (const track of tracks) {
      const disc = track.discNumber ?? 1;
      const list = groups.get(disc);
      if (list) list.push(track);
      else groups.set(disc, [track]);
    }
    return [...groups.entries()].sort((a, b) => a[0] - b[0]);
  }, [tracks]);

  if (detailQuery.isPending) {
    return (
      <PageFrame>
        <LoadingRows rows={8} />
      </PageFrame>
    );
  }
  if (detailQuery.isError || !detailQuery.data) {
    return (
      <PageFrame>
        <ErrorState error={detailQuery.error} onRetry={() => void detailQuery.refetch()} />
      </PageFrame>
    );
  }

  const { album } = detailQuery.data;
  const albumTitle = album.name ?? t("musicApp.albums.unknown");
  const totalDuration = formatCollectionDuration(album.durationMs, i18n.resolvedLanguage ?? "zh-CN");
  const audioSpecs = tracks[0] ? formatAudioSpecs(tracks[0]) : null;
  const multiDisc = album.discCount > 1 || discGroups.length > 1;

  return (
    <PageFrame>
      <MusicHero
        artworkItemId={album.artworkItemId}
        artwork={
          <MusicArtwork
            itemId={album.artworkItemId}
            size={480}
            eager
            className="size-40 rounded-xl shadow-2xl ring-1 ring-white/20 sm:size-48"
          />
        }
        title={albumTitle}
        subtitle={
          album.artistId ? (
            <Link
              to="/music/artists/$libraryId/$artistId"
              params={{ libraryId: album.libraryId, artistId: album.artistId }}
              className="hover:underline"
            >
              {album.artist ?? t("musicApp.artists.unknown")}
            </Link>
          ) : (
            album.artist ?? t("musicApp.artists.unknown")
          )
        }
        meta={
          <>
            {album.year ? <span data-num className="tabular-nums">{album.year}</span> : null}
            {album.year ? <span aria-hidden>·</span> : null}
            <span data-num className="tabular-nums">{t("musicApp.albums.tracksCount", { count: album.trackCount })}</span>
            {totalDuration ? (
              <>
                <span aria-hidden>·</span>
                <span>{totalDuration}</span>
              </>
            ) : null}
            {audioSpecs ? (
              <>
                <span aria-hidden>·</span>
                <span className="rounded-chip bg-white/15 px-2 py-0.5 text-xs font-mid text-white">{audioSpecs}</span>
              </>
            ) : null}
          </>
        }
        extra={
          album.genres.length > 0 ? (
            <>
              {album.genres.map((genre) => (
                <Link
                  key={genre}
                  to="/music/albums"
                  search={{ genre }}
                  className="rounded-full bg-white/14 px-2.5 py-0.5 text-xs font-mid text-white/95 transition-colors hover:bg-white/24"
                >
                  {genre}
                </Link>
              ))}
            </>
          ) : null
        }
        actions={
          <>
            <Button
              size="sm"
              className="bg-white font-strong text-[#121316] hover:bg-white/90"
              isDisabled={tracks.length === 0}
              onPress={() => playerStore.playQueue(tracks, 0, { shuffle: false })}
            >
              <Play className="fill-current" aria-hidden />
              {t("musicApp.actions.playAll")}
            </Button>
            <Button
              size="sm"
              className="bg-white/16 text-white hover:bg-white/24"
              isDisabled={tracks.length === 0}
              onPress={() => playerStore.playShuffled(tracks)}
            >
              <Shuffle aria-hidden />
              {t("musicApp.actions.shuffle")}
            </Button>
            <Button
              size="sm"
              className="bg-white/14 text-white hover:bg-white/22"
              isDisabled={tracks.length === 0}
              onPress={() => {
                playerStore.playNext(tracks);
                toast.success(t("musicApp.actions.queuedNext"));
              }}
            >
              <ListStart aria-hidden />
              <span className="max-sm:sr-only">{t("musicApp.actions.playNext")}</span>
            </Button>
            <Button
              size="sm"
              className="bg-white/14 text-white hover:bg-white/22"
              isDisabled={tracks.length === 0}
              onPress={() => {
                playerStore.enqueue(tracks);
                toast.success(t("musicApp.actions.queuedCount", { count: tracks.length }));
              }}
            >
              <ListEnd aria-hidden />
              <span className="max-sm:sr-only">{t("musicApp.actions.addToQueue")}</span>
            </Button>
            <Button
              size="sm"
              className="bg-white/14 text-white hover:bg-white/22"
              isDisabled={tracks.length === 0}
              onPress={() => setPickerIds(tracks.map((tr) => tr.id))}
            >
              <ListPlus aria-hidden />
              <span className="max-sm:sr-only">{t("musicApp.actions.addToPlaylist")}</span>
            </Button>
            <Button
              size="sm"
              isIconOnly
              aria-label={album.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
              aria-pressed={album.favorite}
              className="bg-white/14 text-white hover:bg-white/22"
              onPress={() => toggleFav.mutate({ libraryId: album.libraryId, albumId: album.id, favorite: !album.favorite })}
            >
              <Heart className={cn("size-4", album.favorite && "fill-rose-400 text-rose-400")} aria-hidden />
            </Button>
            {sharesEnabled ? (
              <Button
                size="sm"
                className="bg-white/14 text-white hover:bg-white/22"
                onPress={() => setShareOpen(true)}
              >
                <Share2 className="size-4" aria-hidden />
                <span className="max-sm:sr-only">{t("sharesApp.button")}</span>
              </Button>
            ) : null}
            {collectionsEnabled ? (
              <Button
                size="sm"
                className="bg-white/14 text-white hover:bg-white/22"
                onPress={() => setCollectionOpen(true)}
              >
                <Layers className="size-4" aria-hidden />
                <span className="max-sm:sr-only">{t("collectionsApp.addToCollection")}</span>
              </Button>
            ) : null}
          </>
        }
      />

      {/* Disc-grouped tracklist */}
      <div className="grid gap-5">
        {discGroups.map(([discNumber, discTracks]) => (
          <section key={discNumber} className="grid gap-1">
            {multiDisc ? (
              <h2 className="flex items-center gap-2 border-b border-line px-2 pb-2 text-sm font-strong text-fg-muted">
                <Disc3 className="size-4 text-icon" aria-hidden />
                {t("musicApp.albums.disc", { number: discNumber })}
              </h2>
            ) : null}
            <div className="divide-y divide-line/60">
              {discTracks.map((track) => (
                <TrackRow
                  key={track.id}
                  track={track}
                  contextTracks={tracks}
                  showArtist={Boolean(track.artist && track.artist !== album.artist)}
                />
              ))}
            </div>
          </section>
        ))}

        {/* Cross-Media Related Works (Movies, Series, Books, Comics) */}
        {relatedWorks.length > 0 ? (
          <section className="grid gap-3 pt-2">
            <h2 className="font-heading text-base font-title text-heading">
              {t("collectionsApp.relatedRailTitle")}
            </h2>
            <div className="no-scrollbar flex gap-3.5 overflow-x-auto pb-1">
              {relatedWorks.map((rel: RelatedWork) => {
                const Icon = rel.entryKind === "video" ? Clapperboard : rel.entryKind === "music_album" ? Disc3 : BookOpen;
                const openRelated = () => {
                  if (rel.entryKind === "video") {
                    void navigate({
                      to: "/video/$libraryId/works/$workId",
                      params: { libraryId: rel.libraryId, workId: rel.targetId },
                    });
                  } else if (rel.entryKind === "book") {
                    void navigate({ to: "/reading/book/$itemId", params: { itemId: rel.targetId } });
                  } else if (rel.entryKind === "comic") {
                    void navigate({ to: "/reading/comic/$itemId", params: { itemId: rel.targetId } });
                  } else if (rel.entryKind === "music_album") {
                    void navigate({
                      to: "/music/albums/$libraryId/$albumId",
                      params: { libraryId: rel.libraryId, albumId: rel.targetId },
                    });
                  }
                };
                return (
                  <button
                    key={`${rel.entryKind}:${rel.libraryId}:${rel.targetId}`}
                    type="button"
                    onClick={openRelated}
                    className="group flex w-36 shrink-0 flex-col gap-2 rounded-tile text-start outline-none focus-visible:ring-2 focus-visible:ring-focus"
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
                        {[t(`collectionsApp.entryKinds.${rel.entryKind}`), rel.subtitle].filter(Boolean).join(" · ")}
                      </span>
                    </div>
                  </button>
                );
              })}
            </div>
          </section>
        ) : null}
      </div>

      <PlaylistPickerDialog trackIds={pickerIds} onClose={() => setPickerIds(null)} />
      {shareOpen ? (
        <ShareDialog
          isOpen
          onClose={() => setShareOpen(false)}
          targetKind="music_album"
          targetId={album.id}
          libraryId={album.libraryId}
          defaultTitle={albumTitle}
        />
      ) : null}
      <AddToCollectionDialog
        isOpen={collectionOpen}
        onClose={() => setCollectionOpen(false)}
        item={{
          entryKind: "music_album",
          targetId: album.id,
          libraryId: album.libraryId,
          title: albumTitle,
          subtitle: album.artist ?? null,
          artworkId: album.artworkItemId ? `ma_${album.artworkItemId}` : null,
        }}
      />
    </PageFrame>
  );
}
