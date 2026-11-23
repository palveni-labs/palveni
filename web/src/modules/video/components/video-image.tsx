import { useQuery } from "@tanstack/react-query";
import type { LucideIcon } from "lucide-react";
import { Film } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { videoApi } from "../api";
import { videoKeys } from "../queries";

const HAS_IO = typeof IntersectionObserver !== "undefined";

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
      className={cn("relative grid place-items-center overflow-hidden bg-sunken text-fg-subtle", className)}
    >
      {!query.data ? (
        fallbackText ? (
          <span className="font-heading text-lg font-title tracking-tight text-fg-muted select-none">
            {fallbackText}
          </span>
        ) : (
          <FallbackIcon className="size-8 opacity-65" strokeWidth={1.5} aria-hidden />
        )
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
