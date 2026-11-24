import { toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import {
  Check,
  ChevronLeft,
  ChevronRight,
  Download,
  FolderPlus,
  Heart,
  Info,
  Layers,
  Pause,
  Play,
  Share2,
  UserPlus,
  Users,
  X,
  ZoomIn,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { AddToCollectionDialog } from "@/ui/add-to-collection-dialog";
import { ShareDialog } from "@/ui/share-dialog";
import { photosApi, type PhotoTimelineItem } from "../api";
import { formatBytesCompact, formatCameraLabel } from "../format";
import {
  useCreatePhotoPerson,
  usePhotoInfo,
  usePhotoItemPeople,
  usePhotoPeople,
  useTagPhotoPerson,
  useTogglePhotoFavorite,
  useUntagPhotoPerson,
} from "../queries";
import { AddToAlbumModal } from "./add-to-album-modal";
import { PhotoImage } from "./photo-image";

const SLIDESHOW_INTERVAL_MS = 4_000;

export function PhotoLightbox({
  items,
  index,
  initialSlideshow = false,
  onIndexChange,
  onClose,
}: {
  items: readonly PhotoTimelineItem[];
  index: number | null;
  initialSlideshow?: boolean;
  onIndexChange: (nextIndex: number) => void;
  onClose: () => void;
}) {
  if (index === null || !items[index]) return null;
  return (
    <PhotoLightboxSession
      key={initialSlideshow ? "slideshow" : "viewer"}
      items={items}
      index={index}
      initialSlideshow={initialSlideshow}
      onIndexChange={onIndexChange}
      onClose={onClose}
    />
  );
}

interface TransformState {
  photoId: string;
  zoom: number;
  panX: number;
  panY: number;
}

function PhotoLightboxSession({
  items,
  index,
  initialSlideshow,
  onIndexChange,
  onClose,
}: {
  items: readonly PhotoTimelineItem[];
  index: number;
  initialSlideshow: boolean;
  onIndexChange: (nextIndex: number) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const current = items[index]!;
  const [transform, setTransform] = useState<TransformState>(() => ({
    photoId: current.id,
    zoom: 1,
    panX: 0,
    panY: 0,
  }));
  const [dragging, setDragging] = useState(false);
  const dragStart = useRef({ x: 0, y: 0, panX: 0, panY: 0 });
  const [slideshow, setSlideshow] = useState(initialSlideshow);
  const [infoOpen, setInfoOpen] = useState(false);
  const [peopleOpen, setPeopleOpen] = useState(false);
  const [newPersonName, setNewPersonName] = useState("");
  const [albumModalOpen, setAlbumModalOpen] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const [collectionOpen, setCollectionOpen] = useState(false);
  const sharesEnabled = useFeature("shares");
  const collectionsEnabled = useFeature("collections");
  const modalPaused = albumModalOpen || shareOpen || collectionOpen;

  const zoom = transform.photoId === current.id ? transform.zoom : 1;
  const panX = transform.photoId === current.id ? transform.panX : 0;
  const panY = transform.photoId === current.id ? transform.panY : 0;

  const toggleFav = useTogglePhotoFavorite();
  const { data: detail } = usePhotoInfo(current.id, infoOpen);
  const { data: peopleList } = usePhotoPeople(peopleOpen);
  const { data: itemPeopleData } = usePhotoItemPeople(current.id, peopleOpen || infoOpen);
  const createPerson = useCreatePhotoPerson();
  const tagPerson = useTagPhotoPerson();
  const untagPerson = useUntagPhotoPerson();

  const itemPeople = itemPeopleData?.people ?? [];
  const taggedPersonIds = new Set(itemPeople.map((p) => p.id));

  const handleTogglePersonTag = (personId: string, name: string) => {
    if (taggedPersonIds.has(personId)) {
      untagPerson.mutate(
        { itemId: current.id, personId },
        {
          onSuccess: () => toast.success(t("photosApp.people.untaggedToast")),
          onError: (err) => toast.danger(errorText(t, err)),
        },
      );
    } else {
      tagPerson.mutate(
        { itemId: current.id, personId },
        {
          onSuccess: () => toast.success(t("photosApp.people.taggedToast", { name })),
          onError: (err) => toast.danger(errorText(t, err)),
        },
      );
    }
  };

  const handleQuickCreateAndTag = async (e: React.FormEvent) => {
    e.preventDefault();
    const clean = newPersonName.trim();
    if (!clean) return;
    try {
      const created = await createPerson.mutateAsync({ name: clean });
      await tagPerson.mutateAsync({ itemId: current.id, personId: created.id });
      setNewPersonName("");
      toast.success(t("photosApp.people.taggedToast", { name: created.name }));
    } catch (err) {
      toast.danger(errorText(t, err));
    }
  };

  const goPrev = useCallback(() => {
    if (items.length === 0) return;
    onIndexChange((index - 1 + items.length) % items.length);
  }, [index, items.length, onIndexChange]);

  const goNext = useCallback(() => {
    if (items.length === 0) return;
    onIndexChange((index + 1) % items.length);
  }, [index, items.length, onIndexChange]);

  useEffect(() => {
    if (!slideshow || items.length <= 1 || modalPaused) return;
    const timer = window.setInterval(() => {
      goNext();
    }, SLIDESHOW_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [slideshow, items.length, modalPaused, goNext]);

  useEffect(() => {
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = prevOverflow;
    };
  }, []);

  useEffect(() => {
    if (modalPaused) return;
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT")) {
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        goPrev();
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        goNext();
      } else if (e.key === " ") {
        e.preventDefault();
        setSlideshow((prev) => !prev);
      } else if (e.key === "i" || e.key === "I") {
        e.preventDefault();
        setInfoOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [modalPaused, onClose, goPrev, goNext]);

  const isFavorite = detail?.favorite ?? current.favorite;
  const cameraText = formatCameraLabel(
    detail?.cameraMake ?? current.cameraMake,
    detail?.cameraModel ?? current.cameraModel,
  );

  const handleToggleZoom = () => {
    if (zoom > 1.05) {
      setTransform({ photoId: current.id, zoom: 1, panX: 0, panY: 0 });
    } else {
      setTransform({ photoId: current.id, zoom: 2, panX: 0, panY: 0 });
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY < 0 ? 0.25 : -0.25;
    const next = Math.min(4, Math.max(1, Number((zoom + delta).toFixed(2))));
    setTransform({
      photoId: current.id,
      zoom: next,
      panX: next === 1 ? 0 : panX,
      panY: next === 1 ? 0 : panY,
    });
  };

  const handleDownload = async () => {
    try {
      const blob = await photosApi.originalBlob(current.id);
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = detail?.fileName ?? `${current.title}.jpg`;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 5_000);
    } catch (err) {
      toast.danger(errorText(t, err));
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("photosApp.lightbox.dialogLabel")}
      className="fixed inset-0 z-40 flex flex-col bg-neutral-950/96 text-neutral-100 select-none"
    >
      <header className="flex min-h-14 shrink-0 flex-wrap items-center justify-between gap-2 border-b border-white/10 bg-neutral-950/90 px-3 py-2 backdrop-blur-md sm:px-5">
        <div className="flex min-w-0 items-center gap-3">
          <span className="truncate text-sm font-strong text-white">{current.title}</span>
          <span data-num className="shrink-0 text-xs text-neutral-400 tabular-nums">
            {t("photosApp.lightbox.counter", { index: index + 1, total: items.length })}
          </span>
          {slideshow ? (
            <span className="rounded-full bg-white/15 px-2.5 py-0.5 text-xs font-mid text-white">
              {t("photosApp.lightbox.slideshowPlaying")}
            </span>
          ) : null}
        </div>

        <div className="flex flex-wrap items-center gap-1">
          <button
            type="button"
            aria-label={
              slideshow ? t("photosApp.lightbox.slideshowPause") : t("photosApp.lightbox.slideshowPlay")
            }
            aria-pressed={slideshow}
            onClick={() => setSlideshow((prev) => !prev)}
            className={cn(
              "inline-flex h-8 items-center gap-1.5 rounded-control px-2.5 text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-white",
              slideshow ? "bg-white text-neutral-950" : "text-neutral-200 hover:bg-white/12",
            )}
          >
            {slideshow ? <Pause className="size-3.5" aria-hidden /> : <Play className="size-3.5" aria-hidden />}
          </button>

          <button
            type="button"
            aria-label={zoom > 1.05 ? t("photosApp.lightbox.zoomOne") : t("photosApp.lightbox.zoomTwo")}
            aria-pressed={zoom > 1.05}
            onClick={handleToggleZoom}
            className={cn(
              "inline-flex h-8 items-center gap-1 rounded-control px-2.5 text-xs font-mid tabular-nums transition-colors focus-visible:outline-2 focus-visible:outline-white",
              zoom > 1.05 ? "bg-white text-neutral-950" : "text-neutral-200 hover:bg-white/12",
            )}
          >
            <ZoomIn className="size-3.5" aria-hidden />
            <span>{zoom > 1.05 ? "2×" : "1×"}</span>
          </button>

          <button
            type="button"
            aria-label={isFavorite ? t("photosApp.lightbox.favorited") : t("photosApp.lightbox.favorite")}
            aria-pressed={isFavorite}
            onClick={() =>
              toggleFav.mutate(
                { itemId: current.id, favorite: !isFavorite },
                { onError: (err) => toast.danger(errorText(t, err)) },
              )
            }
            className="grid size-8 place-items-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
          >
            <Heart className={cn("size-4", isFavorite && "fill-current text-amber-400")} aria-hidden />
          </button>

          <button
            type="button"
            aria-label={t("photosApp.lightbox.addToAlbum")}
            onClick={() => setAlbumModalOpen(true)}
            className="grid size-8 place-items-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
          >
            <FolderPlus className="size-4" aria-hidden />
          </button>

          {collectionsEnabled ? (
            <button
              type="button"
              aria-label={t("collectionsApp.addToCollection")}
              onClick={() => setCollectionOpen(true)}
              className="grid size-8 place-items-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
            >
              <Layers className="size-4" aria-hidden />
            </button>
          ) : null}

          {sharesEnabled ? (
            <button
              type="button"
              aria-label={t("sharesApp.button")}
              onClick={() => setShareOpen(true)}
              className="grid size-8 place-items-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
            >
              <Share2 className="size-4" aria-hidden />
            </button>
          ) : null}

          <button
            type="button"
            aria-label={t("photosApp.lightbox.tagPeople")}
            aria-pressed={peopleOpen}
            onClick={() => setPeopleOpen((prev) => !prev)}
            className={cn(
              "grid size-8 place-items-center rounded-control transition-colors focus-visible:outline-2 focus-visible:outline-white",
              peopleOpen ? "bg-white text-neutral-950" : "text-neutral-200 hover:bg-white/12",
            )}
          >
            <Users className="size-4" aria-hidden />
          </button>

          <button
            type="button"
            aria-label={t("photosApp.lightbox.downloadOriginal")}
            onClick={() => void handleDownload()}
            className="grid size-8 place-items-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
          >
            <Download className="size-4" aria-hidden />
          </button>

          <button
            type="button"
            aria-label={t("photosApp.lightbox.infoToggle")}
            aria-pressed={infoOpen}
            onClick={() => setInfoOpen((prev) => !prev)}
            className={cn(
              "grid size-8 place-items-center rounded-control transition-colors focus-visible:outline-2 focus-visible:outline-white",
              infoOpen ? "bg-white text-neutral-950" : "text-neutral-200 hover:bg-white/12",
            )}
          >
            <Info className="size-4" aria-hidden />
          </button>

          <button
            type="button"
            aria-label={t("photosApp.lightbox.close")}
            onClick={onClose}
            className="grid size-8 place-items-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-white"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>
      </header>

      <div className="relative flex min-h-0 flex-1 overflow-hidden">
        <div
          onWheel={handleWheel}
          onPointerDown={(e) => {
            if (zoom <= 1) return;
            setDragging(true);
            dragStart.current = { x: e.clientX, y: e.clientY, panX, panY };
            e.currentTarget.setPointerCapture(e.pointerId);
          }}
          onPointerMove={(e) => {
            if (!dragging) return;
            setTransform({
              photoId: current.id,
              zoom,
              panX: dragStart.current.panX + (e.clientX - dragStart.current.x),
              panY: dragStart.current.panY + (e.clientY - dragStart.current.y),
            });
          }}
          onPointerUp={() => setDragging(false)}
          onPointerCancel={() => setDragging(false)}
          className={cn(
            "relative flex min-w-0 flex-1 items-center justify-center overflow-hidden p-2 sm:p-6",
            zoom > 1 ? (dragging ? "cursor-grabbing" : "cursor-grab") : "cursor-default",
          )}
        >
          <div
            style={{
              transform: `translate3d(${panX}px, ${panY}px, 0) scale(${zoom})`,
            }}
            className="size-full transition-transform duration-100 motion-reduce:transition-none"
          >
            <PhotoImage
              itemId={current.id}
              size={1920}
              alt={current.title}
              eager
              fit="contain"
              className="size-full bg-transparent text-neutral-600"
            />
          </div>

          {items.length > 1 ? (
            <>
              <button
                type="button"
                aria-label={t("photosApp.lightbox.previous")}
                onClick={goPrev}
                className="absolute start-2 top-1/2 grid size-10 -translate-y-1/2 place-items-center rounded-full bg-black/65 text-white transition-colors hover:bg-black/85 focus-visible:outline-2 focus-visible:outline-white sm:start-4"
              >
                <ChevronLeft className="size-5" aria-hidden />
              </button>
              <button
                type="button"
                aria-label={t("photosApp.lightbox.next")}
                onClick={goNext}
                className="absolute end-2 top-1/2 grid size-10 -translate-y-1/2 place-items-center rounded-full bg-black/65 text-white transition-colors hover:bg-black/85 focus-visible:outline-2 focus-visible:outline-white sm:end-4"
              >
                <ChevronRight className="size-5" aria-hidden />
              </button>
            </>
          ) : null}
        </div>

        {peopleOpen ? (
          <aside
            aria-label={t("photosApp.people.tagTitle")}
            className="flex w-full shrink-0 flex-col gap-3 overflow-y-auto border-s border-white/12 bg-neutral-900 px-4 py-4 text-sm text-neutral-100 sm:w-72"
          >
            <h2 className="text-base font-strong text-white">{t("photosApp.people.tagTitle")}</h2>
            {(peopleList?.people ?? []).length === 0 ? (
              <p className="text-xs text-neutral-400">{t("photosApp.people.noPeopleYet")}</p>
            ) : (
              <ul className="grid gap-1.5">
                {(peopleList?.people ?? []).map((person) => {
                  const isTagged = taggedPersonIds.has(person.id);
                  return (
                    <li key={person.id}>
                      <button
                        type="button"
                        aria-pressed={isTagged}
                        disabled={tagPerson.isPending || untagPerson.isPending}
                        onClick={() => handleTogglePersonTag(person.id, person.name)}
                        className={cn(
                          "flex w-full items-center justify-between rounded-control border px-3 py-2 text-start text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-white",
                          isTagged
                            ? "border-emerald-400/50 bg-emerald-500/20 text-white"
                            : "border-white/12 bg-white/5 text-neutral-200 hover:bg-white/10",
                        )}
                      >
                        <span className="truncate">{person.name}</span>
                        {isTagged ? <Check className="size-3.5 shrink-0 text-emerald-300" aria-hidden /> : null}
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}

            <form onSubmit={(e) => void handleQuickCreateAndTag(e)} className="mt-auto grid gap-2 border-t border-white/10 pt-3">
              <input
                type="text"
                aria-label={t("photosApp.people.quickCreatePlaceholder")}
                placeholder={t("photosApp.people.quickCreatePlaceholder")}
                value={newPersonName}
                onChange={(e) => setNewPersonName(e.target.value)}
                className="h-8 rounded-control border border-white/20 bg-neutral-950 px-2.5 text-xs text-white placeholder:text-neutral-500 focus-visible:outline-2 focus-visible:outline-white"
              />
              <button
                type="submit"
                disabled={!newPersonName.trim() || createPerson.isPending || tagPerson.isPending}
                className="inline-flex h-8 items-center justify-center gap-1.5 rounded-control bg-white px-3 text-xs font-strong text-neutral-950 transition-opacity hover:opacity-90 disabled:opacity-40"
              >
                <UserPlus className="size-3.5" aria-hidden />
                {t("photosApp.people.quickCreateAdd")}
              </button>
            </form>
          </aside>
        ) : null}

        {infoOpen ? (
          <aside
            aria-label={t("photosApp.lightbox.exif.title")}
            className="w-full shrink-0 overflow-y-auto border-s border-white/12 bg-neutral-900 px-4 py-4 text-sm text-neutral-100 sm:w-80"
          >
            <h2 className="mb-3 text-base font-strong text-white">{t("photosApp.lightbox.exif.title")}</h2>
            <dl className="grid gap-2.5">
              <ExifRow label={t("photosApp.lightbox.exif.takenAt")} value={new Date(detail?.takenAt ?? current.takenAt).toLocaleString()} />
              <ExifRow label={t("photosApp.lightbox.exif.fileName")} value={detail?.fileName ?? current.title} />
              <ExifRow label={t("photosApp.lightbox.exif.path")} value={detail?.path ?? "—"} mono />
              <ExifRow label={t("photosApp.lightbox.exif.fileSize")} value={formatBytesCompact(detail?.sizeBytes)} mono />
              <ExifRow
                label={t("photosApp.lightbox.exif.dimensions")}
                value={
                  (detail?.width ?? current.width) && (detail?.height ?? current.height)
                    ? `${detail?.width ?? current.width} × ${detail?.height ?? current.height}`
                    : t("photosApp.lightbox.exif.none")
                }
                mono
              />
              <ExifRow
                label={t("photosApp.lightbox.exif.orientation")}
                value={String(detail?.orientation ?? current.orientation)}
                mono
              />
              <ExifRow label={t("photosApp.lightbox.exif.camera")} value={cameraText ?? t("photosApp.lightbox.exif.none")} />
              <ExifRow label={t("photosApp.lightbox.exif.lens")} value={detail?.lensModel ?? t("photosApp.lightbox.exif.none")} />
              <ExifRow
                label={t("photosApp.lightbox.exif.iso")}
                value={detail?.iso != null ? `ISO ${detail.iso}` : t("photosApp.lightbox.exif.none")}
                mono
              />
              <ExifRow
                label={t("photosApp.lightbox.exif.aperture")}
                value={detail?.fNumber != null ? `f/${detail.fNumber}` : t("photosApp.lightbox.exif.none")}
                mono
              />
              <ExifRow
                label={t("photosApp.lightbox.exif.shutter")}
                value={detail?.exposureTime ?? t("photosApp.lightbox.exif.none")}
                mono
              />
              <ExifRow
                label={t("photosApp.lightbox.exif.focalLength")}
                value={detail?.focalLength != null ? `${detail.focalLength} mm` : t("photosApp.lightbox.exif.none")}
                mono
              />
              <ExifRow
                label={t("photosApp.lightbox.exif.coordinates")}
                value={
                  (detail?.latitude ?? current.latitude) != null && (detail?.longitude ?? current.longitude) != null
                    ? `${(detail?.latitude ?? current.latitude ?? 0).toFixed(4)}, ${(detail?.longitude ?? current.longitude ?? 0).toFixed(4)}`
                    : t("photosApp.lightbox.exif.none")
                }
                mono
              />
              {itemPeople.length > 0 ? (
                <div className="grid gap-1 border-t border-white/10 pt-2">
                  <dt className="text-xs text-neutral-400">{t("photosApp.lightbox.exif.people")}</dt>
                  <dd className="flex flex-wrap gap-1.5">
                    {itemPeople.map((p) => (
                      <Link
                        key={p.id}
                        to="/photos/people"
                        onClick={onClose}
                        className="rounded-control bg-white/12 px-2 py-0.5 text-xs text-white hover:bg-white/20"
                      >
                        {p.name}
                      </Link>
                    ))}
                  </dd>
                </div>
              ) : null}
              {detail?.albums && detail.albums.length > 0 ? (
                <div className="grid gap-1 border-t border-white/10 pt-2">
                  <dt className="text-xs text-neutral-400">{t("photosApp.lightbox.exif.albums")}</dt>
                  <dd className="flex flex-wrap gap-1.5">
                    {detail.albums.map((alb) => (
                      <Link
                        key={alb.id}
                        to="/photos/albums/$albumId"
                        params={{ albumId: alb.id }}
                        onClick={onClose}
                        className="rounded-control bg-white/12 px-2 py-0.5 text-xs text-white hover:bg-white/20"
                      >
                        {alb.title}
                      </Link>
                    ))}
                  </dd>
                </div>
              ) : null}
            </dl>
          </aside>
        ) : null}
      </div>

      <AddToAlbumModal
        itemIds={[current.id]}
        open={albumModalOpen}
        onClose={() => setAlbumModalOpen(false)}
      />

      {shareOpen ? (
        <ShareDialog
          isOpen
          onClose={() => setShareOpen(false)}
          targetKind="item"
          targetId={current.id}
          defaultTitle={current.title}
        />
      ) : null}

      <AddToCollectionDialog
        isOpen={collectionOpen}
        onClose={() => setCollectionOpen(false)}
        item={{
          entryKind: "photo",
          targetId: current.id,
          title: current.title,
          subtitle: cameraText || null,
          artworkId: `ph_${current.id}`,
        }}
      />
    </div>
  );
}

function ExifRow({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="grid gap-0.5 border-b border-white/8 pb-2 last:border-b-0">
      <dt className="text-xs text-neutral-400">{label}</dt>
      <dd className={cn("break-all text-neutral-100", mono && "font-mono text-xs tabular-nums")}>{value}</dd>
    </div>
  );
}
