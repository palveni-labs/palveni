import { Button, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { FolderHeart, FolderPlus, Pencil, Trash2, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "@/lib/errors";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import type { PhotoAlbumSummary } from "../api";
import { PhotoMosaic } from "../components/photo-image";
import {
  useCreatePhotoAlbum,
  useDeletePhotoAlbum,
  usePhotoAlbums,
  useUpdatePhotoAlbum,
} from "../queries";

export function PhotoAlbumsPage() {
  const { t } = useTranslation();
  const albumsQuery = usePhotoAlbums();
  const createAlbum = useCreatePhotoAlbum();
  const updateAlbum = useUpdatePhotoAlbum();
  const deleteAlbum = useDeletePhotoAlbum();

  const [createOpen, setCreateOpen] = useState(false);
  const [editing, setEditing] = useState<PhotoAlbumSummary | null>(null);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");

  const albums = albumsQuery.data?.albums ?? [];

  const openCreate = () => {
    setEditing(null);
    setTitle("");
    setDescription("");
    setCreateOpen(true);
  };

  const openEdit = (album: PhotoAlbumSummary) => {
    setEditing(album);
    setTitle(album.title);
    setDescription(album.description ?? "");
    setCreateOpen(true);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const cleanTitle = title.trim();
    if (!cleanTitle) return;
    if (editing) {
      updateAlbum.mutate(
        { albumId: editing.id, body: { title: cleanTitle, description } },
        {
          onSuccess: () => {
            toast.success(t("photosApp.albums.updatedToast"));
            setCreateOpen(false);
            setEditing(null);
          },
          onError: (err) => toast.danger(errorText(t, err)),
        },
      );
    } else {
      createAlbum.mutate(
        { title: cleanTitle, description: description.trim() || undefined },
        {
          onSuccess: (created) => {
            toast.success(t("photosApp.albums.createdToast", { title: created.title }));
            setCreateOpen(false);
          },
          onError: (err) => toast.danger(errorText(t, err)),
        },
      );
    }
  };

  const handleDelete = (album: PhotoAlbumSummary) => {
    deleteAlbum.mutate(album.id, {
      onSuccess: () => toast.success(t("photosApp.albums.deletedToast")),
      onError: (err) => toast.danger(errorText(t, err)),
    });
  };

  return (
    <div className="grid gap-5">
      <PageHeader
        title={t("photosApp.albums.title")}
        summary={t("photosApp.albums.count", { count: albums.length })}
        actions={
          <Button size="sm" variant="primary" onPress={openCreate}>
            <FolderPlus className="size-4" aria-hidden />
            {t("photosApp.albums.create")}
          </Button>
        }
      />

      {albumsQuery.isLoading ? (
        <LoadingRows rows={4} />
      ) : albumsQuery.isError ? (
        <ErrorState error={albumsQuery.error} onRetry={() => void albumsQuery.refetch()} />
      ) : albums.length === 0 ? (
        <EmptyState
          icon={FolderHeart}
          title={t("photosApp.albums.emptyTitle")}
          detail={t("photosApp.albums.emptyDetail")}
          action={
            <Button size="sm" variant="primary" onPress={openCreate}>
              <FolderPlus className="size-4" aria-hidden />
              {t("photosApp.albums.create")}
            </Button>
          }
        />
      ) : (
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
          {albums.map((album) => (
            <div
              key={album.id}
              className="group relative flex flex-col gap-2 rounded-panel border border-line bg-surface p-2.5 transition-colors hover:border-line-strong"
            >
              <Link
                to="/photos/albums/$albumId"
                params={{ albumId: album.id }}
                className="grid gap-2 outline-none focus-visible:ring-2 focus-visible:ring-focus"
              >
                <PhotoMosaic
                  coverItemId={album.coverItemId}
                  previewItemIds={album.previewItemIds}
                  className="aspect-square w-full"
                />
                <div className="grid min-w-0 gap-0.5 px-0.5">
                  <span className="truncate text-sm font-strong text-fg">{album.title}</span>
                  <span data-num className="text-xs text-fg-muted tabular-nums">
                    {t("photosApp.albums.itemCount", { count: album.itemCount })}
                  </span>
                </div>
              </Link>

              <div className="flex items-center justify-end gap-1 border-t border-line pt-1.5">
                <button
                  type="button"
                  aria-label={`${t("photosApp.albums.rename")} ${album.title}`}
                  onClick={() => openEdit(album)}
                  className="grid size-7 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                >
                  <Pencil className="size-3.5" aria-hidden />
                </button>
                <button
                  type="button"
                  aria-label={`${t("photosApp.albums.delete")} ${album.title}`}
                  onClick={() => handleDelete(album)}
                  className="grid size-7 place-items-center rounded-control text-fg-muted hover:bg-failed-soft hover:text-failed focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                >
                  <Trash2 className="size-3.5" aria-hidden />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {createOpen ? (
        <div
          className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
          onClick={(e) => {
            if (e.target === e.currentTarget) setCreateOpen(false);
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label={editing ? t("photosApp.albums.rename") : t("photosApp.albums.createTitle")}
            className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="font-heading text-lg font-strong text-heading">
                {editing ? t("photosApp.albums.rename") : t("photosApp.albums.createTitle")}
              </h2>
              <button
                type="button"
                aria-label={t("photosApp.albums.cancel")}
                onClick={() => setCreateOpen(false)}
                className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <X className="size-4" aria-hidden />
              </button>
            </div>

            <form onSubmit={handleSubmit} className="grid gap-3">
              <div className="grid gap-1.5">
                <label htmlFor="album-form-title" className="text-sm font-mid text-fg">
                  {t("photosApp.albums.nameLabel")}
                </label>
                <input
                  id="album-form-title"
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder={t("photosApp.albums.namePlaceholder")}
                  maxLength={128}
                  required
                  className="h-9 rounded-control border border-line-strong bg-surface px-3 text-sm text-fg placeholder:text-fg-subtle focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                />
              </div>

              <div className="grid gap-1.5">
                <label htmlFor="album-form-desc" className="text-sm font-mid text-fg">
                  {t("photosApp.albums.descriptionLabel")}
                </label>
                <textarea
                  id="album-form-desc"
                  rows={2}
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder={t("photosApp.albums.descriptionPlaceholder")}
                  maxLength={1024}
                  className="rounded-control border border-line-strong bg-surface p-2.5 text-sm text-fg placeholder:text-fg-subtle focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                />
              </div>

              <div className="flex justify-end gap-2 pt-1">
                <Button type="button" size="sm" variant="outline" onPress={() => setCreateOpen(false)}>
                  {t("photosApp.albums.cancel")}
                </Button>
                <Button
                  type="submit"
                  size="sm"
                  variant="primary"
                  isDisabled={!title.trim() || createAlbum.isPending || updateAlbum.isPending}
                >
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
