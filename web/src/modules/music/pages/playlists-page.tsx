import { AlertDialog, Button, Dropdown, Label } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ListMusic, MoreHorizontal, Pencil, Play, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { playerStore } from "@/player/store";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { lookupTracks, musicApi, type PlaylistSummary } from "../api";
import { MusicMosaic } from "../artwork";
import { PlaylistNameDialog } from "../components/playlist-picker-dialog";
import { musicKeys, useCreatePlaylist, useDeletePlaylist, usePlaylists, useSavePlaylist } from "../queries";

function PlaylistCard({
  summary,
  onRename,
  onDelete,
}: {
  summary: PlaylistSummary;
  onRename: (summary: PlaylistSummary, trackIds: string[]) => void;
  onDelete: (summary: PlaylistSummary) => void;
}) {
  const { t } = useTranslation();
  const detailQuery = useQuery({
    queryKey: musicKeys.playlist(summary.id),
    queryFn: ({ signal }) => musicApi.playlists.get(summary.id, signal),
    staleTime: 60_000,
  });
  const trackIds = detailQuery.data?.trackIds ?? [];

  const playPlaylist = async () => {
    const ids = trackIds.length > 0 ? trackIds : (await musicApi.playlists.get(summary.id)).trackIds;
    if (ids.length === 0) return;
    const tracks = await lookupTracks(ids);
    if (tracks.length > 0) playerStore.playQueue(tracks, 0, { shuffle: false });
  };

  return (
    <article className="group flex min-w-0 flex-col gap-2">
      <div className="relative aspect-square w-full overflow-hidden rounded-tile shadow-art ring-1 ring-line/50">
        <Link
          to="/music/playlists/$playlistId"
          params={{ playlistId: summary.id }}
          aria-hidden="true"
          tabIndex={-1}
          className="block size-full outline-none"
        >
          <MusicMosaic itemIds={trackIds.slice(0, 4)} size={480} className="size-full rounded-none transition-transform duration-300 group-hover:scale-[1.03]" />
        </Link>
        <div className="pointer-events-none absolute inset-x-0 bottom-0 flex items-end justify-between bg-gradient-to-t from-black/55 via-black/15 to-transparent p-2.5 opacity-0 transition-opacity duration-200 group-hover:opacity-100 group-focus-within:opacity-100">
          <button
            type="button"
            disabled={trackIds.length === 0}
            onClick={(event) => {
              event.preventDefault();
              void playPlaylist();
            }}
            aria-label={`${t("musicApp.actions.play")} ${summary.name}`}
            className="pointer-events-auto grid size-10 place-items-center rounded-full bg-white text-[#121316] shadow-overlay transition-transform hover:scale-105 disabled:opacity-50"
          >
            <Play className="ms-0.5 size-4.5 fill-current" aria-hidden />
          </button>
          <div className="pointer-events-auto">
            <Dropdown>
              <Dropdown.Trigger
                aria-label={t("musicApp.actions.more")}
                className="grid size-8 place-items-center rounded-full bg-black/55 text-white backdrop-blur-sm hover:bg-black/75"
              >
                <MoreHorizontal className="size-4" aria-hidden />
              </Dropdown.Trigger>
              <Dropdown.Popover placement="bottom end" className="min-w-40">
                <Dropdown.Menu
                  aria-label={summary.name}
                  onAction={(key) => {
                    if (key === "rename") onRename(summary, trackIds);
                    if (key === "delete") onDelete(summary);
                  }}
                >
                  <Dropdown.Item id="rename" textValue={t("musicApp.playlists.rename")}>
                    <Pencil className="size-4 text-fg-muted" aria-hidden />
                    <Label>{t("musicApp.playlists.rename")}</Label>
                  </Dropdown.Item>
                  <Dropdown.Item id="delete" variant="danger" textValue={t("musicApp.playlists.delete")}>
                    <Trash2 className="size-4 text-failed" aria-hidden />
                    <Label>{t("musicApp.playlists.delete")}</Label>
                  </Dropdown.Item>
                </Dropdown.Menu>
              </Dropdown.Popover>
            </Dropdown>
          </div>
        </div>
      </div>
      <div className="grid min-w-0 gap-0.5 px-0.5">
        <Link
          to="/music/playlists/$playlistId"
          params={{ playlistId: summary.id }}
          className="truncate text-sm font-strong text-fg hover:underline"
        >
          {summary.name}
        </Link>
        <span data-num className="text-xs text-fg-muted tabular-nums">
          {detailQuery.data ? t("musicApp.playlists.tracksCount", { count: detailQuery.data.trackIds.length }) : "…"}
        </span>
      </div>
    </article>
  );
}

export function MusicPlaylistsPage() {
  const { t } = useTranslation();
  const playlistsQuery = usePlaylists();
  const createPlaylist = useCreatePlaylist();
  const savePlaylist = useSavePlaylist();
  const deletePlaylist = useDeletePlaylist();

  const [createOpen, setCreateOpen] = useState(false);
  const [renaming, setRenaming] = useState<{ summary: PlaylistSummary; trackIds: string[] } | null>(null);
  const [deleting, setDeleting] = useState<PlaylistSummary | null>(null);

  const playlists = playlistsQuery.data?.playlists ?? [];

  return (
    <PageFrame>
      <PageHeader
        title={t("musicApp.playlists.title")}
        actions={
          <Button size="sm" variant="primary" onPress={() => setCreateOpen(true)}>
            <Plus aria-hidden />
            {t("musicApp.playlists.create")}
          </Button>
        }
      />

      {playlistsQuery.isPending ? (
        <LoadingRows rows={4} />
      ) : playlistsQuery.isError ? (
        <ErrorState error={playlistsQuery.error} onRetry={() => void playlistsQuery.refetch()} />
      ) : playlists.length === 0 ? (
        <EmptyState
          icon={ListMusic}
          title={t("musicApp.playlists.empty")}
          detail={t("musicApp.playlists.emptyHint")}
          action={
            <Button size="sm" variant="primary" onPress={() => setCreateOpen(true)}>
              <Plus aria-hidden />
              {t("musicApp.playlists.create")}
            </Button>
          }
        />
      ) : (
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
          {playlists.map((playlist) => (
            <PlaylistCard
              key={playlist.id}
              summary={playlist}
              onRename={(summary, trackIds) => setRenaming({ summary, trackIds })}
              onDelete={(summary) => setDeleting(summary)}
            />
          ))}
        </div>
      )}

      <PlaylistNameDialog
        key={createOpen ? "create-open" : "create-closed"}
        isOpen={createOpen}
        onOpenChange={setCreateOpen}
        title={t("musicApp.playlists.createTitle")}
        submitLabel={t("common.create")}
        isPending={createPlaylist.isPending}
        onSubmit={async (name) => {
          await createPlaylist.mutateAsync({ name });
        }}
      />

      <PlaylistNameDialog
        key={renaming?.summary.id ?? "rename-closed"}
        isOpen={Boolean(renaming)}
        onOpenChange={(open) => { if (!open) setRenaming(null); }}
        title={t("musicApp.playlists.renameTitle")}
        initialName={renaming?.summary.name ?? ""}
        submitLabel={t("common.done")}
        isPending={savePlaylist.isPending}
        onSubmit={async (name) => {
          if (!renaming) return;
          const fresh = await musicApi.playlists.get(renaming.summary.id);
          await savePlaylist.mutateAsync({
            id: fresh.id,
            name,
            trackIds: fresh.trackIds,
            expectedVersion: fresh.version,
          });
        }}
      />

      <AlertDialog.Backdrop isOpen={Boolean(deleting)} onOpenChange={(open) => { if (!open) setDeleting(null); }}>
        <AlertDialog.Container>
          <AlertDialog.Dialog>
            <AlertDialog.Header>
              <AlertDialog.Heading>{t("musicApp.playlists.delete")}</AlertDialog.Heading>
            </AlertDialog.Header>
            <AlertDialog.Body>
              <p className="text-sm text-fg-muted">
                {t("musicApp.playlists.deleteConfirm", { name: deleting?.name ?? "" })}
              </p>
            </AlertDialog.Body>
            <AlertDialog.Footer>
              <Button variant="ghost" onPress={() => setDeleting(null)}>
                {t("common.cancel")}
              </Button>
              <Button
                variant="danger"
                isDisabled={deletePlaylist.isPending}
                onPress={() => {
                  if (!deleting) return;
                  deletePlaylist.mutate({ id: deleting.id, version: deleting.version }, { onSuccess: () => setDeleting(null) });
                }}
              >
                {t("common.delete")}
              </Button>
            </AlertDialog.Footer>
          </AlertDialog.Dialog>
        </AlertDialog.Container>
      </AlertDialog.Backdrop>
    </PageFrame>
  );
}
