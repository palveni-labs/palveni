import { Button, Dropdown, Input, Label, Spinner, toast, ToggleButton } from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowDown,
  ArrowUp,
  Check,
  ChevronDown,
  Heart,
  ListEnd,
  ListPlus,
  ListStart,
  Music2,
  Pause,
  Play,
  Shuffle,
  Volume2,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useScrollContainer } from "@/lib/scroll-container";
import { useOffsetWithin } from "@/lib/use-element-size";
import { playerStore, usePlayer } from "@/player/store";
import { useSlot } from "@/skins/scope";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { musicApi, type MusicTrack, type Order, type TrackSort } from "../api";
import { MusicArtwork } from "../artwork";
import { PlaylistPickerDialog } from "../components/playlist-picker-dialog";
import { StarRating } from "../components/star-rating";
import { useContextPoint } from "../components/context-point";
import { TrackMenu } from "../components/track-menu";
import { formatTrackTime } from "../format";
import { MusicLibraryGuard, MusicLibrarySwitcher } from "../library-context";
import { musicKeys, useRateTrack, useToggleTrackFavorite } from "../queries";
import { observeVirtualRect } from "../virtual";

export interface SongsSearchParams {
  sort?: TrackSort;
  order?: Order;
  genre?: string;
  favorite?: boolean;
  q?: string;
}

const ROW_HEIGHT = 54;
const GRID_COLS =
  "grid-cols-[2.25rem_2.25rem_minmax(0,1fr)_3.25rem_2.25rem] md:grid-cols-[2.25rem_2.25rem_minmax(0,1.5fr)_minmax(0,1fr)_3.5rem_2.25rem_2.25rem] xl:grid-cols-[2.25rem_2.25rem_minmax(0,1.6fr)_minmax(0,1fr)_minmax(0,1fr)_3.75rem_4rem_5.5rem_2.25rem_3.75rem_2.25rem]";

function VirtualSongRow({
  track,
  index,
  allTracks,
  checked,
  onToggleSelect,
}: {
  track: MusicTrack;
  index: number;
  allTracks: MusicTrack[];
  checked: boolean;
  onToggleSelect: (index: number, shiftKey: boolean) => void;
}) {
  const { t } = useTranslation();
  const player = usePlayer();
  const toggleFav = useToggleTrackFavorite();
  const rateTrack = useRateTrack();
  const { point, close, onContextMenu } = useContextPoint();

  const isCurrent = player.currentTrack?.id === track.id;
  const isPlaying = isCurrent && player.playing;

  const activate = () => {
    if (isCurrent) playerStore.togglePlay();
    else playerStore.playQueue(allTracks, index);
  };

  return (
    <div
      onContextMenu={onContextMenu}
      onDoubleClick={activate}
      className={cn(
        "group grid h-full w-full items-center gap-2 px-2 transition-colors",
        GRID_COLS,
        checked ? "bg-selected/12" : isCurrent ? "bg-selected/8" : "hover:bg-hover",
      )}
    >
      {/* Multi-select checkbox */}
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        aria-label={t("musicApp.songs.selectTrack", { title: track.title })}
        onClick={(event) => {
          event.stopPropagation();
          onToggleSelect(index, event.shiftKey);
        }}
        className="grid size-7 place-items-center rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        <span
          aria-hidden
          className={cn(
            "grid size-4 place-items-center rounded-check border transition-colors",
            checked
              ? "border-selected bg-selected text-selected-fg"
              : "border-line-control bg-surface opacity-45 group-hover:opacity-100",
          )}
        >
          {checked ? <Check className="size-3" strokeWidth={3} /> : null}
        </span>
      </button>

      {/* Play / index */}
      <button
        type="button"
        onClick={activate}
        aria-label={isPlaying ? t("musicApp.player.pause") : `${t("musicApp.actions.play")} ${track.title}`}
        className="grid size-7 place-items-center rounded-control text-xs text-fg-muted outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        {isPlaying ? (
          <>
            <Volume2 className="size-4 text-fg group-hover:hidden" aria-hidden />
            <Pause className="hidden size-3.5 fill-current text-fg group-hover:block" aria-hidden />
          </>
        ) : (
          <>
            <span data-num className={cn("tabular-nums group-hover:hidden", isCurrent && "font-strong text-fg")}>
              {index + 1}
            </span>
            <Play className="hidden ms-0.5 size-3.5 fill-current text-fg group-hover:block" aria-hidden />
          </>
        )}
      </button>

      {/* Cover + Title */}
      <div className="flex min-w-0 items-center gap-2.5">
        <MusicArtwork itemId={track.artworkItemId ?? track.id} size={96} className="size-9 shrink-0 rounded-[calc(var(--pv-radius-tile)*0.7)] ring-1 ring-line/50" />
        <div className="grid min-w-0">
          <button
            type="button"
            onClick={activate}
            className={cn("truncate text-start text-sm hover:underline", isCurrent ? "font-strong text-fg" : "font-mid text-fg")}
          >
            {track.title}
          </button>
          <span className="truncate text-xs text-fg-muted md:hidden">
            {track.artist ?? t("musicApp.artists.unknown")}
          </span>
        </div>
      </div>

      {/* Artist */}
      <div className="min-w-0 max-md:hidden">
        {track.artistId ? (
          <Link
            to="/music/artists/$libraryId/$artistId"
            params={{ libraryId: track.libraryId, artistId: track.artistId }}
            className="block truncate text-sm text-fg-muted hover:text-fg hover:underline"
          >
            {track.artist ?? t("musicApp.artists.unknown")}
          </Link>
        ) : (
          <span className="block truncate text-sm text-fg-muted">{track.artist ?? t("musicApp.artists.unknown")}</span>
        )}
      </div>

      {/* Album */}
      <div className="min-w-0 max-xl:hidden">
        {track.album && track.albumId ? (
          <Link
            to="/music/albums/$libraryId/$albumId"
            params={{ libraryId: track.libraryId, albumId: track.albumId }}
            className="block truncate text-sm text-fg-muted hover:text-fg hover:underline"
          >
            {track.album}
          </Link>
        ) : (
          <span className="block truncate text-sm text-fg-muted">{track.album ?? "—"}</span>
        )}
      </div>

      {/* Year */}
      <span data-num className="text-end text-xs text-fg-muted tabular-nums max-xl:hidden">
        {track.year ?? "—"}
      </span>

      {/* Plays */}
      <span data-num className="text-end text-xs text-fg-muted tabular-nums max-xl:hidden">
        {track.state.playCount > 0 ? track.state.playCount : "—"}
      </span>

      {/* Rating (shown when rated or on hover for a clean table) */}
      <div
        className={cn(
          "flex justify-end transition-opacity max-xl:hidden",
          track.state.stars > 0 ? "opacity-100" : "opacity-0 group-hover:opacity-100 focus-within:opacity-100",
        )}
      >
        <StarRating
          value={track.state.stars}
          size="xs"
          onChange={(stars) => rateTrack.mutate({ trackId: track.id, stars })}
        />
      </div>

      {/* Favorite */}
      <button
        type="button"
        aria-label={track.state.favorite ? t("musicApp.actions.unfavorite") : t("musicApp.actions.favorite")}
        aria-pressed={track.state.favorite}
        onClick={() => toggleFav.mutate({ trackId: track.id, favorite: !track.state.favorite })}
        className={cn(
          "grid size-8 place-items-center rounded-control text-fg-muted transition-opacity hover:bg-sunken hover:text-fg max-md:hidden",
          track.state.favorite ? "opacity-100" : "opacity-0 group-hover:opacity-100 focus-visible:opacity-100",
        )}
      >
        <Heart className={cn("size-4", track.state.favorite && "fill-failed text-failed")} aria-hidden />
      </button>

      {/* Duration */}
      <span data-num className="text-end text-xs text-fg-muted tabular-nums">
        {formatTrackTime(track.durationMs)}
      </span>

      {/* Actions */}
      <TrackMenu track={track} contextTracks={allTracks} contextPoint={point} onCloseContext={close} />
    </div>
  );
}

function SortColumnButton({
  field,
  activeSort,
  order,
  label,
  align = "start",
  className,
  onSort,
}: {
  field: TrackSort;
  activeSort: TrackSort;
  order: Order;
  label: string;
  align?: "start" | "end";
  className?: string;
  onSort: (field: TrackSort) => void;
}) {
  const active = activeSort === field;
  return (
    <button
      type="button"
      onClick={() => onSort(field)}
      className={cn(
        "inline-flex items-center gap-1 rounded-sm text-xs font-mid transition-colors hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
        align === "end" && "justify-end",
        active ? "font-strong text-fg" : "text-fg-muted",
        className,
      )}
    >
      <span className="truncate">{label}</span>
      {active ? (
        order === "asc" ? <ArrowUp className="size-3 shrink-0" aria-hidden /> : <ArrowDown className="size-3 shrink-0" aria-hidden />
      ) : null}
    </button>
  );
}

function SongsContent({ libraryId, search }: { libraryId: string; search: SongsSearchParams }) {
  const { t } = useTranslation();
  const navigate = useNavigate({ from: "/music/songs" });
  const navPhone = useSlot("navPhone");
  const scrollRef = useScrollContainer();
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollMargin = useOffsetWithin(containerRef, scrollRef);

  const [draft, setDraft] = useState<{ base: string; value: string }>(() => ({ base: search.q ?? "", value: search.q ?? "" }));
  const queryInput = draft.base === (search.q ?? "") ? draft.value : (search.q ?? "");
  const setQueryInput = (value: string) => setDraft({ base: search.q ?? "", value });
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const lastClickedRef = useRef<number | null>(null);
  const [pickerIds, setPickerIds] = useState<string[] | null>(null);

  const sort: TrackSort = search.sort ?? "title";
  const defaultDesc = sort === "added" || sort === "played" || sort === "plays" || sort === "rating";
  const order: Order = search.order ?? (defaultDesc ? "desc" : "asc");

  const updateSearch = (patch: Partial<SongsSearchParams>) => {
    void navigate({
      search: (prev) => {
        const next = { ...prev, ...patch };
        for (const [k, v] of Object.entries(next)) {
          if (v === undefined || v === "" || v === false) delete (next as Record<string, unknown>)[k];
        }
        return next;
      },
      replace: true,
    });
  };

  useEffect(() => {
    const timer = setTimeout(() => {
      const trimmed = queryInput.trim() || undefined;
      if (trimmed !== (search.q || undefined)) updateSearch({ q: trimmed });
    }, 220);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryInput]);

  const genresQuery = useQuery({
    queryKey: musicKeys.genres(libraryId),
    queryFn: ({ signal }) => musicApi.genres(libraryId, signal),
    staleTime: 60_000,
  });

  const params = {
    sort,
    order: sort === "random" ? undefined : order,
    genre: search.genre,
    favorite: search.favorite ? true : undefined,
    q: search.q,
    limit: 80,
  };

  const tracksQuery = useInfiniteQuery({
    queryKey: musicKeys.tracks(libraryId, params),
    queryFn: ({ pageParam, signal }) => musicApi.tracks(libraryId, { ...params, cursor: pageParam }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });

  const tracks = tracksQuery.data?.pages.flatMap((page) => page.tracks) ?? [];
  const total = tracksQuery.data?.pages[0]?.total;

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: tracks.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
    scrollMargin,
    observeElementRect: observeVirtualRect,
  });

  const virtualRows = virtualizer.getVirtualItems();
  const lastIndex = virtualRows.at(-1)?.index;
  useEffect(() => {
    if (lastIndex !== undefined && lastIndex >= tracks.length - 12 && tracksQuery.hasNextPage && !tracksQuery.isFetchingNextPage) {
      void tracksQuery.fetchNextPage();
    }
  }, [lastIndex, tracks.length, tracksQuery]);

  const handleSort = (field: TrackSort) => {
    if (sort === field) {
      updateSearch({ order: order === "asc" ? "desc" : "asc" });
    } else {
      const desc = field === "plays" || field === "played" || field === "rating" || field === "added" || field === "year";
      updateSearch({ sort: field, order: desc ? "desc" : "asc" });
    }
  };

  const toggleSelect = (index: number, shiftKey: boolean) => {
    const target = tracks[index];
    if (!target) return;
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (shiftKey && lastClickedRef.current !== null) {
        const start = Math.min(lastClickedRef.current, index);
        const end = Math.max(lastClickedRef.current, index);
        const shouldSelect = !prev.has(target.id);
        for (let i = start; i <= end; i++) {
          const item = tracks[i];
          if (!item) continue;
          if (shouldSelect) next.add(item.id);
          else next.delete(item.id);
        }
      } else if (next.has(target.id)) {
        next.delete(target.id);
      } else {
        next.add(target.id);
      }
      return next;
    });
    lastClickedRef.current = index;
  };

  const selectedTracks = tracks.filter((tr) => selectedIds.has(tr.id));
  const allLoadedSelected = tracks.length > 0 && selectedTracks.length === tracks.length;

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          value={queryInput}
          onChange={(event) => setQueryInput(event.target.value)}
          aria-label={t("musicApp.songs.searchPlaceholder")}
          placeholder={t("musicApp.songs.searchPlaceholder")}
          className="h-(--pv-control-h-sm) w-56 max-sm:w-full"
        />

        <Dropdown>
          <Dropdown.Trigger
            aria-label={t("musicApp.genres.title")}
            className={cn(
              "inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border px-2.5 text-sm font-mid outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus",
              search.genre ? "border-selected bg-selected/10 text-fg" : "border-line-control bg-surface text-fg-muted",
            )}
          >
            <span className="max-w-28 truncate">{search.genre ?? t("musicApp.albums.allGenres")}</span>
            <ChevronDown className="size-3.5" aria-hidden />
          </Dropdown.Trigger>
          <Dropdown.Popover placement="bottom start" className="max-h-72 min-w-44 overflow-y-auto">
            <Dropdown.Menu
              aria-label={t("musicApp.genres.title")}
              onAction={(key) => {
                const value = String(key);
                updateSearch({ genre: value === "__all__" ? undefined : value });
              }}
            >
              <Dropdown.Item id="__all__" textValue={t("musicApp.albums.allGenres")}>
                <Label className="flex-1">{t("musicApp.albums.allGenres")}</Label>
                {!search.genre ? <Check className="size-3.5" aria-hidden /> : null}
              </Dropdown.Item>
              {(genresQuery.data?.genres ?? []).map((genre) => (
                <Dropdown.Item key={genre.name} id={genre.name} textValue={genre.name}>
                  <Label className="flex-1 truncate">{genre.name}</Label>
                  <span data-num className="text-xs text-fg-muted tabular-nums">{genre.trackCount}</span>
                  {search.genre === genre.name ? <Check className="size-3.5" aria-hidden /> : null}
                </Dropdown.Item>
              ))}
            </Dropdown.Menu>
          </Dropdown.Popover>
        </Dropdown>

        <ToggleButton
          size="sm"
          isSelected={Boolean(search.favorite)}
          onChange={(on) => updateSearch({ favorite: on ? true : undefined })}
          className="rounded-control"
        >
          <Heart className={cn("size-3.5", search.favorite && "fill-failed text-failed")} aria-hidden />
          {t("musicApp.albums.favoritesOnly")}
        </ToggleButton>

        <span className="flex-1" />

        {total !== undefined ? (
          <span data-num className="text-xs text-fg-muted tabular-nums max-sm:hidden">
            {t("musicApp.songs.count", { count: total })}
          </span>
        ) : null}

        <Button
          size="sm"
          variant="primary"
          isDisabled={tracks.length === 0}
          onPress={() => playerStore.playQueue(tracks, 0, { shuffle: false })}
        >
          <Play className="fill-current" aria-hidden />
          {t("musicApp.actions.playAll")}
        </Button>
        <Button
          size="sm"
          variant="outline"
          isDisabled={tracks.length === 0}
          onPress={() => playerStore.playShuffled(tracks)}
        >
          <Shuffle aria-hidden />
          {t("musicApp.actions.shuffle")}
        </Button>

        <MusicLibrarySwitcher />
      </div>

      {tracksQuery.isPending ? (
        <LoadingRows rows={8} />
      ) : tracksQuery.isError && tracks.length === 0 ? (
        <ErrorState error={tracksQuery.error} onRetry={() => void tracksQuery.refetch()} />
      ) : tracks.length === 0 ? (
        <EmptyState icon={Music2} title={t("musicApp.songs.empty")} />
      ) : (
        <div className="overflow-hidden rounded-panel border border-line bg-surface">
          {/* Sortable column headers */}
          <div className={cn("grid items-center gap-2 border-b border-line px-2 py-2.5", GRID_COLS)}>
            <button
              type="button"
              role="checkbox"
              aria-checked={allLoadedSelected}
              aria-label={t("musicApp.songs.selectAll")}
              onClick={() => {
                if (allLoadedSelected) setSelectedIds(new Set());
                else setSelectedIds(new Set(tracks.map((tr) => tr.id)));
              }}
              className="grid size-7 place-items-center rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-focus"
            >
              <span
                aria-hidden
                className={cn(
                  "grid size-4 place-items-center rounded-check border transition-colors",
                  allLoadedSelected ? "border-selected bg-selected text-selected-fg" : "border-line-control bg-surface",
                )}
              >
                {allLoadedSelected ? <Check className="size-3" strokeWidth={3} /> : null}
              </span>
            </button>
            <span className="text-center text-xs font-mid text-fg-muted">{t("musicApp.songs.columns.index")}</span>
            <SortColumnButton field="title" activeSort={sort} order={order} label={t("musicApp.songs.columns.title")} onSort={handleSort} />
            <SortColumnButton
              field="artist"
              activeSort={sort}
              order={order}
              label={t("musicApp.songs.columns.artist")}
              onSort={handleSort}
              className="max-md:hidden"
            />
            <SortColumnButton
              field="album"
              activeSort={sort}
              order={order}
              label={t("musicApp.songs.columns.album")}
              onSort={handleSort}
              className="max-xl:hidden"
            />
            <SortColumnButton
              field="year"
              activeSort={sort}
              order={order}
              align="end"
              label={t("musicApp.songs.columns.year")}
              onSort={handleSort}
              className="max-xl:hidden"
            />
            <SortColumnButton
              field="plays"
              activeSort={sort}
              order={order}
              align="end"
              label={t("musicApp.songs.columns.plays")}
              onSort={handleSort}
              className="max-xl:hidden"
            />
            <SortColumnButton
              field="rating"
              activeSort={sort}
              order={order}
              align="end"
              label={t("musicApp.songs.columns.rating")}
              onSort={handleSort}
              className="max-xl:hidden"
            />
            <span className="max-md:hidden" aria-hidden />
            <SortColumnButton
              field="duration"
              activeSort={sort}
              order={order}
              align="end"
              label={t("musicApp.songs.columns.duration")}
              onSort={handleSort}
            />
            <span className="sr-only">{t("musicApp.actions.more")}</span>
          </div>

          {/* Virtualized rows */}
          <div ref={containerRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
            {virtualRows.map((row) => {
              const track = tracks[row.index]!;
              return (
                <div
                  key={row.key}
                  className="absolute inset-x-0 top-0 border-b border-line/70 last:border-b-0"
                  style={{ height: ROW_HEIGHT, transform: `translateY(${row.start - scrollMargin}px)` }}
                >
                  <VirtualSongRow
                    track={track}
                    index={row.index}
                    allTracks={tracks}
                    checked={selectedIds.has(track.id)}
                    onToggleSelect={toggleSelect}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      {tracksQuery.isFetchingNextPage ? (
        <div role="status" className="flex justify-center py-3">
          <Spinner size="sm" />
        </div>
      ) : null}

      {/* Floating multi-select action bar */}
      {selectedTracks.length > 0 ? (
        <div
          className={cn(
            "pointer-events-none sticky bottom-4 z-20 flex justify-center px-2",
            navPhone === "bottom-bar" && "max-md:bottom-[calc(4.75rem+env(safe-area-inset-bottom))]",
          )}
        >
          <div
            role="toolbar"
            aria-label={t("musicApp.songs.selected", { count: selectedTracks.length })}
            className="pointer-events-auto flex flex-wrap items-center gap-1.5 rounded-panel border border-line bg-raised p-1.5 ps-3.5 shadow-overlay"
          >
            <span data-num className="me-1 text-sm font-strong text-fg tabular-nums">
              {t("musicApp.songs.selected", { count: selectedTracks.length })}
            </span>
            <Button
              size="sm"
              variant="primary"
              onPress={() => {
                playerStore.playQueue(selectedTracks, 0, { shuffle: false });
                setSelectedIds(new Set());
              }}
            >
              <Play className="fill-current" aria-hidden />
              {t("musicApp.actions.play")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onPress={() => {
                playerStore.playNext(selectedTracks);
                toast.success(t("musicApp.actions.queuedNext"));
                setSelectedIds(new Set());
              }}
            >
              <ListStart aria-hidden />
              {t("musicApp.actions.playNext")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onPress={() => {
                playerStore.enqueue(selectedTracks);
                toast.success(t("musicApp.actions.queuedCount", { count: selectedTracks.length }));
                setSelectedIds(new Set());
              }}
            >
              <ListEnd aria-hidden />
              {t("musicApp.actions.addToQueue")}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onPress={() => setPickerIds(selectedTracks.map((tr) => tr.id))}
            >
              <ListPlus aria-hidden />
              {t("musicApp.actions.addToPlaylist")}
            </Button>
            <Button
              size="sm"
              variant="ghost"
              isIconOnly
              aria-label={t("musicApp.songs.clearSelection")}
              onPress={() => setSelectedIds(new Set())}
            >
              <X aria-hidden />
            </Button>
          </div>
        </div>
      ) : null}

      <PlaylistPickerDialog
        trackIds={pickerIds}
        onClose={() => {
          setPickerIds(null);
          setSelectedIds(new Set());
        }}
      />
    </div>
  );
}

export function MusicSongsPage() {
  const { t } = useTranslation();
  const search = useSearch({ from: "/_app/music/songs" });
  return (
    <PageFrame>
      <PageHeader title={t("musicApp.songs.title")} />
      <MusicLibraryGuard>{(library) => <SongsContent libraryId={library.id} search={search} />}</MusicLibraryGuard>
    </PageFrame>
  );
}
