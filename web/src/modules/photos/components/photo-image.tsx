import { Check, Heart, Image as ImageIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { PhotoThumbSize, PhotoTimelineItem } from "../api";
import { orientedAspectRatio } from "../format";
import { usePhotoUrl } from "../photo-cache";

export function PhotoImage({
  itemId,
  size = 480,
  alt = "",
  eager = false,
  fit = "cover",
  className,
}: {
  itemId: string | null | undefined;
  size?: PhotoThumbSize;
  alt?: string;
  eager?: boolean;
  fit?: "cover" | "contain";
  className?: string;
}) {
  const hostRef = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(() => eager || typeof IntersectionObserver === "undefined");
  const [brokenUrl, setBrokenUrl] = useState<string | null>(null);

  useEffect(() => {
    if (eager || visible || !itemId || typeof IntersectionObserver === "undefined") return;
    const node = hostRef.current;
    if (!node) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "280px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [eager, visible, itemId]);

  const url = usePhotoUrl(itemId, size, visible);
  const broken = Boolean(url && brokenUrl === url);

  return (
    <span
      ref={hostRef}
      className={cn(
        "relative grid shrink-0 place-items-center overflow-hidden bg-sunken text-fg-subtle select-none",
        className,
      )}
    >
      <ImageIcon className="size-6 opacity-55" strokeWidth={1.6} aria-hidden />
      {url && !broken ? (
        <img
          src={url}
          alt={alt}
          draggable={false}
          decoding="async"
          onError={() => setBrokenUrl(url)}
          className={cn(
            "absolute inset-0 size-full",
            fit === "contain" ? "object-contain" : "object-cover",
          )}
        />
      ) : null}
    </span>
  );
}

/** 2×2 mosaic cover for photo albums, falling back to single cover or placeholder. */
export function PhotoMosaic({
  coverItemId,
  previewItemIds,
  className,
}: {
  coverItemId?: string | null;
  previewItemIds: readonly string[];
  className?: string;
}) {
  const ids: string[] = [];
  for (const id of previewItemIds) {
    if (id && !ids.includes(id)) ids.push(id);
    if (ids.length === 4) break;
  }
  if (ids.length >= 4) {
    return (
      <span className={cn("grid grid-cols-2 grid-rows-2 gap-0.5 overflow-hidden rounded-tile bg-sunken", className)}>
        {ids.map((id) => (
          <PhotoImage key={id} itemId={id} size={240} className="size-full" />
        ))}
      </span>
    );
  }
  const single = coverItemId ?? ids[0] ?? null;
  return <PhotoImage itemId={single} size={480} className={cn("rounded-tile", className)} />;
}

export function PhotoGrid({
  items,
  mode = "square",
  selectMode = false,
  selectedIds,
  onToggleSelect,
  onOpenPhoto,
}: {
  items: readonly PhotoTimelineItem[];
  mode?: "square" | "aspect";
  selectMode?: boolean;
  selectedIds?: ReadonlySet<string>;
  onToggleSelect?: (id: string) => void;
  onOpenPhoto: (index: number) => void;
}) {
  const { t } = useTranslation();

  if (mode === "aspect") {
    return (
      <div className="flex flex-wrap gap-2 after:flex-[999_1_auto] after:content-['']">
        {items.map((photo, index) => {
          const ratio = orientedAspectRatio(photo);
          const selected = Boolean(selectedIds?.has(photo.id));
          const basisPx = Math.round(180 * ratio);
          return (
            <button
              key={photo.id}
              type="button"
              aria-label={photo.title}
              aria-pressed={selectMode ? selected : undefined}
              onClick={() => {
                if (selectMode && onToggleSelect) onToggleSelect(photo.id);
                else onOpenPhoto(index);
              }}
              style={{
                flexGrow: ratio,
                flexBasis: `${basisPx}px`,
                aspectRatio: `${ratio}`,
              }}
              className={cn(
                "group relative max-h-64 min-h-36 overflow-hidden rounded-tile bg-sunken text-start outline-none transition-transform duration-150 motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-focus",
                selected && "ring-2 ring-action",
              )}
            >
              <PhotoImage itemId={photo.id} size={480} alt={photo.title} className="size-full transition-transform duration-200 group-hover:scale-[1.02] motion-reduce:transition-none" />
              <PhotoTileOverlay photo={photo} selectMode={selectMode} selected={selected} favLabel={t("photosApp.lightbox.favorited")} />
            </button>
          );
        })}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-3 gap-1.5 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-7">
      {items.map((photo, index) => {
        const selected = Boolean(selectedIds?.has(photo.id));
        return (
          <button
            key={photo.id}
            type="button"
            aria-label={photo.title}
            aria-pressed={selectMode ? selected : undefined}
            onClick={() => {
              if (selectMode && onToggleSelect) onToggleSelect(photo.id);
              else onOpenPhoto(index);
            }}
            className={cn(
              "group relative aspect-square overflow-hidden rounded-tile bg-sunken text-start outline-none transition-transform duration-150 motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-focus",
              selected && "ring-2 ring-action",
            )}
          >
            <PhotoImage itemId={photo.id} size={240} alt={photo.title} className="size-full transition-transform duration-200 group-hover:scale-[1.03] motion-reduce:transition-none" />
            <PhotoTileOverlay photo={photo} selectMode={selectMode} selected={selected} favLabel={t("photosApp.lightbox.favorited")} />
          </button>
        );
      })}
    </div>
  );
}

function PhotoTileOverlay({
  photo,
  selectMode,
  selected,
  favLabel,
}: {
  photo: PhotoTimelineItem;
  selectMode: boolean;
  selected: boolean;
  favLabel: string;
}) {
  return (
    <>
      <span className="pointer-events-none absolute inset-x-0 bottom-0 flex items-end justify-between bg-gradient-to-t from-black/75 via-black/25 to-transparent px-2 py-1.5 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 motion-reduce:transition-none">
        <span className="truncate text-xs font-mid text-white">{photo.title}</span>
      </span>
      {photo.favorite ? (
        <span
          aria-label={favLabel}
          className="pointer-events-none absolute end-1.5 bottom-1.5 grid size-6 place-items-center rounded-full bg-black/65 text-white"
        >
          <Heart className="size-3.5 fill-current" strokeWidth={1.8} aria-hidden />
        </span>
      ) : null}
      {selectMode ? (
        <span
          aria-hidden
          className={cn(
            "pointer-events-none absolute start-2 top-2 grid size-6 place-items-center rounded-full border text-white shadow-sm",
            selected ? "border-white bg-neutral-900" : "border-white/85 bg-black/50",
          )}
        >
          {selected ? <Check className="size-3.5" strokeWidth={2.4} /> : null}
        </span>
      ) : null}
    </>
  );
}
