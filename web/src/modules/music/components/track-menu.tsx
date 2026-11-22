import { Dropdown, Label, Separator, toast } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import {
  Disc3,
  Download,
  Heart,
  ListEnd,
  ListPlus,
  ListStart,
  MoreHorizontal,
  Play,
  Shuffle,
  Star,
  Trash2,
  UserRound,
} from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { playerStore } from "@/player/store";
import { downloadOriginal, musicApi, type MusicAlbum, type MusicTrack } from "../api";
import { useRateTrack, useToggleAlbumFavorite, useToggleTrackFavorite } from "../queries";
import type { ContextPoint } from "./context-point";
import { PlaylistPickerDialog } from "./playlist-picker-dialog";

/* ---------------------------------------------------------------- shared floating context menu */

function FloatingMenu({
  point,
  onClose,
  label,
  children,
}: {
  point: ContextPoint | null;
  onClose: () => void;
  label: string;
  children: ReactNode;
}) {
  useEffect(() => {
    if (!point) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    const onScroll = () => onClose();
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", onScroll, true);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [point, onClose]);

  if (!point || typeof document === "undefined") return null;

  const left = Math.max(8, Math.min(point.x, window.innerWidth - 228));
  const top = Math.max(8, Math.min(point.y, window.innerHeight - 340));

  return createPortal(
    <>
      <div
        aria-hidden
        className="fixed inset-0 z-50"
        onClick={onClose}
        onContextMenu={(event) => {
          event.preventDefault();
          onClose();
        }}
      />
      <div
        role="menu"
        aria-label={label}
        style={{ left, top }}
        className="fixed z-50 w-52 overflow-hidden rounded-panel border border-line bg-raised p-1 text-fg shadow-overlay"
      >
        {children}
      </div>
    </>,
    document.body,
  );
}

function ContextItem({
  icon,
  label,
  danger = false,
  onSelect,
}: {
  icon: ReactNode;
  label: string;
  danger?: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onSelect}
      className={cn(
        "flex w-full items-center gap-2.5 rounded-control px-2.5 py-1.5 text-start text-sm transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
        danger ? "text-failed" : "text-fg",
      )}
    >
      <span className={cn("shrink-0", danger ? "text-failed" : "text-fg-muted")} aria-hidden>
        {icon}
      </span>
      <span className="min-w-0 flex-1 truncate">{label}</span>
    </button>
  );
}

/* ---------------------------------------------------------------- track menu */

export interface TrackMenuProps {
  track: MusicTrack;
  contextTracks?: MusicTrack[];
  onRemove?: () => void;
  removeLabel?: string;
  triggerClassName?: string;
  /** Optional externally opened right-click point. */
  contextPoint?: ContextPoint | null;
  onCloseContext?: () => void;
}

export function TrackMenu({
  track,
  contextTracks,
  onRemove,
  removeLabel,
  triggerClassName,
  contextPoint = null,
  onCloseContext,
}: TrackMenuProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const toggleFav = useToggleTrackFavorite();
  const rateTrack = useRateTrack();
  const [pickerIds, setPickerIds] = useState<string[] | null>(null);

  const runAction = (action: string) => {
    onCloseContext?.();
    if (action === "play") {
      playerStore.playTrack(track, contextTracks);
    } else if (action === "playNext") {
      playerStore.playNext(track);
      toast.success(t("musicApp.actions.queuedNext"));
    } else if (action === "addQueue") {
      playerStore.enqueue(track);
      toast.success(t("musicApp.actions.queuedEnd"));
    } else if (action === "playlist") {
      setPickerIds([track.id]);
    } else if (action === "favorite") {
      toggleFav.mutate({ trackId: track.id, favorite: !track.state.favorite });
    } else if (action.startsWith("rate:")) {
      const stars = Number(action.slice(5));
      rateTrack.mutate({ trackId: track.id, stars });
    } else if (action === "album" && track.albumId) {
      void navigate({ to: "/music/albums/$libraryId/$albumId", params: { libraryId: track.libraryId, albumId: track.albumId } });
    } else if (action === "artist" && track.artistId) {
      void navigate({ to: "/music/artists/$libraryId/$artistId", params: { libraryId: track.libraryId, artistId: track.artistId } });
    } else if (action === "download") {
      void downloadOriginal(track.id).catch(() => toast.danger(t("musicApp.actions.downloadFailed")));
    } else if (action === "remove") {
      onRemove?.();
    }
  };

  return (
    <>
      <Dropdown>
        <Dropdown.Trigger
          aria-label={t("musicApp.actions.more")}
          onClick={(event) => event.stopPropagation()}
          className={cn(
            "grid size-8 shrink-0 place-items-center rounded-control text-fg-muted outline-none transition-colors hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus",
            triggerClassName,
          )}
        >
          <MoreHorizontal className="size-4" aria-hidden />
        </Dropdown.Trigger>
        <Dropdown.Popover placement="bottom end" className="min-w-48">
          <Dropdown.Menu aria-label={track.title} onAction={(key) => runAction(String(key))}>
            <Dropdown.Item id="play" textValue={t("musicApp.actions.play")}>
              <Play className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.play")}</Label>
            </Dropdown.Item>
            <Dropdown.Item id="playNext" textValue={t("musicApp.actions.playNext")}>
              <ListStart className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.playNext")}</Label>
            </Dropdown.Item>
            <Dropdown.Item id="addQueue" textValue={t("musicApp.actions.addToQueue")}>
              <ListEnd className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.addToQueue")}</Label>
            </Dropdown.Item>
            <Dropdown.Item id="playlist" textValue={t("musicApp.actions.addToPlaylist")}>
              <ListPlus className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.addToPlaylist")}</Label>
            </Dropdown.Item>
            <Separator />
            <Dropdown.Item id="favorite" textValue={track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}>
              <Heart className={cn("size-4", track.state.favorite ? "fill-failed text-failed" : "text-fg-muted")} aria-hidden />
              <Label>{track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}</Label>
            </Dropdown.Item>
            <Dropdown.Item
              id={`rate:${track.state.stars === 5 ? 0 : 5}`}
              textValue={track.state.stars === 5 ? t("musicApp.actions.clearRating") : t("musicApp.actions.rateStars", { stars: 5 })}
            >
              <Star className={cn("size-4", track.state.stars > 0 ? "fill-degraded text-degraded" : "text-fg-muted")} aria-hidden />
              <Label>{track.state.stars === 5 ? t("musicApp.actions.clearRating") : t("musicApp.actions.rateStars", { stars: 5 })}</Label>
            </Dropdown.Item>
            {track.albumId || track.artistId ? <Separator /> : null}
            {track.albumId ? (
              <Dropdown.Item id="album" textValue={t("musicApp.actions.goToAlbum")}>
                <Disc3 className="size-4 text-fg-muted" aria-hidden />
                <Label>{t("musicApp.actions.goToAlbum")}</Label>
              </Dropdown.Item>
            ) : null}
            {track.artistId ? (
              <Dropdown.Item id="artist" textValue={t("musicApp.actions.goToArtist")}>
                <UserRound className="size-4 text-fg-muted" aria-hidden />
                <Label>{t("musicApp.actions.goToArtist")}</Label>
              </Dropdown.Item>
            ) : null}
            <Separator />
            <Dropdown.Item id="download" textValue={t("musicApp.actions.download")}>
              <Download className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.download")}</Label>
            </Dropdown.Item>
            {onRemove ? (
              <Dropdown.Item id="remove" variant="danger" textValue={removeLabel ?? t("musicApp.playlists.removeTrack")}>
                <Trash2 className="size-4 text-failed" aria-hidden />
                <Label>{removeLabel ?? t("musicApp.playlists.removeTrack")}</Label>
              </Dropdown.Item>
            ) : null}
          </Dropdown.Menu>
        </Dropdown.Popover>
      </Dropdown>

      <FloatingMenu point={contextPoint} onClose={() => onCloseContext?.()} label={track.title}>
        <ContextItem icon={<Play className="size-4" />} label={t("musicApp.actions.play")} onSelect={() => runAction("play")} />
        <ContextItem icon={<ListStart className="size-4" />} label={t("musicApp.actions.playNext")} onSelect={() => runAction("playNext")} />
        <ContextItem icon={<ListEnd className="size-4" />} label={t("musicApp.actions.addToQueue")} onSelect={() => runAction("addQueue")} />
        <ContextItem icon={<ListPlus className="size-4" />} label={t("musicApp.actions.addToPlaylist")} onSelect={() => runAction("playlist")} />
        <div className="my-1 border-t border-line" />
        <ContextItem
          icon={<Heart className={cn("size-4", track.state.favorite && "fill-failed text-failed")} />}
          label={track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
          onSelect={() => runAction("favorite")}
        />
        <div className="flex items-center justify-between px-2.5 py-1.5">
          <span className="text-xs text-fg-muted">{t("musicApp.actions.rating")}</span>
          <span className="inline-flex items-center gap-0.5">
            {[1, 2, 3, 4, 5].map((star) => (
              <button
                key={star}
                type="button"
                aria-label={track.state.stars === star ? t("musicApp.actions.clearRating") : t("musicApp.actions.rateStars", { stars: star })}
                onClick={() => runAction(`rate:${track.state.stars === star ? 0 : star}`)}
                className="p-0.5"
              >
                <Star className={cn("size-3.5", star <= track.state.stars ? "fill-degraded text-degraded" : "text-fg-subtle")} />
              </button>
            ))}
          </span>
        </div>
        {track.albumId || track.artistId ? <div className="my-1 border-t border-line" /> : null}
        {track.albumId ? <ContextItem icon={<Disc3 className="size-4" />} label={t("musicApp.actions.goToAlbum")} onSelect={() => runAction("album")} /> : null}
        {track.artistId ? <ContextItem icon={<UserRound className="size-4" />} label={t("musicApp.actions.goToArtist")} onSelect={() => runAction("artist")} /> : null}
        <div className="my-1 border-t border-line" />
        <ContextItem icon={<Download className="size-4" />} label={t("musicApp.actions.download")} onSelect={() => runAction("download")} />
        {onRemove ? (
          <ContextItem
            danger
            icon={<Trash2 className="size-4" />}
            label={removeLabel ?? t("musicApp.playlists.removeTrack")}
            onSelect={() => runAction("remove")}
          />
        ) : null}
      </FloatingMenu>

      <PlaylistPickerDialog trackIds={pickerIds} onClose={() => setPickerIds(null)} />
    </>
  );
}

/* ---------------------------------------------------------------- album menu */

export function AlbumMenu({
  album,
  triggerClassName,
  contextPoint = null,
  onCloseContext,
}: {
  album: MusicAlbum;
  triggerClassName?: string;
  contextPoint?: ContextPoint | null;
  onCloseContext?: () => void;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const toggleFav = useToggleAlbumFavorite();
  const [pickerIds, setPickerIds] = useState<string[] | null>(null);

  const loadTracks = async () => {
    const detail = await musicApi.album(album.libraryId, album.id);
    return detail.tracks;
  };

  const runAction = async (action: string) => {
    onCloseContext?.();
    if (action === "play") {
      const tracks = await loadTracks();
      if (tracks.length) playerStore.playQueue(tracks, 0, { shuffle: false });
    } else if (action === "shuffle") {
      const tracks = await loadTracks();
      if (tracks.length) playerStore.playShuffled(tracks);
    } else if (action === "playNext") {
      const tracks = await loadTracks();
      if (tracks.length) {
        playerStore.playNext(tracks);
        toast.success(t("musicApp.actions.queuedNext"));
      }
    } else if (action === "addQueue") {
      const tracks = await loadTracks();
      if (tracks.length) {
        playerStore.enqueue(tracks);
        toast.success(t("musicApp.actions.queuedCount", { count: tracks.length }));
      }
    } else if (action === "playlist") {
      const tracks = await loadTracks();
      if (tracks.length) setPickerIds(tracks.map((tr) => tr.id));
    } else if (action === "favorite") {
      toggleFav.mutate({ libraryId: album.libraryId, albumId: album.id, favorite: !album.favorite });
    } else if (action === "artist" && album.artistId) {
      void navigate({ to: "/music/artists/$libraryId/$artistId", params: { libraryId: album.libraryId, artistId: album.artistId } });
    }
  };

  return (
    <>
      <Dropdown>
        <Dropdown.Trigger
          aria-label={t("musicApp.actions.more")}
          onClick={(event) => event.stopPropagation()}
          className={cn(
            "grid size-8 shrink-0 place-items-center rounded-control text-fg-muted outline-none transition-colors hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus",
            triggerClassName,
          )}
        >
          <MoreHorizontal className="size-4" aria-hidden />
        </Dropdown.Trigger>
        <Dropdown.Popover placement="bottom end" className="min-w-48">
          <Dropdown.Menu aria-label={album.name ?? t("musicApp.albums.unknown")} onAction={(key) => void runAction(String(key))}>
            <Dropdown.Item id="play" textValue={t("musicApp.actions.play")}>
              <Play className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.play")}</Label>
            </Dropdown.Item>
            <Dropdown.Item id="shuffle" textValue={t("musicApp.actions.shuffle")}>
              <Shuffle className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.shuffle")}</Label>
            </Dropdown.Item>
            <Dropdown.Item id="playNext" textValue={t("musicApp.actions.playNext")}>
              <ListStart className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.playNext")}</Label>
            </Dropdown.Item>
            <Dropdown.Item id="addQueue" textValue={t("musicApp.actions.addToQueue")}>
              <ListEnd className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.addToQueue")}</Label>
            </Dropdown.Item>
            <Dropdown.Item id="playlist" textValue={t("musicApp.actions.addToPlaylist")}>
              <ListPlus className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("musicApp.actions.addToPlaylist")}</Label>
            </Dropdown.Item>
            <Separator />
            <Dropdown.Item id="favorite" textValue={album.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}>
              <Heart className={cn("size-4", album.favorite ? "fill-failed text-failed" : "text-fg-muted")} aria-hidden />
              <Label>{album.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}</Label>
            </Dropdown.Item>
            {album.artistId ? (
              <Dropdown.Item id="artist" textValue={t("musicApp.actions.goToArtist")}>
                <UserRound className="size-4 text-fg-muted" aria-hidden />
                <Label>{t("musicApp.actions.goToArtist")}</Label>
              </Dropdown.Item>
            ) : null}
          </Dropdown.Menu>
        </Dropdown.Popover>
      </Dropdown>

      <FloatingMenu point={contextPoint} onClose={() => onCloseContext?.()} label={album.name ?? t("musicApp.albums.unknown")}>
        <ContextItem icon={<Play className="size-4" />} label={t("musicApp.actions.play")} onSelect={() => void runAction("play")} />
        <ContextItem icon={<Shuffle className="size-4" />} label={t("musicApp.actions.shuffle")} onSelect={() => void runAction("shuffle")} />
        <ContextItem icon={<ListStart className="size-4" />} label={t("musicApp.actions.playNext")} onSelect={() => void runAction("playNext")} />
        <ContextItem icon={<ListEnd className="size-4" />} label={t("musicApp.actions.addToQueue")} onSelect={() => void runAction("addQueue")} />
        <ContextItem icon={<ListPlus className="size-4" />} label={t("musicApp.actions.addToPlaylist")} onSelect={() => void runAction("playlist")} />
        <div className="my-1 border-t border-line" />
        <ContextItem
          icon={<Heart className={cn("size-4", album.favorite && "fill-failed text-failed")} />}
          label={album.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
          onSelect={() => void runAction("favorite")}
        />
        {album.artistId ? <ContextItem icon={<UserRound className="size-4" />} label={t("musicApp.actions.goToArtist")} onSelect={() => void runAction("artist")} /> : null}
      </FloatingMenu>

      <PlaylistPickerDialog trackIds={pickerIds} onClose={() => setPickerIds(null)} />
    </>
  );
}
