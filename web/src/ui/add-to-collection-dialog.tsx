import { Button, toast } from "@heroui/react";
import { Layers, Plus, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useAddCollectionItem,
  useCollections,
  useCreateCollection,
  type AddMediaCollectionItemRequest,
} from "@/api/queries/shares-collections";
import { errorText } from "@/lib/errors";
import { FieldInput } from "@/ui/controls";

export interface AddToCollectionDialogProps {
  isOpen: boolean;
  onClose: () => void;
  item: AddMediaCollectionItemRequest;
}

export function AddToCollectionDialog({ isOpen, onClose, item }: AddToCollectionDialogProps) {
  const { t } = useTranslation();
  const collectionsQuery = useCollections(isOpen);
  const createCollection = useCreateCollection();
  const addItem = useAddCollectionItem();

  const [newTitle, setNewTitle] = useState("");

  if (!isOpen) return null;

  const collections = collectionsQuery.data?.collections ?? [];

  const handleAddTo = (collectionId: string) => {
    addItem.mutate(
      { collectionId, body: item },
      {
        onSuccess: () => {
          toast.success(t("collectionsApp.addedToast"));
          onClose();
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleCreateAndAdd = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = newTitle.trim();
    if (!trimmed) return;
    createCollection.mutate(
      { title: trimmed },
      {
        onSuccess: (created) => {
          setNewTitle("");
          handleAddTo(created.id);
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
        aria-label={t("collectionsApp.pickCollectionTitle")}
        className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
      >
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Layers className="size-4 text-fg-muted" aria-hidden />
            <h2 className="font-heading text-lg font-strong text-heading">
              {t("collectionsApp.pickCollectionTitle")}
            </h2>
          </div>
          <button
            type="button"
            aria-label={t("a11y.close")}
            onClick={onClose}
            className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>

        {collections.length > 0 ? (
          <ul className="grid max-h-56 gap-1.5 overflow-y-auto">
            {collections.map((coll) => (
              <li key={coll.id}>
                <button
                  type="button"
                  disabled={addItem.isPending}
                  onClick={() => handleAddTo(coll.id)}
                  className="flex w-full items-center justify-between gap-3 rounded-control border border-line bg-surface px-3 py-2.5 text-start text-sm transition-colors hover:bg-hover"
                >
                  <span className="truncate font-mid text-fg">{coll.title}</span>
                  <span data-num className="text-xs text-fg-muted tabular-nums">
                    {t("collectionsApp.itemCount", { count: coll.itemCount })}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        ) : null}

        <form onSubmit={handleCreateAndAdd} className="flex items-center gap-2 border-t border-line pt-3">
          <FieldInput
            type="text"
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            placeholder={t("collectionsApp.namePlaceholder")}
            className="min-w-0 flex-1"
          />
          <Button
            type="submit"
            size="sm"
            variant="primary"
            isDisabled={!newTitle.trim() || createCollection.isPending || addItem.isPending}
          >
            <Plus className="size-4" aria-hidden />
            {t("collectionsApp.createButton")}
          </Button>
        </form>
      </div>
    </div>
  );
}
