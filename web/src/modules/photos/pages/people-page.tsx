import { Button, toast } from "@heroui/react";
import { ArrowLeft, Pencil, Trash2, UserPlus, Users, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "@/lib/errors";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import type { PhotoPersonSummary } from "../api";
import { PhotoGrid, PhotoImage } from "../components/photo-image";
import { PhotoLightbox } from "../components/photo-lightbox";
import {
  useCreatePhotoPerson,
  useDeletePhotoPerson,
  usePhotoPeople,
  usePhotoSearch,
  useUntagPhotoPerson,
  useUpdatePhotoPerson,
} from "../queries";

export function PhotoPeoplePage() {
  const { t } = useTranslation();
  const peopleQuery = usePhotoPeople();
  const createPerson = useCreatePhotoPerson();
  const updatePerson = useUpdatePhotoPerson();
  const deletePerson = useDeletePhotoPerson();
  const untagPerson = useUntagPhotoPerson();

  const [selectedPersonId, setSelectedPersonId] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<PhotoPersonSummary | null>(null);
  const [nameInput, setNameInput] = useState("");
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);

  const people = peopleQuery.data?.people ?? [];
  const activePerson = people.find((p) => p.id === selectedPersonId) ?? null;

  const personPhotos = usePhotoSearch(
    { personId: activePerson?.id, limit: 200 },
    Boolean(activePerson),
  );
  const items = personPhotos.data?.items ?? [];

  const openCreate = () => {
    setEditing(null);
    setNameInput("");
    setModalOpen(true);
  };

  const openRename = (person: PhotoPersonSummary) => {
    setEditing(person);
    setNameInput(person.name);
    setModalOpen(true);
  };

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    const clean = nameInput.trim();
    if (!clean) return;
    if (editing) {
      updatePerson.mutate(
        { personId: editing.id, body: { name: clean } },
        {
          onSuccess: () => {
            toast.success(t("photosApp.people.updatedToast"));
            setModalOpen(false);
            setEditing(null);
          },
          onError: (err) => toast.danger(errorText(t, err)),
        },
      );
    } else {
      createPerson.mutate(
        { name: clean },
        {
          onSuccess: (created) => {
            toast.success(t("photosApp.people.createdToast", { name: created.name }));
            setModalOpen(false);
          },
          onError: (err) => toast.danger(errorText(t, err)),
        },
      );
    }
  };

  const handleDelete = (person: PhotoPersonSummary) => {
    deletePerson.mutate(person.id, {
      onSuccess: () => {
        if (selectedPersonId === person.id) setSelectedPersonId(null);
        toast.success(t("photosApp.people.deletedToast", { name: person.name }));
      },
      onError: (err) => toast.danger(errorText(t, err)),
    });
  };

  return (
    <PageFrame className="gap-5">
      {activePerson ? (
        <>
          <PageHeader
            title={activePerson.name}
            summary={t("photosApp.people.photoCount", { count: activePerson.photoCount })}
            actions={
              <div className="flex flex-wrap items-center gap-2">
                <Button size="sm" variant="outline" onPress={() => setSelectedPersonId(null)}>
                  <ArrowLeft className="size-4" aria-hidden />
                  {t("photosApp.people.backToList")}
                </Button>
                <Button size="sm" variant="outline" onPress={() => openRename(activePerson)}>
                  <Pencil className="size-4" aria-hidden />
                  {t("photosApp.people.rename")}
                </Button>
              </div>
            }
          />

          {personPhotos.isLoading ? (
            <LoadingRows rows={4} />
          ) : personPhotos.isError ? (
            <ErrorState error={personPhotos.error} onRetry={() => void personPhotos.refetch()} />
          ) : items.length === 0 ? (
            <EmptyState
              icon={Users}
              title={t("photosApp.people.emptyPersonTitle")}
              detail={t("photosApp.people.emptyPersonDetail")}
            />
          ) : (
            <div className="grid gap-4">
              <PhotoGrid
                items={items}
                mode="square"
                onOpenPhoto={(idx) => setLightboxIndex(idx)}
              />
              <div className="flex flex-wrap gap-2">
                {items.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() =>
                      untagPerson.mutate(
                        { itemId: item.id, personId: activePerson.id },
                        {
                          onSuccess: () => toast.success(t("photosApp.people.untaggedToast")),
                          onError: (err) => toast.danger(errorText(t, err)),
                        },
                      )
                    }
                    className="inline-flex items-center gap-1 rounded-control border border-line bg-surface px-2.5 py-1 text-xs text-fg-muted hover:border-line-strong hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                  >
                    <X className="size-3" aria-hidden />
                    <span>{t("photosApp.people.untagItem", { title: item.title })}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          <PhotoLightbox
            items={items}
            index={lightboxIndex}
            onIndexChange={setLightboxIndex}
            onClose={() => setLightboxIndex(null)}
          />
        </>
      ) : (
        <>
          <PageHeader
            title={t("photosApp.people.title")}
            summary={t("photosApp.people.count", { count: people.length })}
            actions={
              <Button size="sm" variant="primary" onPress={openCreate}>
                <UserPlus className="size-4" aria-hidden />
                {t("photosApp.people.create")}
              </Button>
            }
          />

          {peopleQuery.isLoading ? (
            <LoadingRows rows={4} />
          ) : peopleQuery.isError ? (
            <ErrorState error={peopleQuery.error} onRetry={() => void peopleQuery.refetch()} />
          ) : people.length === 0 ? (
            <EmptyState
              icon={Users}
              title={t("photosApp.people.emptyTitle")}
              detail={t("photosApp.people.emptyDetail")}
              action={
                <Button size="sm" variant="primary" onPress={openCreate}>
                  <UserPlus className="size-4" aria-hidden />
                  {t("photosApp.people.create")}
                </Button>
              }
            />
          ) : (
            <div className="pv-grid-square">
              {people.map((person) => (
                <article
                  key={person.id}
                  className="flex flex-col gap-2 rounded-panel border border-line bg-surface p-2.5 transition-colors hover:border-line-strong"
                >
                  <button
                    type="button"
                    onClick={() => setSelectedPersonId(person.id)}
                    className="grid gap-2 text-start outline-none focus-visible:ring-2 focus-visible:ring-focus"
                  >
                    <div className="aspect-square w-full overflow-hidden rounded-tile bg-sunken">
                      {person.coverItemId ? (
                        <PhotoImage
                          itemId={person.coverItemId}
                          size={480}
                          alt={person.name}
                          className="size-full"
                        />
                      ) : (
                        <div className="grid size-full place-items-center text-fg-subtle">
                          <Users className="size-10" aria-hidden />
                        </div>
                      )}
                    </div>
                    <div className="grid min-w-0 gap-0.5 px-0.5">
                      <span className="truncate text-sm font-strong text-fg">{person.name}</span>
                      <span data-num className="text-xs text-fg-muted tabular-nums">
                        {t("photosApp.people.photoCount", { count: person.photoCount })}
                      </span>
                    </div>
                  </button>

                  <div className="flex items-center justify-end gap-1 border-t border-line pt-1.5">
                    <button
                      type="button"
                      aria-label={`${t("photosApp.people.rename")} ${person.name}`}
                      onClick={() => openRename(person)}
                      className="grid size-7 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                    >
                      <Pencil className="size-3.5" aria-hidden />
                    </button>
                    <button
                      type="button"
                      aria-label={`${t("photosApp.people.delete")} ${person.name}`}
                      onClick={() => handleDelete(person)}
                      className="grid size-7 place-items-center rounded-control text-fg-muted hover:bg-failed-soft hover:text-failed focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                    >
                      <Trash2 className="size-3.5" aria-hidden />
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
        </>
      )}

      {modalOpen ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-label={editing ? t("photosApp.people.renameTitle") : t("photosApp.people.createTitle")}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
        >
          <form
            onSubmit={handleSubmit}
            className="grid w-full max-w-sm gap-4 rounded-panel border border-line-strong bg-raised p-5 text-fg shadow-panel"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="font-heading text-lg font-title text-heading">
                {editing ? t("photosApp.people.renameTitle") : t("photosApp.people.createTitle")}
              </h2>
              <button
                type="button"
                aria-label={t("photosApp.people.cancel")}
                onClick={() => setModalOpen(false)}
                className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              >
                <X className="size-4" aria-hidden />
              </button>
            </div>

            <div className="grid gap-1.5">
              <label htmlFor="photo-person-name" className="text-sm font-mid text-fg">
                {t("photosApp.people.nameLabel")}
              </label>
              <input
                id="photo-person-name"
                type="text"
                required
                maxLength={128}
                placeholder={t("photosApp.people.namePlaceholder")}
                value={nameInput}
                onChange={(e) => setNameInput(e.target.value)}
                className="h-9 rounded-control border border-line-strong bg-surface px-3 text-sm text-fg placeholder:text-fg-subtle focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              />
            </div>

            <div className="flex justify-end gap-2">
              <Button size="sm" variant="outline" onPress={() => setModalOpen(false)}>
                {t("photosApp.people.cancel")}
              </Button>
              <Button
                size="sm"
                variant="primary"
                type="submit"
                isDisabled={!nameInput.trim() || createPerson.isPending || updatePerson.isPending}
              >
                {t("photosApp.people.save")}
              </Button>
            </div>
          </form>
        </div>
      ) : null}
    </PageFrame>
  );
}
