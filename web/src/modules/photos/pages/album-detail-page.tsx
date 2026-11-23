import { Button, toast } from "@heroui/react";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { CheckSquare, FolderHeart, Pencil, Play, Trash2, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "@/lib/errors";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { PhotoGrid } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import {
  useDeletePhotoAlbum,
  usePhotoAlbum,
  usePhotoTimeline,
  useRemovePhotoAlbumItems,
  useUpdatePhotoAlbum,
} from "../queries";

const routeApi = getRouteApi("/_app/photos/albums/$albumId");

export function PhotoAlbumDetailPage() {
  const { t } = useTranslation();
  const { albumId } = routeApi.useParams();
  const navigate = useNavigate();

  const albumQuery = usePhotoAlbum(albumId);
  const timeline = usePhotoTimeline({ albumId });
  const updateAlbum = useUpdatePhotoAlbum();
  const deleteAlbum = useDeletePhotoAlbum();
  const removeItems = useRemovePhotoAlbumItems();

  const items = useMemo(
    () => timeline.data?.pages.flatMap((page) => page.items) ?? [],
    [timeline.data?.pages],
  );

  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const [lightboxState, setLightboxState] = useState<{ index: number; slideshow: boolean } | null>(null);
  const [renameOpen, setRenameOpen] = useState(false);
  const [editTitle, setEditTitle] = useState("");
  const [editDesc, setEditDesc] = useState("");

  if (albumQuery.isLoading || timeline.isLoading) {
    return <LoadingRows rows={4} />;
  }
  if (albumQuery.isError) {
    return <ErrorState error={albumQuery.error} onRetry={() => void albumQuery.refetch()} />;
  }
  const album = albumQuery.data;
  if (!album) {
    return <ErrorState error={new Error("Not found")} />;
  }

  const selectedArray = [...selectedIds];

  const handleToggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleRemoveSelected = () => {
    if (selectedArray.length === 0) return;
    removeItems.mutate(
      { albumId, itemIds: selectedArray },
      {
        onSuccess: () => {
          toast.success(t("photosApp.albums.removedToast", { count: selectedArray.length }));
          setSelectedIds(new Set());
          setSelectMode(false);
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleRenameSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = editTitle.trim();
    if (!trimmed) return;
    updateAlbum.mutate(
      { albumId, body: { title: trimmed, description: editDesc } },
      {
        onSuccess: () => {
          toast.success(t("photosApp.albums.updatedToast"));
          setRenameOpen(false);
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleDeleteAlbum = () => {
    deleteAlbum.mutate(albumId, {
      onSuccess: () => {
        toast.success(t("photosApp.albums.deletedToast"));
        void navigate({ to: "/photos/albums" });
      },
      onError: (err) => toast.danger(errorText(t, err)),
    });
  };

  return (
    <div className="grid gap-5">
      <PageHeader
        title={album.title}
        summary={
          album.description
            ? `${album.description} · ${t("photosApp.albums.itemCount", { count: album.itemCount })}`
            : t("photosApp.albums.itemCount", { count: album.itemCount })
        }
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {items.length > 0 ? (
              <>
                <Button
                  size="sm"
                  variant="primary"
                  onPress={() => setLightboxState({ index: 0, slideshow: true })}
                >
                  <Play className="size-4" aria-hidden />
                  {t("photosApp.albums.slideshow")}
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
              </>
            ) : null}

            <Button
              size="sm"
              variant="outline"
              onPress={() => {
                setEditTitle(album.title);
                setEditDesc(album.description ?? "");
                setRenameOpen(true);
              }}
            >
              <Pencil className="size-3.5" aria-hidden />
              {t("photosApp.albums.rename")}
            </Button>

            <Button size="sm" variant="outline" onPress={handleDeleteAlbum}>
              <Trash2 className="size-3.5" aria-hidden />
              {t("photosApp.albums.delete")}
            </Button>
          </div>
        }
      />

      {selectMode && selectedArray.length > 0 ? (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-panel border border-line-strong bg-surface px-4 py-2.5 shadow-panel">
          <span data-num className="text-sm font-strong text-fg tabular-nums">
            {t("photosApp.timeline.selectedCount", { count: selectedArray.length })}
          </span>
          <Button
            size="sm"
            variant="outline"
            isDisabled={removeItems.isPending}
            onPress={handleRemoveSelected}
          >
            <Trash2 className="size-4" aria-hidden />
            {t("photosApp.albums.removeSelected")}
          </Button>
        </div>
      ) : null}

      {items.length === 0 ? (
        <EmptyState
          icon={FolderHeart}
          title={t("photosApp.albums.emptyAlbumTitle")}
          detail={t("photosApp.albums.emptyAlbumDetail")}
        />
      ) : (
        <PhotoGrid
          items={items}
          mode="aspect"
          selectMode={selectMode}
          selectedIds={selectedIds}
          onToggleSelect={handleToggleSelect}
          onOpenPhoto={(idx) => setLightboxState({ index: idx, slideshow: false })}
        />
      )}

      <PhotoLightbox
        items={items}
        index={lightboxState?.index ?? null}
        initialSlideshow={lightboxState?.slideshow ?? false}
        onIndexChange={(nextIndex) =>
          setLightboxState((prev) => (prev ? { ...prev, index: nextIndex } : null))
        }
        onClose={() => setLightboxState(null)}
      />

      {renameOpen ? (
        <div
          className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
          onClick={(e) => {
            if (e.target === e.currentTarget) setRenameOpen(false);
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label={t("photosApp.albums.rename")}
            className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="font-heading text-lg font-strong text-heading">
                {t("photosApp.albums.rename")}
              </h2>
              <button
                type="button"
                aria-label={t("photosApp.albums.cancel")}
                onClick={() => setRenameOpen(false)}
                className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <X className="size-4" aria-hidden />
              </button>
            </div>
            <form onSubmit={handleRenameSubmit} className="grid gap-3">
              <div className="grid gap-1.5">
                <label htmlFor="album-detail-title" className="text-sm font-mid text-fg">
                  {t("photosApp.albums.nameLabel")}
                </label>
                <input
                  id="album-detail-title"
                  type="text"
                  value={editTitle}
                  onChange={(e) => setEditTitle(e.target.value)}
                  maxLength={128}
                  required
                  className="h-9 rounded-control border border-line-strong bg-surface px-3 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                />
              </div>
              <div className="grid gap-1.5">
                <label htmlFor="album-detail-desc" className="text-sm font-mid text-fg">
                  {t("photosApp.albums.descriptionLabel")}
                </label>
                <textarea
                  id="album-detail-desc"
                  rows={2}
                  value={editDesc}
                  onChange={(e) => setEditDesc(e.target.value)}
                  maxLength={1024}
                  className="rounded-control border border-line-strong bg-surface p-2.5 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                />
              </div>
              <div className="flex justify-end gap-2">
                <Button type="button" size="sm" variant="outline" onPress={() => setRenameOpen(false)}>
                  {t("photosApp.albums.cancel")}
                </Button>
                <Button type="submit" size="sm" variant="primary" isDisabled={!editTitle.trim() || updateAlbum.isPending}>
                  {t("photosApp.albums.save")}
                </Button>
              </div>
            </form>
          </div>
        </div>
      ) : null}
    </div>
  );
}
