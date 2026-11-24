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
  Sparkles,
  Upload,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLibraries } from "@/api/queries/libraries";
import { currentLanguage } from "@/i18n";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import type { PhotoQueryFilter } from "../api";
import { AddToAlbumModal } from "../components/add-to-album-modal";
import { PhotoGrid } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import { UploadPhotoModal } from "../components/upload-photo-modal";
import { groupPhotosByDay } from "../format";
import {
  useAnalyzePhotos,
  useBatchPhotoFavorite,
  usePhotoMemories,
  usePhotoSearch,
  usePhotoTimeline,
} from "../queries";
import type { PhotosTimelineSearch } from "../routes/index";

const routeApi = getRouteApi("/_app/photos/");

const SMART_SUGGESTIONS_ZH = ["去年秋天的夜景", "蓝色天空", "索尼竖拍"] as const;
const SMART_SUGGESTIONS_EN = ["last year autumn night", "blue sky", "sony portrait"] as const;

const COLOR_DOT_STYLE: Record<string, string> = {
  red: "#c83b3b",
  orange: "#d9752b",
  yellow: "#d6a428",
  green: "#2f8f5b",
  blue: "#2b6cb0",
  warm: "#b87a3d",
  dark: "#333942",
  bright: "#d6dbe0",
};

function ParsedFilterChips({ filter }: { filter: PhotoQueryFilter }) {
  const { t } = useTranslation();
  const chips: Array<{ key: string; label: string; dot?: string }> = [];

  if (filter.year !== null) {
    chips.push({ key: "year", label: t("photosApp.ai.filter.year", { value: filter.year }) });
  }
  if (filter.month !== null) {
    chips.push({ key: "month", label: t("photosApp.ai.filter.month", { value: filter.month }) });
  }
  if (filter.season) {
    chips.push({
      key: "season",
      label: t(`photosApp.ai.filter.season.${filter.season}` as "photosApp.ai.filter.season.autumn"),
    });
  }
  if (filter.timeOfDay) {
    chips.push({
      key: "timeOfDay",
      label: t(
        `photosApp.ai.filter.timeOfDay.${filter.timeOfDay}` as "photosApp.ai.filter.timeOfDay.night",
      ),
    });
  }
  if (filter.color) {
    chips.push({
      key: "color",
      label: t(`photosApp.ai.filter.color.${filter.color}` as "photosApp.ai.filter.color.blue"),
      dot: COLOR_DOT_STYLE[filter.color],
    });
  }
  if (filter.orientation) {
    chips.push({
      key: "orientation",
      label: t(
        `photosApp.ai.filter.orientation.${filter.orientation}` as "photosApp.ai.filter.orientation.portrait",
      ),
    });
  }
  if (filter.cameraBrand) {
    chips.push({ key: "camera", label: filter.cameraBrand.toUpperCase() });
  }
  if (filter.geotagged) {
    chips.push({ key: "geo", label: t("photosApp.ai.filter.geotagged") });
  }
  for (const kw of filter.keywords) {
    chips.push({ key: `kw:${kw}`, label: kw });
  }

  if (chips.length === 0) return null;

  return (
    <div
      role="region"
      aria-label={t("photosApp.search.parsedLabel")}
      className="flex flex-wrap items-center gap-1.5"
    >
      <span className="text-xs text-fg-muted">{t("photosApp.search.parsedLabel")}:</span>
      {chips.map((chip) => (
        <span
          key={chip.key}
          className="inline-flex items-center gap-1.5 rounded-full border border-line-strong bg-surface px-2.5 py-0.5 text-xs font-mid text-fg"
        >
          {chip.dot ? (
            <span
              aria-hidden
              className="size-2.5 rounded-full border border-black/15"
              style={{ backgroundColor: chip.dot }}
            />
          ) : null}
          {chip.label}
        </span>
      ))}
    </div>
  );
}

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

  const smartQueryText = search.q?.trim() ?? "";
  const isSmartSearch = smartQueryText.length > 0;

  const timeline = usePhotoTimeline({
    libraryId: search.libraryId,
    camera: search.camera,
    month: search.month,
    q: search.q,
  });

  const smartSearch = usePhotoSearch({ q: smartQueryText, limit: 200 }, isSmartSearch);
  const analyze = useAnalyzePhotos();

  const allItems = useMemo(
    () => timeline.data?.pages.flatMap((page) => page.items) ?? [],
    [timeline.data?.pages],
  );
  const smartItems = useMemo(() => {
    const raw = smartSearch.data?.items ?? [];
    return raw.filter((item) => {
      if (search.libraryId && item.libraryId !== search.libraryId) return false;
      if (search.month && item.monthBucket !== search.month) return false;
      return true;
    });
  }, [smartSearch.data?.items, search.libraryId, search.month]);

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
  const suggestions = currentLanguage() === "zh-CN" ? SMART_SUGGESTIONS_ZH : SMART_SUGGESTIONS_EN;
  const lightboxItems = isSmartSearch ? smartItems : allItems;

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

  const handleRunAnalysis = () => {
    analyze.mutate(undefined, {
      onSuccess: (res) => {
        toast.success(
          t("photosApp.ai.analyzeDone", {
            analyzed: res.analyzedCount,
            total: res.totalTagged,
            remaining: res.remaining,
          }),
        );
      },
      onError: (err) => toast.danger(errorText(t, err)),
    });
  };

  const selectedArray = useMemo(() => [...selectedIds], [selectedIds]);

  return (
    <div className="grid gap-5">
      <PageHeader
        title={t("photosApp.timeline.title")}
        summary={
          isSmartSearch && smartSearch.data
            ? t("photosApp.search.resultCount", { count: smartItems.length })
            : t("photosApp.timeline.totalCount", { count: total })
        }
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
              variant="outline"
              isDisabled={analyze.isPending}
              onPress={handleRunAnalysis}
            >
              <Sparkles className="size-4" aria-hidden />
              {analyze.isPending ? t("photosApp.ai.analyzing") : t("photosApp.ai.runAnalysis")}
            </Button>

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

      <div className="grid gap-2.5">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-56 flex-1 sm:max-w-md">
            <Search
              className="pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-fg-subtle"
              aria-hidden
            />
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

        <div
          aria-label={t("photosApp.search.suggestionsLabel")}
          className="flex flex-wrap items-center gap-1.5"
        >
          <span className="text-xs text-fg-subtle">{t("photosApp.search.suggestionsLabel")}:</span>
          {suggestions.map((preset) => (
            <button
              key={preset}
              type="button"
              onClick={() => updateSearch({ q: preset })}
              className="rounded-full border border-line bg-sunken px-2.5 py-0.5 text-xs text-fg-muted transition-colors hover:border-line-strong hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              {preset}
            </button>
          ))}
        </div>

        {isSmartSearch && smartSearch.data ? (
          <ParsedFilterChips filter={smartSearch.data.parsedFilters} />
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

      {isSmartSearch ? (
        smartSearch.isLoading ? (
          <LoadingRows rows={4} />
        ) : smartSearch.isError ? (
          <ErrorState error={smartSearch.error} onRetry={() => void smartSearch.refetch()} />
        ) : smartItems.length === 0 ? (
          <EmptyState
            icon={Images}
            title={t("photosApp.timeline.filteredEmptyTitle")}
            detail={t("photosApp.timeline.filteredEmptyDetail")}
            action={
              <Button size="sm" variant="outline" onPress={() => updateSearch({ q: undefined })}>
                {t("photosApp.search.clear")}
              </Button>
            }
          />
        ) : (
          <PhotoGrid
            items={smartItems}
            mode={gridMode}
            selectMode={selectMode}
            selectedIds={selectedIds}
            onToggleSelect={toggleSelect}
            onOpenPhoto={(idx) => setLightboxIndex(idx)}
          />
        )
      ) : timeline.isLoading ? (
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
                  !search.month
                    ? "bg-nav-active font-strong text-nav-active-fg"
                    : "text-fg-muted hover:bg-hover hover:text-fg",
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
        items={lightboxItems}
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
