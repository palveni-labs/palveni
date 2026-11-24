import { AlertDialog, Button, Input, Modal } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { useParams, useNavigate } from "@tanstack/react-router";
import { ArrowDown, ArrowUp, GripVertical, ListMusic, Pencil, Play, Plus, Share2, Shuffle, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useFeature } from "@/modules/access";
import { playerStore } from "@/player/store";
import { PageFrame } from "@/ui/frame";
import { ShareDialog } from "@/ui/share-dialog";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { musicApi, type MusicTrack, type Playlist } from "../api";
import { MusicArtwork, MusicMosaic } from "../artwork";
import { MusicHero } from "../components/music-hero";
import { PlaylistNameDialog } from "../components/playlist-picker-dialog";
import { TrackRow } from "../components/track-row";
import { formatCollectionDuration, formatTrackTime } from "../format";
import { useMusicLibraries } from "../library-store";
import {
  musicKeys,
  useDeletePlaylist,
  usePlaylist,
  usePlaylistHydratedTracks,
  useSavePlaylist,
} from "../queries";

function AddTracksDialog({
  playlist,
  isOpen,
  onOpenChange,
  onAdd,
}: {
  playlist: Playlist;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onAdd: (trackId: string) => void;
}) {
  const { t } = useTranslation();
  const { libraryId } = useMusicLibraries();
  const [q, setQ] = useState("");

  const tracksQuery = useQuery({
    queryKey: musicKeys.tracks(libraryId ?? "", { q: q.trim() || undefined, limit: 40 }),
    queryFn: ({ signal }) => musicApi.tracks(libraryId!, { q: q.trim() || undefined, limit: 40 }, signal),
    enabled: isOpen && Boolean(libraryId),
  });

  const existing = new Set(playlist.trackIds);

  return (
    <Modal.Backdrop isOpen={isOpen} onOpenChange={onOpenChange}>
      <Modal.Container placement="center" size="md">
        <Modal.Dialog>
          <Modal.Header>
            <Modal.Heading>{t("musicApp.playlists.addTracksTitle", { name: playlist.name })}</Modal.Heading>
          </Modal.Header>
          <Modal.Body className="grid gap-3">
            <Input
              autoFocus
              type="search"
              value={q}
              onChange={(event) => setQ(event.target.value)}
              placeholder={t("musicApp.playlists.searchTracksPlaceholder")}
              aria-label={t("musicApp.playlists.searchTracksPlaceholder")}
            />
            <div className="max-h-80 overflow-y-auto">
              {tracksQuery.isPending ? (
                <LoadingRows rows={4} />
              ) : (tracksQuery.data?.tracks.length ?? 0) === 0 ? (
                <p className="py-6 text-center text-sm text-fg-muted">{t("musicApp.songs.empty")}</p>
              ) : (
                <ul className="divide-y divide-line">
                  {tracksQuery.data!.tracks.map((track) => {
                    const already = existing.has(track.id);
                    return (
                      <li key={track.id} className="flex items-center gap-3 py-2">
                        <MusicArtwork itemId={track.artworkItemId ?? track.id} size={96} className="size-9 rounded-[calc(var(--pv-radius-tile)*0.7)]" />
                        <div className="grid min-w-0 flex-1">
                          <span className="truncate text-sm font-mid text-fg">{track.title}</span>
                          <span className="truncate text-xs text-fg-muted">
                            {track.artist ?? t("musicApp.artists.unknown")}{track.album ? ` · ${track.album}` : ""}
                          </span>
                        </div>
                        <span data-num className="text-xs text-fg-muted tabular-nums">{formatTrackTime(track.durationMs)}</span>
                        <Button
                          size="sm"
                          variant={already ? "ghost" : "outline"}
                          onPress={() => onAdd(track.id)}
                        >
                          <Plus className="size-3.5" aria-hidden />
                          {t("musicApp.playlists.addTracks")}
                        </Button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          </Modal.Body>
          <Modal.Footer>
            <Button variant="primary" onPress={() => onOpenChange(false)}>
              {t("common.done")}
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

export function MusicPlaylistDetailPage() {
  const { playlistId } = useParams({ from: "/_app/music/playlists/$playlistId" });
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const playlistQuery = usePlaylist(playlistId);
  const playlist = playlistQuery.data;
  const tracksQuery = usePlaylistHydratedTracks(playlist);
  const savePlaylist = useSavePlaylist();
  const deletePlaylist = useDeletePlaylist();

  const [renameOpen, setRenameOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const sharesEnabled = useFeature("shares");
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);

  if (playlistQuery.isPending) {
    return (
      <PageFrame>
        <LoadingRows rows={6} />
      </PageFrame>
    );
  }
  if (playlistQuery.isError || !playlist) {
    return (
      <PageFrame>
        <ErrorState error={playlistQuery.error} onRetry={() => void playlistQuery.refetch()} />
      </PageFrame>
    );
  }

  const tracks: MusicTrack[] = tracksQuery.data ?? [];
  const totalMs = tracks.reduce((sum, track) => sum + (track.durationMs ?? 0), 0);
  const totalDuration = formatCollectionDuration(totalMs, i18n.resolvedLanguage ?? "zh-CN");

  const persistIds = (nextIds: string[], nextName = playlist.name) => {
    savePlaylist.mutate({
      id: playlist.id,
      name: nextName,
      trackIds: nextIds,
      expectedVersion: playlist.version,
    });
  };

  const moveTrack = (from: number, to: number) => {
    if (from === to || from < 0 || to < 0 || from >= playlist.trackIds.length || to >= playlist.trackIds.length) return;
    const next = [...playlist.trackIds];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item!);
    persistIds(next);
  };

  const removeAt = (index: number) => {
    persistIds(playlist.trackIds.filter((_, i) => i !== index));
  };

  return (
    <PageFrame>
      <MusicHero
        artworkItemId={tracks[0]?.artworkItemId ?? tracks[0]?.id}
        artwork={
          <MusicMosaic
            itemIds={tracks.slice(0, 4).map((tr) => tr.artworkItemId ?? tr.id)}
            size={480}
            eager
            className="size-40 rounded-xl shadow-2xl ring-1 ring-white/20 sm:size-44"
          />
        }
        title={playlist.name}
        meta={
          <>
            <span data-num className="tabular-nums">{t("musicApp.playlists.tracksCount", { count: playlist.trackIds.length })}</span>
            {totalDuration ? (
              <>
                <span aria-hidden>·</span>
                <span>{totalDuration}</span>
              </>
            ) : null}
          </>
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
              onPress={() => setAddOpen(true)}
            >
              <Plus aria-hidden />
              {t("musicApp.playlists.addTracks")}
            </Button>
            <Button
              size="sm"
              className="bg-white/14 text-white hover:bg-white/22"
              onPress={() => setRenameOpen(true)}
            >
              <Pencil aria-hidden />
              <span className="max-sm:sr-only">{t("musicApp.playlists.rename")}</span>
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
            <Button
              size="sm"
              isIconOnly
              aria-label={t("musicApp.playlists.delete")}
              className="bg-white/14 text-white hover:bg-rose-500/80"
              onPress={() => setDeleteOpen(true)}
            >
              <Trash2 className="size-4" aria-hidden />
            </Button>
          </>
        }
      />

      {tracksQuery.isPending ? (
        <LoadingRows rows={5} />
      ) : tracks.length === 0 ? (
        <EmptyState
          icon={ListMusic}
          title={t("musicApp.playlists.emptyTracks")}
          detail={t("musicApp.playlists.emptyTracksHint")}
          action={
            <Button size="sm" variant="primary" onPress={() => setAddOpen(true)}>
              <Plus aria-hidden />
              {t("musicApp.playlists.addTracks")}
            </Button>
          }
        />
      ) : (
        <ol className="divide-y divide-line/60">
          {tracks.map((track, index) => {
            const isDragging = dragIndex === index;
            const isOver = overIndex === index && dragIndex !== null && dragIndex !== index;
            return (
              <li
                key={`${track.id}-${index}`}
                draggable
                onDragStart={(event) => {
                  setDragIndex(index);
                  event.dataTransfer.effectAllowed = "move";
                  event.dataTransfer.setData("text/plain", String(index));
                }}
                onDragOver={(event) => {
                  event.preventDefault();
                  if (overIndex !== index) setOverIndex(index);
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  if (dragIndex !== null && dragIndex !== index) moveTrack(dragIndex, index);
                  setDragIndex(null);
                  setOverIndex(null);
                }}
                onDragEnd={() => {
                  setDragIndex(null);
                  setOverIndex(null);
                }}
                className={cn(isDragging && "opacity-45", isOver && "rounded-tile ring-2 ring-action")}
              >
                <TrackRow
                  track={track}
                  indexLabel={index + 1}
                  contextTracks={tracks}
                  showCover
                  showAlbum
                  onRemove={() => removeAt(index)}
                  leadSlot={
                    <div className="flex shrink-0 items-center gap-0.5">
                      <span
                        aria-hidden
                        title={t("musicApp.playlists.dragHandle")}
                        className="cursor-grab text-fg-subtle active:cursor-grabbing"
                      >
                        <GripVertical className="size-4" />
                      </span>
                      <div className="hidden flex-col group-hover:flex group-focus-within:flex">
                        <button
                          type="button"
                          disabled={index === 0 || savePlaylist.isPending}
                          aria-label={t("musicApp.playlists.moveUp")}
                          onClick={() => moveTrack(index, index - 1)}
                          className="grid size-4 place-items-center rounded-sm text-fg-muted hover:text-fg disabled:opacity-30"
                        >
                          <ArrowUp className="size-3" aria-hidden />
                        </button>
                        <button
                          type="button"
                          disabled={index === tracks.length - 1 || savePlaylist.isPending}
                          aria-label={t("musicApp.playlists.moveDown")}
                          onClick={() => moveTrack(index, index + 1)}
                          className="grid size-4 place-items-center rounded-sm text-fg-muted hover:text-fg disabled:opacity-30"
                        >
                          <ArrowDown className="size-3" aria-hidden />
                        </button>
                      </div>
                    </div>
                  }
                />
              </li>
            );
          })}
        </ol>
      )}

      <AddTracksDialog
        playlist={playlist}
        isOpen={addOpen}
        onOpenChange={setAddOpen}
        onAdd={(trackId) => persistIds([...playlist.trackIds, trackId])}
      />

      <PlaylistNameDialog
        key={renameOpen ? `rename-${playlist.version}` : "rename-closed"}
        isOpen={renameOpen}
        onOpenChange={setRenameOpen}
        title={t("musicApp.playlists.renameTitle")}
        initialName={playlist.name}
        submitLabel={t("common.done")}
        isPending={savePlaylist.isPending}
        onSubmit={(name) => persistIds(playlist.trackIds, name)}
      />

      <AlertDialog.Backdrop isOpen={deleteOpen} onOpenChange={setDeleteOpen}>
        <AlertDialog.Container>
          <AlertDialog.Dialog>
            <AlertDialog.Header>
              <AlertDialog.Heading>{t("musicApp.playlists.delete")}</AlertDialog.Heading>
            </AlertDialog.Header>
            <AlertDialog.Body>
              <p className="text-sm text-fg-muted">
                {t("musicApp.playlists.deleteConfirm", { name: playlist.name })}
              </p>
            </AlertDialog.Body>
            <AlertDialog.Footer>
              <Button variant="ghost" onPress={() => setDeleteOpen(false)}>
                {t("common.cancel")}
              </Button>
              <Button
                variant="danger"
                isDisabled={deletePlaylist.isPending}
                onPress={() => {
                  deletePlaylist.mutate(
                    { id: playlist.id, version: playlist.version },
                    { onSuccess: () => void navigate({ to: "/music/playlists" }) },
                  );
                }}
              >
                {t("common.delete")}
              </Button>
            </AlertDialog.Footer>
          </AlertDialog.Dialog>
        </AlertDialog.Container>
      </AlertDialog.Backdrop>
      {shareOpen ? (
        <ShareDialog
          isOpen
          onClose={() => setShareOpen(false)}
          targetKind="playlist"
          targetId={playlist.id}
          defaultTitle={playlist.name}
        />
      ) : null}
    </PageFrame>
  );
}
