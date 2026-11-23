import { Button } from "@heroui/react";
import { Grid2X2, Heart, LayoutGrid, Play } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { PhotoGrid } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import { usePhotoTimeline } from "../queries";

export function PhotoFavoritesPage() {
  const { t } = useTranslation();
  const timeline = usePhotoTimeline({ favorite: true });
  const items = useMemo(
    () => timeline.data?.pages.flatMap((page) => page.items) ?? [],
    [timeline.data?.pages],
  );
  const total = timeline.data?.pages[0]?.total ?? 0;

  const [gridMode, setGridMode] = useState<"square" | "aspect">("aspect");
  const [viewer, setViewer] = useState<{ index: number; slideshow: boolean } | null>(null);

  return (
    <div className="grid gap-5">
      <PageHeader
        title={t("photosApp.favorites.title")}
        summary={t("photosApp.favorites.count", { count: total })}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <div className="inline-flex rounded-control border border-line bg-sunken p-0.5">
              <button
                type="button"
                aria-label={t("photosApp.timeline.gridSquare")}
                aria-pressed={gridMode === "square"}
                onClick={() => setGridMode("square")}
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
                onClick={() => setGridMode("aspect")}
                className={cn(
                  "grid size-7 place-items-center rounded-control transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  gridMode === "aspect" ? "bg-surface text-fg shadow-xs" : "text-fg-muted hover:text-fg",
                )}
              >
                <LayoutGrid className="size-4" aria-hidden />
              </button>
            </div>
            {items.length > 0 ? (
              <Button
                size="sm"
                variant="primary"
                onPress={() => setViewer({ index: 0, slideshow: true })}
              >
                <Play className="size-4" aria-hidden />
                {t("photosApp.favorites.slideshow")}
              </Button>
            ) : null}
          </div>
        }
      />

      {timeline.isLoading ? (
        <LoadingRows rows={4} />
      ) : timeline.isError ? (
        <ErrorState error={timeline.error} onRetry={() => void timeline.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState
          icon={Heart}
          title={t("photosApp.favorites.emptyTitle")}
          detail={t("photosApp.favorites.emptyDetail")}
        />
      ) : (
        <PhotoGrid
          items={items}
          mode={gridMode}
          onOpenPhoto={(index) => setViewer({ index, slideshow: false })}
        />
      )}

      <PhotoLightbox
        items={items}
        index={viewer?.index ?? null}
        initialSlideshow={viewer?.slideshow ?? false}
        onIndexChange={(nextIndex) =>
          setViewer((prev) => (prev ? { ...prev, index: nextIndex } : null))
        }
        onClose={() => setViewer(null)}
      />
    </div>
  );
}
