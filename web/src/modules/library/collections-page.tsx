import { Button, toast } from "@heroui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Layers, Plus, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useCollections,
  useCreateCollection,
} from "@/api/queries/shares-collections";
import { errorText } from "@/lib/errors";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { CoverMosaic } from "@/ui/work";

export function CollectionsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const query = useCollections();
  const createCollection = useCreateCollection();

  const [createOpen, setCreateOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");

  const collections = query.data?.collections ?? [];

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = title.trim();
    if (!trimmed) return;
    createCollection.mutate(
      { title: trimmed, description: description.trim() || null },
      {
        onSuccess: (created) => {
          toast.success(t("collectionsApp.createdToast"));
          setTitle("");
          setDescription("");
          setCreateOpen(false);
          void navigate({
            to: "/library/collections/$collectionId",
            params: { collectionId: created.id },
          });
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  return (
    <PageFrame>
      <PageHeader
        title={t("collectionsApp.title")}
        summary={t("collectionsApp.summary")}
        actions={
          <Button size="sm" variant="primary" onPress={() => setCreateOpen(true)}>
            <Plus className="size-4" aria-hidden />
            {t("collectionsApp.createButton")}
          </Button>
        }
      />

      {query.isPending ? (
        <LoadingRows rows={4} />
      ) : query.isError ? (
        <ErrorState error={query.error} onRetry={() => void query.refetch()} />
      ) : collections.length === 0 ? (
        <EmptyState
          icon={Layers}
          title={t("collectionsApp.emptyTitle")}
          detail={t("collectionsApp.emptyDetail")}
          action={
            <Button variant="primary" onPress={() => setCreateOpen(true)}>
              <Plus className="size-4" aria-hidden />
              {t("collectionsApp.createButton")}
            </Button>
          }
        />
      ) : (
        <ul className="pv-grid-card">
          {collections.map((coll) => (
            <li key={coll.id}>
              <Link
                to="/library/collections/$collectionId"
                params={{ collectionId: coll.id }}
                className="group flex flex-col overflow-hidden rounded-panel border border-line bg-surface shadow-panel transition-colors hover:border-line-strong hover:bg-hover/40 focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <CoverMosaic
                  artworkIds={coll.previewArtworkIds}
                  title={coll.title}
                  icon={Layers}
                  className="aspect-[16/10] w-full border-b border-line"
                />
                <div className="grid gap-2 p-4">
                  <div className="flex items-baseline justify-between gap-2">
                    <h2 className="truncate font-heading text-base font-strong text-heading group-hover:underline">
                      {coll.title}
                    </h2>
                    <span data-num className="shrink-0 text-xs text-fg-muted tabular-nums">
                      {t("collectionsApp.itemCount", { count: coll.itemCount })}
                    </span>
                  </div>

                  {coll.description ? (
                    <p className="line-clamp-2 text-xs text-fg-muted">{coll.description}</p>
                  ) : null}

                  {coll.kinds.length > 0 ? (
                    <div className="flex flex-wrap gap-1.5 pt-0.5">
                      {coll.kinds.map((k) => (
                        <span
                          key={k}
                          className="rounded-chip bg-sunken px-2 py-0.5 text-[0.7rem] font-mid text-fg-muted"
                        >
                          {t(`collectionsApp.entryKinds.${k}`)}
                        </span>
                      ))}
                    </div>
                  ) : null}
                </div>
              </Link>
            </li>
          ))}
        </ul>
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
            aria-label={t("collectionsApp.createDialogTitle")}
            className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="font-heading text-lg font-strong text-heading">
                {t("collectionsApp.createDialogTitle")}
              </h2>
              <button
                type="button"
                aria-label={t("a11y.close")}
                onClick={() => setCreateOpen(false)}
                className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <X className="size-4" aria-hidden />
              </button>
            </div>

            <form onSubmit={handleCreate} className="grid gap-3.5">
              <div className="grid gap-1.5">
                <label htmlFor="collection-name-input" className="text-sm font-mid text-fg">
                  {t("collectionsApp.nameLabel")}
                </label>
                <input
                  id="collection-name-input"
                  type="text"
                  autoFocus
                  required
                  maxLength={255}
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder={t("collectionsApp.namePlaceholder")}
                  className="pv-field w-full"
                />
              </div>

              <div className="grid gap-1.5">
                <label htmlFor="collection-desc-input" className="text-sm font-mid text-fg">
                  {t("collectionsApp.descLabel")}
                </label>
                <textarea
                  id="collection-desc-input"
                  rows={3}
                  maxLength={2000}
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder={t("collectionsApp.descPlaceholder")}
                  className="pv-textarea"
                />
              </div>

              <div className="flex justify-end gap-2 pt-1">
                <Button size="sm" variant="ghost" onPress={() => setCreateOpen(false)}>
                  {t("common.cancel")}
                </Button>
                <Button
                  type="submit"
                  size="sm"
                  variant="primary"
                  isDisabled={!title.trim() || createCollection.isPending}
                >
                  {t("common.create")}
                </Button>
              </div>
            </form>
          </div>
        </div>
      ) : null}
    </PageFrame>
  );
}
