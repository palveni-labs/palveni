import { Disc3, UserRound, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { useMusicArtworkUrl, type ArtworkSize } from "./artwork-cache";

const FALLBACK_GRADIENTS = [
  "linear-gradient(135deg, oklch(0.38 0.12 246), oklch(0.28 0.10 212))",
  "linear-gradient(135deg, oklch(0.40 0.11 196), oklch(0.28 0.09 164))",
  "linear-gradient(135deg, oklch(0.42 0.12 52), oklch(0.30 0.10 26))",
  "linear-gradient(135deg, oklch(0.38 0.11 154), oklch(0.26 0.08 184))",
  "linear-gradient(135deg, oklch(0.42 0.12 30), oklch(0.29 0.09 16))",
  "linear-gradient(135deg, oklch(0.36 0.09 224), oklch(0.25 0.07 248))",
] as const;

function gradientFor(seed: string): string {
  let hash = 0;
  for (let i = 0; i < seed.length; i += 1) hash = (hash * 31 + seed.charCodeAt(i)) >>> 0;
  return FALLBACK_GRADIENTS[hash % FALLBACK_GRADIENTS.length] ?? FALLBACK_GRADIENTS[0];
}

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
  const fallbackBg = gradientFor(itemId ?? alt ?? "music");

  return (
    <span
      ref={hostRef}
      style={{ backgroundImage: fallbackBg }}
      className={cn(
        "relative grid shrink-0 place-items-center overflow-hidden text-white/80 select-none",
        round ? "rounded-full" : "rounded-tile",
        className,
      )}
    >
      <span
        aria-hidden
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_24%_18%,rgba(255,255,255,0.22),transparent_60%)]"
      />
      <FallbackIcon className="relative size-[38%] max-h-11 max-w-11 drop-shadow-xs" strokeWidth={1.5} aria-hidden />
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
