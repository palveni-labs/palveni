import { useQuery } from "@tanstack/react-query";
import type { LucideIcon } from "lucide-react";
import { Film } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { videoApi } from "../api";
import { videoKeys } from "../queries";

const HAS_IO = typeof IntersectionObserver !== "undefined";

const VIDEO_FALLBACKS = [
  "linear-gradient(145deg, oklch(0.32 0.09 246), oklch(0.20 0.06 218))",
  "linear-gradient(145deg, oklch(0.32 0.08 198), oklch(0.20 0.05 168))",
  "linear-gradient(145deg, oklch(0.34 0.09 42), oklch(0.21 0.06 24))",
  "linear-gradient(145deg, oklch(0.32 0.08 156), oklch(0.20 0.05 190))",
] as const;

function fallbackGradient(seed: string): string {
  let h = 0;
  for (let i = 0; i < seed.length; i += 1) h = (h * 33 + seed.charCodeAt(i)) >>> 0;
  return VIDEO_FALLBACKS[h % VIDEO_FALLBACKS.length] ?? VIDEO_FALLBACKS[0];
}

/**
 * Loads an authenticated video artwork image (poster, backdrop, thumb or person portrait)
 * via Bearer fetch + object URL with viewport lazy-gating, proper cleanup and graceful fallback.
 */
export function VideoImage({
  path,
  alt = "",
  decorative = true,
  fallbackIcon: FallbackIcon = Film,
  fallbackText,
  eager = false,
  className,
  imgClassName,
}: {
  path: string | null | undefined;
  alt?: string;
  decorative?: boolean;
  fallbackIcon?: LucideIcon;
  fallbackText?: string;
  eager?: boolean;
  className?: string;
  imgClassName?: string;
}) {
  const hostRef = useRef<HTMLSpanElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);
  const [visible, setVisible] = useState(!HAS_IO || eager);

  useEffect(() => {
    if (visible || !HAS_IO || !path) return;
    const node = hostRef.current;
    if (!node) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [visible, path]);

  const query = useQuery({
    queryKey: videoKeys.image(path ?? ""),
    queryFn: ({ signal }) => videoApi.image(path!, signal),
    enabled: Boolean(path) && (visible || eager),
    staleTime: 5 * 60_000,
    gcTime: 5 * 60_000,
    retry: false,
  });

  useEffect(() => {
    const el = imgRef.current;
    const blob = query.data;
    if (!el || !blob) return;
    const created = URL.createObjectURL(blob);
    el.hidden = false;
    el.src = created;
    return () => {
      el.removeAttribute("src");
      URL.revokeObjectURL(created);
    };
  }, [query.data]);

  return (
    <span
      ref={hostRef}
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : alt}
      aria-hidden={decorative || undefined}
      style={!query.data ? { backgroundImage: fallbackGradient(path ?? alt ?? fallbackText ?? "video") } : undefined}
      className={cn("relative grid place-items-center overflow-hidden bg-sunken text-white/75", className)}
    >
      {!query.data ? (
        <>
          <span
            aria-hidden
            className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_24%_18%,rgba(255,255,255,0.16),transparent_62%)]"
          />
          {fallbackText ? (
            <span className="relative font-heading text-lg font-title tracking-tight text-white/90 select-none">
              {fallbackText}
            </span>
          ) : (
            <FallbackIcon className="relative size-8 opacity-75" strokeWidth={1.5} aria-hidden />
          )}
        </>
      ) : null}
      {query.data ? (
        <img
          ref={imgRef}
          alt={decorative ? "" : alt}
          draggable={false}
          decoding="async"
          onError={(event) => {
            event.currentTarget.hidden = true;
          }}
          className={cn("absolute inset-0 block size-full object-cover", imgClassName)}
        />
      ) : null}
    </span>
  );
}
