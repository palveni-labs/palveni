import { Button, toast } from "@heroui/react";
import { FolderPlus, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "@/lib/errors";
import { useAddPhotoAlbumItems, useCreatePhotoAlbum, usePhotoAlbums } from "../queries";

export function AddToAlbumModal({
  itemIds,
  open,
  onClose,
  onCompleted,
}: {
  itemIds: string[];
  open: boolean;
  onClose: () => void;
  onCompleted?: () => void;
}) {
  const { t } = useTranslation();
  const { data } = usePhotoAlbums();
  const addItems = useAddPhotoAlbumItems();
  const createAlbum = useCreatePhotoAlbum();
  const [newTitle, setNewTitle] = useState("");

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, onClose]);

  if (!open || itemIds.length === 0) return null;

  const albums = data?.albums ?? [];
  const busy = addItems.isPending || createAlbum.isPending;

  const handlePickExisting = (albumId: string, title: string) => {
    addItems.mutate(
      { albumId, itemIds },
      {
        onSuccess: () => {
          toast.success(t("photosApp.albums.addedToast", { title }));
          onCompleted?.();
          onClose();
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleCreateAndAdd = (e: React.FormEvent) => {
    e.preventDefault();
    const title = newTitle.trim();
    if (!title) return;
    createAlbum.mutate(
      { title, coverItemId: itemIds[0] },
      {
        onSuccess: (created) => {
          addItems.mutate(
            { albumId: created.id, itemIds },
            {
              onSuccess: () => {
                setNewTitle("");
                toast.success(t("photosApp.albums.addedToast", { title: created.title }));
                onCompleted?.();
                onClose();
              },
              onError: (err) => toast.danger(errorText(t, err)),
            },
          );
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t("photosApp.albums.addModalTitle")}
        className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
      >
        <div className="flex items-center justify-between gap-2">
          <h2 className="font-heading text-lg font-strong text-heading">
            {t("photosApp.albums.addModalTitle")}
          </h2>
          <button
            type="button"
            aria-label={t("photosApp.albums.cancel")}
            onClick={onClose}
            className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>

        {albums.length > 0 ? (
          <div className="grid max-h-52 gap-1.5 overflow-y-auto">
            {albums.map((album) => (
              <button
                key={album.id}
                type="button"
                disabled={busy}
                onClick={() => handlePickExisting(album.id, album.title)}
                className="flex items-center justify-between rounded-control border border-line px-3 py-2 text-start text-sm transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus) disabled:opacity-50"
              >
                <span className="truncate font-mid text-fg">{album.title}</span>
                <span data-num className="shrink-0 text-xs text-fg-muted tabular-nums">
                  {t("photosApp.albums.itemCount", { count: album.itemCount })}
                </span>
              </button>
            ))}
          </div>
        ) : null}

        <form onSubmit={handleCreateAndAdd} className="grid gap-2.5 border-t border-line pt-3">
          <label htmlFor="new-photo-album-title" className="text-sm font-mid text-fg">
            {t("photosApp.albums.orCreateNew")}
          </label>
          <div className="flex gap-2">
            <input
              id="new-photo-album-title"
              type="text"
              value={newTitle}
              onChange={(e) => setNewTitle(e.target.value)}
              placeholder={t("photosApp.albums.namePlaceholder")}
              maxLength={128}
              className="pv-field min-w-0 flex-1"
            />
            <Button type="submit" size="sm" variant="primary" isDisabled={!newTitle.trim() || busy}>
              <FolderPlus className="size-4" aria-hidden />
              {t("photosApp.albums.create")}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
