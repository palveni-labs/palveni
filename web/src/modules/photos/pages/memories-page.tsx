import { Button } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { Camera, Play, Sparkles } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PageFrame } from "@/ui/frame";
import { PageHeader, SummaryFacts } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import type { PhotoTimelineItem } from "../api";
import { PhotoGrid } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import { usePhotoMemories } from "../queries";

export function PhotoMemoriesPage() {
  const { t } = useTranslation();
  const memoriesQuery = usePhotoMemories();
  const [viewer, setViewer] = useState<{
    items: PhotoTimelineItem[];
    index: number;
    slideshow: boolean;
  } | null>(null);

  if (memoriesQuery.isLoading) {
    return (
      <PageFrame>
        <LoadingRows rows={4} />
      </PageFrame>
    );
  }
  if (memoriesQuery.isError) {
    return (
      <PageFrame>
        <ErrorState error={memoriesQuery.error} onRetry={() => void memoriesQuery.refetch()} />
      </PageFrame>
    );
  }

  const data = memoriesQuery.data;
  if (!data || data.totalPhotos === 0) {
    return (
      <PageFrame className="gap-5">
        <PageHeader title={t("photosApp.memories.title")} />
        <EmptyState
          icon={Sparkles}
          title={t("photosApp.timeline.emptyTitle")}
          detail={t("photosApp.timeline.emptyDetail")}
        />
      </PageFrame>
    );
  }

  return (
    <PageFrame className="gap-6">
      <PageHeader
        title={t("photosApp.memories.title")}
        actions={
          data.recentHighlights.length > 0 ? (
            <Button
              size="sm"
              variant="primary"
              onPress={() =>
                setViewer({
                  items: data.recentHighlights,
                  index: 0,
                  slideshow: true,
                })
              }
            >
              <Play className="size-4" aria-hidden />
              {t("photosApp.memories.playHighlights")}
            </Button>
          ) : null
        }
      />

      <SummaryFacts
        facts={[
          { label: t("photosApp.memories.totalPhotos"), value: data.totalPhotos },
          { label: t("photosApp.memories.geotagged"), value: data.geotaggedCount },
          { label: t("photosApp.memories.placesCount"), value: data.places.length },
          { label: t("photosApp.memories.camerasCount"), value: data.cameras.length },
        ]}
      />

      <section aria-label={t("photosApp.memories.onThisDay")} className="grid gap-3">
        <h2 className="font-heading text-base font-strong text-heading">
          {t("photosApp.memories.onThisDay")}
        </h2>
        {data.onThisDay.length === 0 ? (
          <p className="rounded-panel border border-line bg-surface px-4 py-5 text-sm text-fg-muted">
            {t("photosApp.memories.onThisDayEmpty")}
          </p>
        ) : (
          <PhotoGrid
            items={data.onThisDay}
            mode="aspect"
            onOpenPhoto={(index) =>
              setViewer({ items: data.onThisDay, index, slideshow: false })
            }
          />
        )}
      </section>

      {data.recentHighlights.length > 0 ? (
        <section aria-label={t("photosApp.memories.recentHighlights")} className="grid gap-3">
          <h2 className="font-heading text-base font-strong text-heading">
            {t("photosApp.memories.recentHighlights")}
          </h2>
          <PhotoGrid
            items={data.recentHighlights}
            mode="square"
            onOpenPhoto={(index) =>
              setViewer({ items: data.recentHighlights, index, slideshow: false })
            }
          />
        </section>
      ) : null}

      {data.cameras.length > 0 ? (
        <section aria-label={t("photosApp.memories.cameras")} className="grid gap-3">
          <h2 className="font-heading text-base font-strong text-heading">
            {t("photosApp.memories.cameras")}
          </h2>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 md:grid-cols-3">
            {data.cameras.map((cam) => (
              <Link
                key={cam.name}
                to="/photos"
                search={{ camera: cam.name }}
                className="flex items-center justify-between gap-3 rounded-panel border border-line bg-surface px-3.5 py-2.5 text-sm transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <span className="flex min-w-0 items-center gap-2.5">
                  <Camera className="size-4 shrink-0 text-fg-muted" aria-hidden />
                  <span className="truncate font-mid text-fg">{cam.name}</span>
                </span>
                <span data-num className="shrink-0 text-xs text-fg-muted tabular-nums">
                  {t("photosApp.memories.cameraCount", { count: cam.count })}
                </span>
              </Link>
            ))}
          </div>
        </section>
      ) : null}

      <PhotoLightbox
        items={viewer?.items ?? []}
        index={viewer?.index ?? null}
        initialSlideshow={viewer?.slideshow ?? false}
        onIndexChange={(nextIndex) =>
          setViewer((prev) => (prev ? { ...prev, index: nextIndex } : null))
        }
        onClose={() => setViewer(null)}
      />
    </PageFrame>
  );
}
