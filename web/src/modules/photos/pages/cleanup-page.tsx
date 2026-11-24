import { Button, toast } from "@heroui/react";
import { CheckCircle2, CopyCheck, Eye, FolderPlus, ShieldCheck, Sparkles, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import type { PhotoDuplicateItem, PhotoTimelineItem } from "../api";
import { PhotoImage } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import { formatBytesCompact } from "../format";
import {
  useAddPhotoAlbumItems,
  useAnalyzePhotos,
  useCreatePhotoAlbum,
  usePhotoAlbums,
  usePhotoDuplicates,
} from "../queries";

const THRESHOLD_OPTIONS = [0, 4, 6, 10] as const;

export function PhotoCleanupPage() {
  const { t } = useTranslation();
  const [threshold, setThreshold] = useState<number>(6);
  const [markedIds, setMarkedIds] = useState<Set<string>>(() => new Set());
  const [previewItems, setPreviewItems] = useState<PhotoTimelineItem[]>([]);
  const [previewIndex, setPreviewIndex] = useState<number | null>(null);

  const dupsQuery = usePhotoDuplicates(threshold);
  const analyze = useAnalyzePhotos();
  const albumsQuery = usePhotoAlbums();
  const createAlbum = useCreatePhotoAlbum();
  const addAlbumItems = useAddPhotoAlbumItems();

  const data = dupsQuery.data;
  const clusters = data?.clusters ?? [];

  const markedArray = useMemo(() => [...markedIds], [markedIds]);

  const toggleMark = (item: PhotoDuplicateItem) => {
    setMarkedIds((prev) => {
      const next = new Set(prev);
      if (next.has(item.id)) next.delete(item.id);
      else next.add(item.id);
      return next;
    });
  };

  const handleAutoMarkAllDuplicates = () => {
    const next = new Set<string>();
    for (const cluster of clusters) {
      for (const item of cluster.items) {
        if (!item.recommendedKeep) next.add(item.id);
      }
    }
    setMarkedIds(next);
  };

  const handleStageMarkedToAlbum = async () => {
    if (markedArray.length === 0) return;
    try {
      const albumTitle = t("photosApp.cleanup.reviewAlbumName");
      const existing = (albumsQuery.data?.albums ?? []).find((a) => a.title === albumTitle);
      const targetId = existing
        ? existing.id
        : (
            await createAlbum.mutateAsync({
              title: albumTitle,
              description: t("photosApp.cleanup.reviewAlbumDesc"),
            })
          ).id;
      await addAlbumItems.mutateAsync({ albumId: targetId, itemIds: markedArray });
      toast.success(
        t("photosApp.cleanup.stagedToast", { count: markedArray.length, album: albumTitle }),
      );
      setMarkedIds(new Set());
    } catch (err) {
      toast.danger(errorText(t, err));
    }
  };

  const handleRunAnalyze = () => {
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

  return (
    <PageFrame className="gap-6">
      <PageHeader
        title={t("photosApp.cleanup.title")}
        summary={
          data
            ? t("photosApp.cleanup.summary", {
                clusters: data.totalClusters,
                photos: data.totalDuplicatePhotos,
                bytes: formatBytesCompact(data.totalReclaimableBytes),
              })
            : undefined
        }
        actions={
          clusters.length > 0 ? (
            <Button size="sm" variant="primary" onPress={handleAutoMarkAllDuplicates}>
              <CopyCheck className="size-4" aria-hidden />
              {t("photosApp.cleanup.selectNonKeep")}
            </Button>
          ) : null
        }
      />

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-panel border border-line/75 bg-surface px-4 py-3 shadow-panel">
        <label className="inline-flex items-center gap-2.5 text-sm font-mid text-fg-muted">
          <span>{t("photosApp.cleanup.thresholdLabel")}</span>
          <select
            aria-label={t("photosApp.cleanup.thresholdLabel")}
            value={threshold}
            onChange={(e) => setThreshold(Number(e.target.value))}
            className="h-9 rounded-control border border-line-control bg-field px-3 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            {THRESHOLD_OPTIONS.map((value) => (
              <option key={value} value={value}>
                {value === 0
                  ? t("photosApp.cleanup.thresholdExact")
                  : t("photosApp.cleanup.thresholdValue", { value })}
              </option>
            ))}
          </select>
        </label>

        <Button
          size="sm"
          variant="secondary"
          isDisabled={analyze.isPending}
          onPress={handleRunAnalyze}
        >
          <Sparkles className="size-4" aria-hidden />
          {analyze.isPending ? t("photosApp.ai.analyzing") : t("photosApp.ai.runAnalysis")}
        </Button>
      </div>

      {data ? (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <div className="rounded-panel border border-line/75 bg-surface px-4 py-3.5 shadow-panel">
            <p className="text-xs text-fg-muted">{t("photosApp.cleanup.statClusters")}</p>
            <p data-num className="mt-1 font-heading text-stat font-title text-heading tabular-nums">
              {data.totalClusters}
            </p>
          </div>
          <div className="rounded-panel border border-line/75 bg-surface px-4 py-3.5 shadow-panel">
            <p className="text-xs text-fg-muted">{t("photosApp.cleanup.statDuplicates")}</p>
            <p data-num className="mt-1 font-heading text-stat font-title text-heading tabular-nums">
              {data.totalDuplicatePhotos}
            </p>
          </div>
          <div className="rounded-panel border border-line/75 bg-surface px-4 py-3.5 shadow-panel">
            <p className="text-xs text-fg-muted">{t("photosApp.cleanup.statReclaimable")}</p>
            <p data-num className="mt-1 font-heading text-stat font-title text-ok tabular-nums">
              {formatBytesCompact(data.totalReclaimableBytes)}
            </p>
          </div>
        </div>
      ) : null}

      {markedArray.length > 0 ? (
        <div
          role="region"
          aria-label={t("photosApp.cleanup.markedCount", { count: markedArray.length })}
          className="flex flex-wrap items-center justify-between gap-3 rounded-panel border border-line-strong bg-surface px-4 py-2.5 shadow-panel"
        >
          <span data-num className="text-sm font-strong text-fg tabular-nums">
            {t("photosApp.cleanup.markedCount", { count: markedArray.length })}
          </span>
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm" variant="outline" onPress={() => setMarkedIds(new Set())}>
              <X className="size-3.5" aria-hidden />
              {t("photosApp.cleanup.clearMarked")}
            </Button>
            <Button
              size="sm"
              variant="primary"
              isDisabled={createAlbum.isPending || addAlbumItems.isPending}
              onPress={() => void handleStageMarkedToAlbum()}
            >
              <FolderPlus className="size-4" aria-hidden />
              {t("photosApp.cleanup.stageToAlbum")}
            </Button>
          </div>
        </div>
      ) : null}

      {dupsQuery.isLoading ? (
        <LoadingRows rows={4} />
      ) : dupsQuery.isError ? (
        <ErrorState error={dupsQuery.error} onRetry={() => void dupsQuery.refetch()} />
      ) : clusters.length === 0 ? (
        <EmptyState
          icon={CheckCircle2}
          title={t("photosApp.cleanup.emptyTitle")}
          detail={t("photosApp.cleanup.emptyDetail")}
          action={
            <Button
              size="sm"
              variant="outline"
              isDisabled={analyze.isPending}
              onPress={handleRunAnalyze}
            >
              <Sparkles className="size-4" aria-hidden />
              {t("photosApp.ai.runAnalysis")}
            </Button>
          }
        />
      ) : (
        <div className="grid gap-5">
          {clusters.map((cluster) => (
            <section
              key={cluster.clusterId}
              aria-label={cluster.clusterId}
              className="grid gap-3 rounded-panel border border-line bg-surface p-4"
            >
              <div className="flex flex-wrap items-center justify-between gap-2 border-b border-line pb-2.5">
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={cn(
                      "inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-xs font-strong",
                      cluster.kind === "exact"
                        ? "border-amber-600/30 bg-amber-500/14 text-amber-900 dark:text-amber-300"
                        : "border-sky-600/30 bg-sky-500/12 text-sky-900 dark:text-sky-300",
                    )}
                  >
                    {cluster.kind === "exact"
                      ? t("photosApp.cleanup.kindExact")
                      : t("photosApp.cleanup.kindSimilar", { distance: cluster.maxDistance })}
                  </span>
                  <span className="text-xs text-fg-muted tabular-nums">
                    {t("photosApp.cleanup.clusterMeta", {
                      count: cluster.items.length,
                      bytes: formatBytesCompact(cluster.reclaimableBytes),
                    })}
                  </span>
                </div>
              </div>

              <div className="pv-grid-card">
                {cluster.items.map((item, idx) => {
                  const marked = markedIds.has(item.id);
                  return (
                    <div
                      key={item.id}
                      className={cn(
                        "flex flex-col gap-2 rounded-tile border p-2.5 transition-colors",
                        item.recommendedKeep
                          ? "border-emerald-600/35 bg-emerald-500/6"
                          : marked
                            ? "border-amber-600/40 bg-amber-500/10"
                            : "border-line bg-sunken",
                      )}
                    >
                      <div className="relative aspect-square w-full overflow-hidden rounded-control bg-surface">
                        <PhotoImage
                          itemId={item.id}
                          size={480}
                          alt={item.title}
                          className="size-full"
                        />
                        {item.recommendedKeep ? (
                          <span className="absolute start-2 top-2 inline-flex items-center gap-1 rounded-full bg-emerald-700 px-2 py-0.5 text-[11px] font-strong text-white shadow-xs">
                            <ShieldCheck className="size-3" aria-hidden />
                            {t("photosApp.cleanup.keepBadge")}
                          </span>
                        ) : marked ? (
                          <span className="absolute start-2 top-2 inline-flex items-center gap-1 rounded-full bg-amber-700 px-2 py-0.5 text-[11px] font-strong text-white shadow-xs">
                            {t("photosApp.cleanup.markedBadge")}
                          </span>
                        ) : null}
                      </div>

                      <div className="grid min-w-0 gap-0.5 text-xs">
                        <span className="truncate font-strong text-fg" title={item.fileName}>
                          {item.fileName}
                        </span>
                        <span data-num className="text-fg-muted tabular-nums">
                          {item.width && item.height ? `${item.width} × ${item.height}` : "—"} ·{" "}
                          {formatBytesCompact(item.sizeBytes)}
                        </span>
                        <span data-num className="text-fg-subtle tabular-nums">
                          {item.dayBucket}
                        </span>
                      </div>

                      <div className="mt-auto flex items-center justify-between gap-1.5 pt-1">
                        <Button
                          size="sm"
                          variant="ghost"
                          onPress={() => {
                            setPreviewItems(cluster.items);
                            setPreviewIndex(idx);
                          }}
                        >
                          <Eye className="size-3.5" aria-hidden />
                          {t("photosApp.cleanup.preview")}
                        </Button>

                        {!item.recommendedKeep ? (
                          <Button
                            size="sm"
                            variant={marked ? "primary" : "outline"}
                            onPress={() => toggleMark(item)}
                          >
                            {marked
                              ? t("photosApp.cleanup.unmarkButton")
                              : t("photosApp.cleanup.markButton")}
                          </Button>
                        ) : null}
                      </div>
                    </div>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      )}

      <PhotoLightbox
        items={previewItems}
        index={previewIndex}
        onIndexChange={setPreviewIndex}
        onClose={() => setPreviewIndex(null)}
      />
    </PageFrame>
  );
}
