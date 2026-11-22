import { Button } from "@heroui/react";
import { ArrowDown, ArrowUp, GripVertical, ListPlus, Play, Trash2, Volume2, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { MusicArtwork } from "@/modules/music/artwork";
import { PlaylistNameDialog } from "@/modules/music/components/playlist-picker-dialog";
import { formatCollectionDuration, formatTrackTime } from "@/modules/music/format";
import { useCreatePlaylist } from "@/modules/music/queries";
import { playerStore, usePlayer } from "./store";

/**
 * Reusable interactive queue list supporting click-to-jump, drag-and-drop reorder,
 * keyboard up/down reorder, and item removal. Works in both the light/dark drawer and the
 * dark full-screen Now Playing view.
 */
export function QueueList({ dark = false }: { dark?: boolean }) {
  const { t } = useTranslation();
  const player = usePlayer();
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);

  if (player.queue.length === 0) {
    return (
      <p className={cn("py-12 text-center text-sm", dark ? "text-white/70" : "text-fg-muted")}>
        {t("musicApp.player.emptyQueue")}
      </p>
    );
  }

  return (
    <ol className="grid gap-0.5">
      {player.queue.map((track, index) => {
        const active = index === player.currentIndex;
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
              event.dataTransfer.dropEffect = "move";
              if (overIndex !== index) setOverIndex(index);
            }}
            onDrop={(event) => {
              event.preventDefault();
              if (dragIndex !== null && dragIndex !== index) {
                playerStore.moveInQueue(dragIndex, index);
              }
              setDragIndex(null);
              setOverIndex(null);
            }}
            onDragEnd={() => {
              setDragIndex(null);
              setOverIndex(null);
            }}
            className={cn(
              "group flex items-center gap-2 rounded-tile px-2 py-1.5 transition-colors",
              isDragging && "opacity-45",
              isOver && (dark ? "ring-2 ring-white/60" : "ring-2 ring-action"),
              active
                ? dark
                  ? "bg-white/14 text-white"
                  : "bg-selected/12 text-fg"
                : dark
                  ? "text-white/90 hover:bg-white/8"
                  : "text-fg hover:bg-hover",
            )}
          >
            <span
              aria-hidden
              title={t("musicApp.playlists.dragHandle")}
              className={cn("cursor-grab px-0.5 active:cursor-grabbing", dark ? "text-white/45" : "text-fg-subtle")}
            >
              <GripVertical className="size-3.5" />
            </span>
            <button
              type="button"
              onClick={() => playerStore.jumpTo(index)}
              className="flex min-w-0 flex-1 items-center gap-2.5 text-start outline-none focus-visible:ring-2 focus-visible:ring-focus"
            >
              <span className="relative shrink-0">
                <MusicArtwork itemId={track.artworkItemId ?? track.id} size={96} className="size-10 rounded-[calc(var(--pv-radius-tile)*0.7)]" />
                {active ? (
                  <span className="absolute inset-0 grid place-items-center rounded-[calc(var(--pv-radius-tile)*0.7)] bg-black/55 text-white">
                    {player.playing ? <Volume2 className="size-4" aria-hidden /> : <Play className="size-4 fill-current" aria-hidden />}
                  </span>
                ) : null}
              </span>
              <span className="grid min-w-0 flex-1 gap-0.5">
                <span className={cn("truncate text-sm", active ? "font-strong" : "font-mid")}>{track.title}</span>
                <span className={cn("truncate text-xs", dark ? "text-white/65" : "text-fg-muted")}>
                  {track.artist ?? t("musicApp.artists.unknown")}
                </span>
              </span>
            </button>
            <span data-num className={cn("text-xs tabular-nums group-hover:hidden", dark ? "text-white/65" : "text-fg-muted")}>
              {formatTrackTime(track.durationMs)}
            </span>
            <div className="hidden items-center gap-0.5 group-hover:flex group-focus-within:flex">
              <button
                type="button"
                disabled={index === 0}
                aria-label={t("musicApp.playlists.moveUp")}
                onClick={() => playerStore.moveInQueue(index, index - 1)}
                className={cn(
                  "grid size-6 place-items-center rounded-sm disabled:opacity-30",
                  dark ? "text-white/80 hover:bg-white/15" : "text-fg-muted hover:bg-sunken hover:text-fg",
                )}
              >
                <ArrowUp className="size-3.5" aria-hidden />
              </button>
              <button
                type="button"
                disabled={index === player.queue.length - 1}
                aria-label={t("musicApp.playlists.moveDown")}
                onClick={() => playerStore.moveInQueue(index, index + 1)}
                className={cn(
                  "grid size-6 place-items-center rounded-sm disabled:opacity-30",
                  dark ? "text-white/80 hover:bg-white/15" : "text-fg-muted hover:bg-sunken hover:text-fg",
                )}
              >
                <ArrowDown className="size-3.5" aria-hidden />
              </button>
              <button
                type="button"
                aria-label={t("musicApp.player.removeFromQueue")}
                onClick={() => playerStore.removeAt(index)}
                className={cn(
                  "grid size-6 place-items-center rounded-sm",
                  dark ? "text-white/80 hover:bg-white/15" : "text-fg-muted hover:bg-sunken hover:text-fg",
                )}
              >
                <X className="size-3.5" aria-hidden />
              </button>
            </div>
          </li>
        );
      })}
    </ol>
  );
}

/** Slide-over queue drawer anchored on the right of the viewport above the player bar. */
export function QueueDrawer() {
  const { t, i18n } = useTranslation();
  const player = usePlayer();
  const createPlaylist = useCreatePlaylist();
  const [saveOpen, setSaveOpen] = useState(false);

  if (!player.queueOpen) return null;

  const totalMs = player.queue.reduce((sum, track) => sum + (track.durationMs ?? 0), 0);
  const durationLabel = formatCollectionDuration(totalMs, i18n.resolvedLanguage ?? "zh-CN");

  return (
    <>
      <div
        aria-hidden
        onClick={() => playerStore.setQueueOpen(false)}
        className="fixed inset-0 z-40 bg-black/30 backdrop-blur-[2px]"
      />
      <aside
        role="dialog"
        aria-label={t("musicApp.player.queue")}
        className="fixed end-0 top-0 bottom-0 z-50 flex w-96 max-w-full flex-col border-s border-line bg-raised text-fg shadow-overlay"
      >
        <header className="flex items-center gap-2 border-b border-line px-4 py-3.5">
          <div className="min-w-0 flex-1">
            <h2 className="font-heading text-md font-title text-heading">{t("musicApp.player.queue")}</h2>
            <p className="text-xs text-fg-muted tabular-nums">
              {t("musicApp.playlists.tracksCount", { count: player.queue.length })}
              {durationLabel ? ` · ${durationLabel}` : ""}
            </p>
          </div>
          <Button
            size="sm"
            variant="ghost"
            isDisabled={player.queue.length === 0}
            onPress={() => setSaveOpen(true)}
            aria-label={t("musicApp.player.saveAsPlaylist")}
          >
            <ListPlus className="size-4" aria-hidden />
            <span className="max-sm:sr-only">{t("musicApp.player.saveAsPlaylist")}</span>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            isIconOnly
            isDisabled={player.queue.length === 0}
            onPress={() => playerStore.clearQueue()}
            aria-label={t("musicApp.player.clearQueue")}
          >
            <Trash2 className="size-4" aria-hidden />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            isIconOnly
            onPress={() => playerStore.setQueueOpen(false)}
            aria-label={t("a11y.close")}
          >
            <X className="size-4" aria-hidden />
          </Button>
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto px-2 py-2">
          <QueueList />
        </div>
      </aside>

      <PlaylistNameDialog
        key={saveOpen ? "open" : "closed"}
        isOpen={saveOpen}
        onOpenChange={setSaveOpen}
        title={t("musicApp.playlists.saveQueueTitle")}
        submitLabel={t("common.create")}
        isPending={createPlaylist.isPending}
        onSubmit={async (name) => {
          await createPlaylist.mutateAsync({ name, trackIds: player.queue.map((track) => track.id) });
        }}
      />
    </>
  );
}
