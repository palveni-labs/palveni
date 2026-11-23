import { Button, toast } from "@heroui/react";
import { getRouteApi } from "@tanstack/react-router";
import {
  CheckSquare,
  FolderPlus,
  Grid2X2,
  Heart,
  Images,
  LayoutGrid,
  Search,
  Upload,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLibraries } from "@/api/queries/libraries";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { AddToAlbumModal } from "../components/add-to-album-modal";
import { PhotoGrid } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import { UploadPhotoModal } from "../components/upload-photo-modal";
import { groupPhotosByDay } from "../format";
import { useBatchPhotoFavorite, usePhotoMemories, usePhotoTimeline } from "../queries";
import type { PhotosTimelineSearch } from "../routes/index";

const routeApi = getRouteApi("/_app/photos/");

export function PhotosTimelinePage() {
  const { t } = useTranslation();
  const search = routeApi.useSearch();
  const navigate = routeApi.useNavigate();

  const { data: libList } = useLibraries();
  const photoLibraries = useMemo(
    () => (libList?.libraries ?? []).filter((lib) => lib.kind === "photo"),
    [libList?.libraries],
  );
  const { data: memories } = usePhotoMemories();

  const timeline = usePhotoTimeline({
    libraryId: search.libraryId,
    camera: search.camera,
    month: search.month,
    q: search.q,
  });

  const allItems = useMemo(
    () => timeline.data?.pages.flatMap((page) => page.items) ?? [],
    [timeline.data?.pages],
  );
  const months = timeline.data?.pages[0]?.months ?? [];
  const total = timeline.data?.pages[0]?.total ?? 0;
  const dayGroups = useMemo(() => groupPhotosByDay(allItems), [allItems]);

  const indexMap = useMemo(() => {
    const map = new Map<string, number>();
    allItems.forEach((item, idx) => map.set(item.id, idx));
    return map;
  }, [allItems]);

  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);
  const [uploadOpen, setUploadOpen] = useState(false);
  const [batchAlbumOpen, setBatchAlbumOpen] = useState(false);

  const batchFavorite = useBatchPhotoFavorite();
  const gridMode = search.grid ?? "square";
  const hasActiveFilter = Boolean(search.libraryId || search.camera || search.month || search.q);

  const updateSearch = (patch: Partial<PhotosTimelineSearch>) => {
    void navigate({
      search: (prev) => ({
        ...prev,
        ...patch,
      }),
    });
  };

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleDayGroup = (dayItems: readonly { id: string }[]) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      const allSelected = dayItems.every((item) => next.has(item.id));
      for (const item of dayItems) {
        if (allSelected) next.delete(item.id);
        else next.add(item.id);
      }
      return next;
    });
  };

  const selectedArray = useMemo(() => [...selectedIds], [selectedIds]);

  return (
    <div className="grid gap-5">
      <PageHeader
        title={t("photosApp.timeline.title")}
        summary={t("photosApp.timeline.totalCount", { count: total })}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <div className="inline-flex rounded-control border border-line bg-sunken p-0.5">
              <button
                type="button"
                aria-label={t("photosApp.timeline.gridSquare")}
                aria-pressed={gridMode === "square"}
                onClick={() => updateSearch({ grid: "square" })}
                className={cn(
                  "grid size-7 place-items-center rounded-control transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  gridMode === "square" ? "bg-surface text-fg shadow-xs" : "text-fg-muted hover:text-fg",
                )}
              >
                <Grid2X2 className="size-4" aria-hidden />
              </button>
              <button
                type="button"
                aria-label={t("photosApp.timeline.gridAspect")}
                aria-pressed={gridMode === "aspect"}
                onClick={() => updateSearch({ grid: "aspect" })}
                className={cn(
                  "grid size-7 place-items-center rounded-control transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  gridMode === "aspect" ? "bg-surface text-fg shadow-xs" : "text-fg-muted hover:text-fg",
                )}
              >
                <LayoutGrid className="size-4" aria-hidden />
              </button>
            </div>

            <Button
              size="sm"
              variant={selectMode ? "primary" : "outline"}
              onPress={() => {
                setSelectMode((prev) => {
                  if (prev) setSelectedIds(new Set());
                  return !prev;
                });
              }}
            >
              <CheckSquare className="size-4" aria-hidden />
              {selectMode ? t("photosApp.timeline.exitSelect") : t("photosApp.timeline.selectMode")}
            </Button>

            <Button size="sm" variant="primary" onPress={() => setUploadOpen(true)}>
              <Upload className="size-4" aria-hidden />
              {t("photosApp.timeline.upload")}
            </Button>
          </div>
        }
      />

      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-48 flex-1 sm:max-w-xs">
          <Search className="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-fg-subtle" aria-hidden />
          <input
            type="search"
            aria-label={t("photosApp.search.placeholder")}
            value={search.q ?? ""}
            onChange={(e) => updateSearch({ q: e.target.value || undefined })}
            placeholder={t("photosApp.search.placeholder")}
            className="h-9 w-full rounded-control border border-line-strong bg-surface ps-8 pe-3 text-sm text-fg placeholder:text-fg-subtle focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          />
        </div>

        <select
          aria-label={t("photosApp.timeline.allLibraries")}
          value={search.libraryId ?? ""}
          onChange={(e) => updateSearch({ libraryId: e.target.value || undefined })}
          className="h-9 rounded-control border border-line-strong bg-surface px-2.5 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        >
          <option value="">{t("photosApp.timeline.allLibraries")}</option>
          {photoLibraries.map((lib) => (
            <option key={lib.id} value={lib.id}>
              {lib.name}
            </option>
          ))}
        </select>

        <select
          aria-label={t("photosApp.timeline.allCameras")}
          value={search.camera ?? ""}
          onChange={(e) => updateSearch({ camera: e.target.value || undefined })}
          className="h-9 max-w-56 rounded-control border border-line-strong bg-surface px-2.5 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        >
          <option value="">{t("photosApp.timeline.allCameras")}</option>
          {(memories?.cameras ?? []).map((cam) => (
            <option key={cam.name} value={cam.name}>
              {cam.name} ({cam.count})
            </option>
          ))}
        </select>

        {hasActiveFilter ? (
          <Button
            size="sm"
            variant="ghost"
            onPress={() =>
              updateSearch({
                libraryId: undefined,
                camera: undefined,
                month: undefined,
                q: undefined,
              })
            }
          >
            <X className="size-3.5" aria-hidden />
            {t("photosApp.search.clear")}
          </Button>
        ) : null}
      </div>

      {selectMode && selectedArray.length > 0 ? (
        <div
          role="region"
          aria-label={t("photosApp.timeline.selectedCount", { count: selectedArray.length })}
          className="flex flex-wrap items-center justify-between gap-3 rounded-panel border border-line-strong bg-surface px-4 py-2.5 shadow-panel"
        >
          <span data-num className="text-sm font-strong text-fg tabular-nums">
            {t("photosApp.timeline.selectedCount", { count: selectedArray.length })}
          </span>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="sm"
              variant="outline"
              isDisabled={batchFavorite.isPending}
              onPress={() =>
                batchFavorite.mutate(
                  { itemIds: selectedArray, favorite: true },
                  {
                    onSuccess: () => {
                      setSelectedIds(new Set());
                      setSelectMode(false);
                    },
                    onError: (err) => toast.danger(errorText(t, err)),
                  },
                )
              }
            >
              <Heart className="size-4" aria-hidden />
              {t("photosApp.timeline.batchFavorite")}
            </Button>
            <Button size="sm" variant="primary" onPress={() => setBatchAlbumOpen(true)}>
              <FolderPlus className="size-4" aria-hidden />
              {t("photosApp.timeline.batchAddToAlbum")}
            </Button>
          </div>
        </div>
      ) : null}

      {timeline.isLoading ? (
        <LoadingRows rows={5} />
      ) : timeline.isError ? (
        <ErrorState error={timeline.error} onRetry={() => void timeline.refetch()} />
      ) : allItems.length === 0 ? (
        <EmptyState
          icon={Images}
          title={
            hasActiveFilter
              ? t("photosApp.timeline.filteredEmptyTitle")
              : t("photosApp.timeline.emptyTitle")
          }
          detail={
            hasActiveFilter
              ? t("photosApp.timeline.filteredEmptyDetail")
              : t("photosApp.timeline.emptyDetail")
          }
          action={
            hasActiveFilter ? (
              <Button
                size="sm"
                variant="outline"
                onPress={() =>
                  updateSearch({
                    libraryId: undefined,
                    camera: undefined,
                    month: undefined,
                    q: undefined,
                  })
                }
              >
                {t("photosApp.search.clear")}
              </Button>
            ) : (
              <Button size="sm" variant="primary" onPress={() => setUploadOpen(true)}>
                <Upload className="size-4" aria-hidden />
                {t("photosApp.timeline.upload")}
              </Button>
            )
          }
        />
      ) : (
        <div className="flex items-start gap-4">
          <div className="grid min-w-0 flex-1 gap-6">
            {dayGroups.map((group) => (
              <section key={group.day} aria-label={group.day} className="grid gap-2.5">
                <div className="flex items-center justify-between gap-2">
                  <h2 className="font-heading text-base font-strong text-heading tabular-nums">
                    {group.day}
                    <span className="ms-2 text-xs font-normal text-fg-muted">
                      ({group.items.length})
                    </span>
                  </h2>
                  {selectMode ? (
                    <button
                      type="button"
                      onClick={() => toggleDayGroup(group.items)}
                      className="rounded-control px-2 py-0.5 text-xs font-mid text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                    >
                      {t("photosApp.timeline.selectAllDay")}
                    </button>
                  ) : null}
                </div>
                <PhotoGrid
                  items={group.items}
                  mode={gridMode}
                  selectMode={selectMode}
                  selectedIds={selectedIds}
                  onToggleSelect={toggleSelect}
                  onOpenPhoto={(localIdx) => {
                    const target = group.items[localIdx];
                    if (target) {
                      setLightboxIndex(indexMap.get(target.id) ?? 0);
                    }
                  }}
                />
              </section>
            ))}

            {timeline.hasNextPage ? (
              <div className="flex justify-center pt-2">
                <Button
                  size="sm"
                  variant="outline"
                  isDisabled={timeline.isFetchingNextPage}
                  onPress={() => void timeline.fetchNextPage()}
                >
                  {t("photosApp.timeline.loadMore")}
                </Button>
              </div>
            ) : null}
          </div>

          {months.length > 0 ? (
            <nav
              aria-label={t("photosApp.timeline.monthScrubber")}
              className="sticky top-4 hidden max-h-[75vh] w-28 shrink-0 flex-col gap-1 overflow-y-auto rounded-panel border border-line bg-surface p-2 sm:flex"
            >
              <button
                type="button"
                aria-pressed={!search.month}
                onClick={() => updateSearch({ month: undefined })}
                className={cn(
                  "flex items-center justify-between rounded-control px-2 py-1 text-xs transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  !search.month ? "bg-nav-active font-strong text-nav-active-fg" : "text-fg-muted hover:bg-hover hover:text-fg",
                )}
              >
                <span className="truncate">{t("photosApp.timeline.allMonths")}</span>
              </button>
              {months.map((bucket) => {
                const active = search.month === bucket.month;
                return (
                  <button
                    key={bucket.month}
                    type="button"
                    aria-pressed={active}
                    onClick={() => updateSearch({ month: active ? undefined : bucket.month })}
                    className={cn(
                      "flex items-center justify-between gap-1 rounded-control px-2 py-1 text-xs tabular-nums transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                      active
                        ? "bg-nav-active font-strong text-nav-active-fg"
                        : "text-fg-muted hover:bg-hover hover:text-fg",
                    )}
                  >
                    <span>{bucket.month}</span>
                    <span className="opacity-70">{bucket.count}</span>
                  </button>
                );
              })}
            </nav>
          ) : null}
        </div>
      )}

      <PhotoLightbox
        items={allItems}
        index={lightboxIndex}
        onIndexChange={setLightboxIndex}
        onClose={() => setLightboxIndex(null)}
      />

      <UploadPhotoModal
        open={uploadOpen}
        onClose={() => setUploadOpen(false)}
        photoLibraries={photoLibraries}
        defaultLibraryId={search.libraryId}
      />

      <AddToAlbumModal
        itemIds={selectedArray}
        open={batchAlbumOpen}
        onClose={() => setBatchAlbumOpen(false)}
        onCompleted={() => {
          setSelectedIds(new Set());
          setSelectMode(false);
        }}
      />
    </div>
  );
}
