import { Button } from "@heroui/react";
import { getRouteApi } from "@tanstack/react-router";
import { MapPin, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { PhotoGrid, PhotoImage } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import { WorldPlacesMap } from "../components/world-places-map";
import { usePhotoMemories, usePhotoTimeline } from "../queries";

const routeApi = getRouteApi("/_app/photos/places");

export function PhotoPlacesPage() {
  const { t } = useTranslation();
  const search = routeApi.useSearch();
  const navigate = routeApi.useNavigate();

  const memoriesQuery = usePhotoMemories();
  const places = memoriesQuery.data?.places ?? [];
  const activeKey = search.place ?? places[0]?.key ?? null;
  const activeCluster = places.find((c) => c.key === activeKey) ?? null;

  const timeline = usePhotoTimeline({
    hasLocation: true,
    place: activeCluster?.key,
  });
  const items = useMemo(
    () => timeline.data?.pages.flatMap((page) => page.items) ?? [],
    [timeline.data?.pages],
  );

  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);

  const selectPlace = (key: string | undefined) => {
    void navigate({ search: { place: key } });
  };

  return (
    <div className="grid gap-5">
      <PageHeader
        title={t("photosApp.places.title")}
        summary={t("photosApp.places.summary", {
          geotagged: memoriesQuery.data?.geotaggedCount ?? 0,
          clusters: places.length,
        })}
        actions={
          search.place ? (
            <Button size="sm" variant="outline" onPress={() => selectPlace(undefined)}>
              <X className="size-3.5" aria-hidden />
              {t("photosApp.places.allPlaces")}
            </Button>
          ) : null
        }
      />

      {memoriesQuery.isLoading ? (
        <LoadingRows rows={4} />
      ) : memoriesQuery.isError ? (
        <ErrorState error={memoriesQuery.error} onRetry={() => void memoriesQuery.refetch()} />
      ) : places.length === 0 ? (
        <EmptyState
          icon={MapPin}
          title={t("photosApp.places.emptyTitle")}
          detail={t("photosApp.places.emptyDetail")}
        />
      ) : (
        <div className="grid gap-5">
          <WorldPlacesMap
            places={places}
            selectedKey={activeCluster?.key ?? null}
            onSelectPlace={(key) => selectPlace(key)}
          />

          <div className="flex flex-wrap gap-2">
            {places.map((cluster) => {
              const selected = activeCluster?.key === cluster.key;
              return (
                <button
                  key={cluster.key}
                  type="button"
                  aria-pressed={selected}
                  onClick={() => selectPlace(cluster.key)}
                  className={cn(
                    "flex items-center gap-2.5 rounded-panel border px-2.5 py-1.5 text-start transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                    selected
                      ? "border-line-strong bg-nav-active text-nav-active-fg"
                      : "border-line bg-surface text-fg hover:bg-hover",
                  )}
                >
                  <PhotoImage itemId={cluster.coverItemId} size={96} className="size-9 rounded-control" />
                  <div className="grid min-w-0">
                    <span className="font-mono text-xs font-strong">{cluster.label}</span>
                    <span data-num className="text-xs opacity-75 tabular-nums">
                      {t("photosApp.albums.itemCount", { count: cluster.count })}
                    </span>
                  </div>
                </button>
              );
            })}
          </div>

          {activeCluster ? (
            <section
              aria-label={t("photosApp.places.selectedPlace", {
                label: activeCluster.label,
                count: activeCluster.count,
              })}
              className="grid gap-3"
            >
              <h2 className="font-heading text-base font-strong text-heading">
                {t("photosApp.places.selectedPlace", {
                  label: activeCluster.label,
                  count: activeCluster.count,
                })}
              </h2>
              {timeline.isLoading ? (
                <LoadingRows rows={3} />
              ) : (
                <PhotoGrid items={items} mode="square" onOpenPhoto={setLightboxIndex} />
              )}
            </section>
          ) : null}
        </div>
      )}

      <PhotoLightbox
        items={items}
        index={lightboxIndex}
        onIndexChange={setLightboxIndex}
        onClose={() => setLightboxIndex(null)}
      />
    </div>
  );
}
