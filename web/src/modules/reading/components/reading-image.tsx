import { useQuery } from "@tanstack/react-query";
import type { LucideIcon } from "lucide-react";
import { BookOpen } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";
import { readingApi } from "../api";
import { readingKeys } from "../queries";

const HAS_IO = typeof IntersectionObserver !== "undefined";

/**
 * Loads an authenticated reading cover or comic page image via Bearer fetch + Object URL
 * with viewport lazy-gating, memory cleanup, and typographic book cover fallback.
 */
export function ReadingImage({
  path,
  alt = "",
  decorative = true,
  fallbackIcon: FallbackIcon = BookOpen,
  fallbackTitle,
  fallbackSubtitle,
  eager = false,
  onErrorStatus,
  className,
  imgClassName,
}: {
  path: string | null | undefined;
  alt?: string;
  decorative?: boolean;
  fallbackIcon?: LucideIcon;
  fallbackTitle?: string;
  fallbackSubtitle?: string;
  eager?: boolean;
  onErrorStatus?: (error: unknown) => void;
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
      { rootMargin: "240px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [visible, path]);

  const query = useQuery({
    queryKey: readingKeys.image(path ?? ""),
    queryFn: ({ signal }) => readingApi.image(path!, signal),
    enabled: Boolean(path) && (visible || eager),
    staleTime: 5 * 60_000,
    gcTime: 5 * 60_000,
    retry: false,
  });

  useEffect(() => {
    if (query.error && onErrorStatus) {
      onErrorStatus(query.error);
    }
  }, [query.error, onErrorStatus]);

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
        fallbackTitle ? (
          <span className="flex size-full flex-col justify-between p-3.5 text-start select-none">
            <FallbackIcon className="size-4 text-fg-muted opacity-75" strokeWidth={1.6} aria-hidden />
            <span className="grid gap-1">
              <span className="line-clamp-3 font-heading text-sm font-title leading-snug text-fg">
                {fallbackTitle}
              </span>
              {fallbackSubtitle ? (
                <span className="truncate text-xs text-fg-muted">{fallbackSubtitle}</span>
              ) : null}
            </span>
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
