import { Button, toast } from "@heroui/react";
import { useNavigate, useParams } from "@tanstack/react-router";
import {
  ArrowDown,
  ArrowUp,
  BookOpen,
  Disc3,
  Film,
  Image as ImageIcon,
  Layers,
  Music2,
  Pencil,
  Plus,
  Search,
  Share2,
  Sparkles,
  Trash2,
  X,
   type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useAddCollectionItem,
  useCollection,
  useDeleteCollection,
  useGlobalSearch,
  useRemoveCollectionItem,
  useUpdateCollection,
  type GlobalSearchHit,
  type MediaCollectionEntryKind,
  type RelatedWork,
} from "@/api/queries/shares-collections";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows } from "@/ui/section";
import { ShareDialog } from "@/ui/share-dialog";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { Cover } from "@/ui/work";

const KIND_ICONS: Record<MediaCollectionEntryKind, LucideIcon> = {
  video: Film,
  music_album: Disc3,
  track: Music2,
  book: BookOpen,
  comic: BookOpen,
  photo: ImageIcon,
};

function mapSearchHitToEntryKind(hit: GlobalSearchHit): MediaCollectionEntryKind | null {
  switch (hit.kind) {
    case "video_work":
      return "video";
    case "music_album":
      return "music_album";
    case "track":
      return "track";
    case "book":
      return "book";
    case "comic":
      return "comic";
    case "photo":
      return "photo";
    default:
      return null;
  }
}

export function CollectionDetailPage() {
  const { t } = useTranslation();
  const { collectionId } = useParams({ from: "/_app/library/collections/$collectionId" });
  const navigate = useNavigate();
  const sharesEnabled = useFeature("shares");
  const searchEnabled = useFeature("globalSearch");

  const detailQuery = useCollection(collectionId);
  const updateCollection = useUpdateCollection();
  const deleteCollection = useDeleteCollection();
  const addItem = useAddCollectionItem();
  const removeItem = useRemoveCollectionItem();

  const [shareOpen, setShareOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [editTitle, setEditTitle] = useState("");
  const [editDesc, setEditDesc] = useState("");
  const [pickerOpen, setPickerOpen] = useState(false);
  const [pickerQuery, setPickerQuery] = useState("");

  const searchQuery = useGlobalSearch(pickerQuery, pickerOpen && searchEnabled, 16);

  if (detailQuery.isPending) {
    return (
      <PageFrame>
        <LoadingRows rows={5} />
      </PageFrame>
    );
  }
  if (detailQuery.isError) {
    return (
      <PageFrame>
        <ErrorState error={detailQuery.error} onRetry={() => void detailQuery.refetch()} />
      </PageFrame>
    );
  }

  const detail = detailQuery.data;
  if (!detail) return null;
  const { collection, items, autoRelated } = detail;

  const navigateToTarget = (
    kind: MediaCollectionEntryKind,
    targetId: string,
    libraryId?: string | null,
    title?: string,
  ) => {
    if (kind === "video" && libraryId) {
      void navigate({
        to: "/video/$libraryId/works/$workId",
        params: { libraryId, workId: targetId },
      });
    } else if (kind === "music_album" && libraryId) {
      void navigate({
        to: "/music/albums/$libraryId/$albumId",
        params: { libraryId, albumId: targetId },
      });
    } else if (kind === "track") {
      void navigate({
        to: "/music/songs",
        search: { q: title },
      });
    } else if (kind === "book") {
      void navigate({ to: "/reading/book/$itemId", params: { itemId: targetId } });
    } else if (kind === "comic") {
      void navigate({ to: "/reading/comic/$itemId", params: { itemId: targetId } });
    } else if (kind === "photo") {
      void navigate({
        to: "/photos",
        search: { libraryId: libraryId ?? undefined, q: title },
      });
    }
  };

  const handleMove = (index: number, delta: -1 | 1) => {
    const nextIdx = index + delta;
    if (nextIdx < 0 || nextIdx >= items.length) return;
    const order = items.map((i) => i.id);
    const [moved] = order.splice(index, 1);
    if (!moved) return;
    order.splice(nextIdx, 0, moved);
    updateCollection.mutate({ id: collection.id, body: { itemOrder: order } });
  };

  const handleAddRelated = (work: RelatedWork) => {
    addItem.mutate(
      {
        collectionId: collection.id,
        body: {
          entryKind: work.entryKind,
          targetId: work.targetId,
          libraryId: work.libraryId,
          title: work.title,
          subtitle: work.subtitle ?? null,
          artworkId: work.artworkId ?? null,
        },
      },
      {
        onSuccess: () => toast.success(t("collectionsApp.addedToast")),
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleAddFromSearch = (hit: GlobalSearchHit) => {
    const entryKind = mapSearchHitToEntryKind(hit);
    if (!entryKind) return;
    addItem.mutate(
      {
        collectionId: collection.id,
        body: {
          entryKind,
          targetId: hit.id,
          libraryId: hit.libraryId ?? null,
          title: hit.title,
          subtitle: hit.subtitle ?? null,
          artworkId: hit.artworkId ?? null,
        },
      },
      {
        onSuccess: () => toast.success(t("collectionsApp.addedToast")),
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleEditSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!editTitle.trim()) return;
    updateCollection.mutate(
      {
        id: collection.id,
        body: { title: editTitle.trim(), description: editDesc },
      },
      {
        onSuccess: () => {
          toast.success(t("collectionsApp.updatedToast"));
          setEditOpen(false);
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleDelete = () => {
    deleteCollection.mutate(collection.id, {
      onSuccess: () => {
        toast.success(t("collectionsApp.deletedToast"));
        void navigate({ to: "/library/collections" });
      },
      onError: (err) => toast.danger(errorText(t, err)),
    });
  };

  const addableHits = (searchQuery.data?.hits ?? []).filter(
    (h) => mapSearchHitToEntryKind(h) !== null,
  );

  return (
    <PageFrame>
      <PageHeader
        title={collection.title}
        summary={
          collection.description
            ? `${collection.description} · ${t("collectionsApp.itemCount", { count: items.length })}`
            : t("collectionsApp.itemCount", { count: items.length })
        }
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {searchEnabled ? (
              <Button size="sm" variant="primary" onPress={() => setPickerOpen(true)}>
                <Plus className="size-4" aria-hidden />
                {t("collectionsApp.addWorkButton")}
              </Button>
            ) : null}
            {sharesEnabled ? (
              <Button size="sm" variant="outline" onPress={() => setShareOpen(true)}>
                <Share2 className="size-3.5" aria-hidden />
                {t("sharesApp.button")}
              </Button>
            ) : null}
            <Button
              size="sm"
              variant="outline"
              onPress={() => {
                setEditTitle(collection.title);
                setEditDesc(collection.description ?? "");
                setEditOpen(true);
              }}
            >
              <Pencil className="size-3.5" aria-hidden />
              {t("collectionsApp.editCollection")}
            </Button>
            <Button size="sm" variant="ghost" onPress={handleDelete}>
              <Trash2 className="size-3.5" aria-hidden />
              {t("collectionsApp.deleteCollection")}
            </Button>
          </div>
        }
      />

      {/* Manual Collection Items */}
      <Section title={t("collectionsApp.itemsSection")} count={items.length}>
        {items.length === 0 ? (
          <EmptyState
            icon={Layers}
            title={t("collectionsApp.emptyItemsTitle")}
            detail={t("collectionsApp.emptyItemsDetail")}
          />
        ) : (
          <SectionRows>
            {items.map((entry, idx) => {
              const Icon = KIND_ICONS[entry.entryKind];
              return (
                <li
                  key={entry.id}
                  className="flex items-center gap-3.5 px-4 py-3"
                >
                  <button
                    type="button"
                    onClick={() =>
                      navigateToTarget(
                        entry.entryKind,
                        entry.targetId,
                        entry.libraryId,
                        entry.title,
                      )
                    }
                    className="flex min-w-0 flex-1 items-center gap-3.5 text-start outline-none focus-visible:underline"
                  >
                    <Cover
                      artworkId={entry.artworkId}
                      title={entry.title}
                      icon={Icon}
                      decorative
                      className="size-12 shrink-0 rounded-tile"
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="truncate font-strong text-heading hover:underline">
                          {entry.title}
                        </span>
                        <span className="rounded-chip bg-sunken px-2 py-0.5 text-xs font-mid text-fg-muted">
                          {t(`collectionsApp.entryKinds.${entry.entryKind}`)}
                        </span>
                      </div>
                      {entry.subtitle ? (
                        <p className="truncate text-xs text-fg-muted">{entry.subtitle}</p>
                      ) : null}
                    </div>
                  </button>

                  <div className="flex shrink-0 items-center gap-1">
                    <Button
                      size="sm"
                      variant="ghost"
                      isIconOnly
                      isDisabled={idx === 0 || updateCollection.isPending}
                      aria-label={t("collectionsApp.moveUp")}
                      onPress={() => handleMove(idx, -1)}
                    >
                      <ArrowUp className="size-3.5" aria-hidden />
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      isIconOnly
                      isDisabled={idx === items.length - 1 || updateCollection.isPending}
                      aria-label={t("collectionsApp.moveDown")}
                      onPress={() => handleMove(idx, 1)}
                    >
                      <ArrowDown className="size-3.5" aria-hidden />
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      isIconOnly
                      isDisabled={removeItem.isPending}
                      aria-label={t("collectionsApp.removeEntry")}
                      onPress={() =>
                        removeItem.mutate(
                          { collectionId: collection.id, entryId: entry.id },
                          {
                            onSuccess: () => toast.success(t("collectionsApp.removedToast")),
                            onError: (err) => toast.danger(errorText(t, err)),
                          },
                        )
                      }
                    >
                      <Trash2 className="size-3.5" aria-hidden />
                    </Button>
                  </div>
                </li>
              );
            })}
          </SectionRows>
        )}
      </Section>

      {/* Auto-Discovered Cross-Media Works */}
      {autoRelated.length > 0 ? (
        <Section
          title={t("collectionsApp.autoRelatedSection")}
          count={autoRelated.length}
          actions={
            <span className="inline-flex items-center gap-1 text-xs text-fg-muted">
              <Sparkles className="size-3.5" aria-hidden />
              {t("collectionsApp.autoRelatedHint")}
            </span>
          }
        >
          <ul className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {autoRelated.map((work) => {
              const Icon = KIND_ICONS[work.entryKind];
              return (
                <li
                  key={`${work.entryKind}:${work.libraryId}:${work.targetId}`}
                  className="flex items-center gap-3 rounded-panel border border-line bg-surface p-3 shadow-panel"
                >
                  <button
                    type="button"
                    onClick={() =>
                      navigateToTarget(work.entryKind, work.targetId, work.libraryId, work.title)
                    }
                    className="flex min-w-0 flex-1 items-center gap-3 text-start"
                  >
                    <Cover
                      artworkId={work.artworkId}
                      title={work.title}
                      icon={Icon}
                      decorative
                      className="size-12 shrink-0 rounded-tile"
                    />
                    <div className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-strong text-heading hover:underline">
                        {work.title}
                      </span>
                      <span className="block truncate text-xs text-fg-muted">
                        {[t(`collectionsApp.entryKinds.${work.entryKind}`), work.subtitle]
                          .filter(Boolean)
                          .join(" · ")}
                      </span>
                    </div>
                  </button>
                  <Button
                    size="sm"
                    variant="outline"
                    isDisabled={addItem.isPending}
                    onPress={() => handleAddRelated(work)}
                  >
                    <Plus className="size-3.5" aria-hidden />
                    {t("collectionsApp.addToCollection")}
                  </Button>
                </li>
              );
            })}
          </ul>
        </Section>
      ) : null}

      <ShareDialog
        isOpen={shareOpen}
        onClose={() => setShareOpen(false)}
        targetKind="collection"
        targetId={collection.id}
        defaultTitle={collection.title}
      />

      {/* Search & Add Item Modal */}
      {pickerOpen ? (
        <div
          className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
          onClick={(e) => {
            if (e.target === e.currentTarget) setPickerOpen(false);
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label={t("collectionsApp.addWorkButton")}
            className="grid w-full max-w-lg gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="font-heading text-lg font-strong text-heading">
                {t("collectionsApp.addWorkButton")}
              </h2>
              <button
                type="button"
                aria-label={t("a11y.close")}
                onClick={() => setPickerOpen(false)}
                className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg"
              >
                <X className="size-4" aria-hidden />
              </button>
            </div>

            <div className="relative flex items-center">
              <Search className="pointer-events-none absolute start-3 size-4 text-fg-muted" aria-hidden />
              <input
                type="search"
                autoFocus
                value={pickerQuery}
                onChange={(e) => setPickerQuery(e.target.value)}
                placeholder={t("collectionsApp.searchToAddPlaceholder")}
                className="h-10 w-full rounded-control border border-line-strong bg-surface ps-9 pe-3 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              />
            </div>

            <ul className="grid max-h-72 gap-1.5 overflow-y-auto">
              {addableHits.map((hit) => {
                const ek = mapSearchHitToEntryKind(hit)!;
                const Icon = KIND_ICONS[ek];
                return (
                  <li
                    key={`${hit.kind}:${hit.id}`}
                    className="flex items-center justify-between gap-3 rounded-control border border-line bg-surface px-3 py-2"
                  >
                    <div className="flex min-w-0 items-center gap-2.5">
                      <Icon className="size-4 shrink-0 text-fg-muted" aria-hidden />
                      <div className="min-w-0">
                        <p className="truncate text-sm font-mid text-fg">{hit.title}</p>
                        <p className="truncate text-xs text-fg-muted">
                          {[t(`globalSearch.kinds.${hit.kind}`), hit.subtitle]
                            .filter(Boolean)
                            .join(" · ")}
                        </p>
                      </div>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      isDisabled={addItem.isPending}
                      onPress={() => handleAddFromSearch(hit)}
                    >
                      <Plus className="size-3.5" aria-hidden />
                      {t("common.create")}
                    </Button>
                  </li>
                );
              })}
            </ul>
          </div>
        </div>
      ) : null}

      {/* Edit Collection Modal */}
      {editOpen ? (
        <div
          className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
          onClick={(e) => {
            if (e.target === e.currentTarget) setEditOpen(false);
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label={t("collectionsApp.editDialogTitle")}
            className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="font-heading text-lg font-strong text-heading">
                {t("collectionsApp.editDialogTitle")}
              </h2>
              <button
                type="button"
                aria-label={t("a11y.close")}
                onClick={() => setEditOpen(false)}
                className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg"
              >
                <X className="size-4" aria-hidden />
              </button>
            </div>
            <form onSubmit={handleEditSubmit} className="grid gap-3">
              <div className="grid gap-1.5">
                <label htmlFor="edit-coll-title" className="text-sm font-mid text-fg">
                  {t("collectionsApp.nameLabel")}
                </label>
                <input
                  id="edit-coll-title"
                  type="text"
                  required
                  maxLength={255}
                  value={editTitle}
                  onChange={(e) => setEditTitle(e.target.value)}
                  className="h-9 rounded-control border border-line-strong bg-surface px-3 text-sm text-fg"
                />
              </div>
              <div className="grid gap-1.5">
                <label htmlFor="edit-coll-desc" className="text-sm font-mid text-fg">
                  {t("collectionsApp.descLabel")}
                </label>
                <textarea
                  id="edit-coll-desc"
                  rows={3}
                  maxLength={2000}
                  value={editDesc}
                  onChange={(e) => setEditDesc(e.target.value)}
                  className="rounded-control border border-line-strong bg-surface p-3 text-sm text-fg"
                />
              </div>
              <div className="flex justify-end gap-2 pt-1">
                <Button size="sm" variant="ghost" onPress={() => setEditOpen(false)}>
                  {t("common.cancel")}
                </Button>
                <Button type="submit" size="sm" variant="primary" isDisabled={updateCollection.isPending}>
                  {t("collectionsApp.save")}
                </Button>
              </div>
            </form>
          </div>
        </div>
      ) : null}
    </PageFrame>
  );
}
