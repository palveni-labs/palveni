import { Disc3, UserRound, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { useMusicArtworkUrl, type ArtworkSize } from "./artwork-cache";

export function MusicArtwork({
  itemId,
  size = 240,
  alt = "",
  round = false,
  eager = false,
  icon,
  className,
}: {
  itemId: string | null | undefined;
  size?: ArtworkSize;
  alt?: string;
  round?: boolean;
  eager?: boolean;
  icon?: LucideIcon;
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
      { rootMargin: "240px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [eager, visible, itemId]);

  const url = useMusicArtworkUrl(itemId, size, visible);
  const broken = Boolean(url && brokenUrl === url);
  const FallbackIcon = icon ?? (round ? UserRound : Disc3);

  return (
    <span
      ref={hostRef}
      className={cn(
        "relative grid shrink-0 place-items-center overflow-hidden bg-sunken text-fg-subtle select-none",
        round ? "rounded-full" : "rounded-tile",
        className,
      )}
    >
      <FallbackIcon className="size-[36%] max-h-10 max-w-10 opacity-65" strokeWidth={1.6} aria-hidden />
      {url && !broken ? (
        <img
          src={url}
          alt={alt}
          draggable={false}
          decoding="async"
          onError={() => setBrokenUrl(url)}
          className="absolute inset-0 size-full object-cover"
        />
      ) : null}
    </span>
  );
}

/** 2×2 cover mosaic for playlists (falls back to a single cover or calm placeholder). */
export function MusicMosaic({
  itemIds,
  size = 240,
  eager = false,
  className,
}: {
  itemIds: readonly (string | null | undefined)[];
  size?: ArtworkSize;
  eager?: boolean;
  className?: string;
}) {
  const unique: string[] = [];
  for (const id of itemIds) {
    if (id && !unique.includes(id)) unique.push(id);
    if (unique.length === 4) break;
  }
  if (unique.length === 0) {
    return <MusicArtwork itemId={null} size={size} eager={eager} className={className} />;
  }
  if (unique.length < 4) {
    return <MusicArtwork itemId={unique[0]} size={size} eager={eager} className={className} />;
  }
  return (
    <span className={cn("grid shrink-0 grid-cols-2 grid-rows-2 gap-px overflow-hidden rounded-tile bg-sunken", className)}>
      {unique.map((id) => (
        <MusicArtwork key={id} itemId={id} size={96} eager={eager} className="size-full rounded-none" />
      ))}
    </span>
  );
}
