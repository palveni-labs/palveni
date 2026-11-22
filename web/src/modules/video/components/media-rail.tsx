import { Link, type LinkProps } from "@tanstack/react-router";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";

/**
 * Horizontal scroll shelf with smooth prev/next arrow buttons on desktop and
 * touch-friendly momentum snapping on phones.
 */
export function MediaRail({
  title,
  count,
  moreTo,
  moreParams,
  moreSearch,
  children,
  className,
}: {
  title: string;
  count?: number;
  moreTo?: LinkProps["to"];
  moreParams?: LinkProps["params"];
  moreSearch?: LinkProps["search"];
  children: ReactNode;
  className?: string;
}) {
  const { t } = useTranslation();
  const trackRef = useRef<HTMLDivElement>(null);
  const [canPrev, setCanPrev] = useState(false);
  const [canNext, setCanNext] = useState(false);

  const syncScroll = useCallback(() => {
    const el = trackRef.current;
    if (!el) return;
    setCanPrev(el.scrollLeft > 4);
    setCanNext(el.scrollLeft + el.clientWidth < el.scrollWidth - 4);
  }, []);

  useEffect(() => {
    syncScroll();
    const el = trackRef.current;
    if (!el) return;
    const observer = new ResizeObserver(syncScroll);
    observer.observe(el);
    return () => observer.disconnect();
  }, [syncScroll, children]);

  const scrollByPage = (direction: -1 | 1) => {
    const el = trackRef.current;
    if (!el) return;
    el.scrollBy({ left: direction * Math.max(280, Math.round(el.clientWidth * 0.82)), behavior: "smooth" });
  };

  return (
    <section aria-label={title} className={cn("grid min-w-0 gap-3", className)}>
      <header className="flex items-center gap-2">
        <h2 className="truncate font-heading text-lg font-title text-heading">{title}</h2>
        {count !== undefined && count > 0 ? (
          <span data-num className="text-sm text-fg-muted tabular-nums">
            {count}
          </span>
        ) : null}
        <span className="flex-1" />
        <div className="hidden items-center gap-1 md:flex">
          <button
            type="button"
            disabled={!canPrev}
            onClick={() => scrollByPage(-1)}
            aria-label={t("videoApp.home.scrollLeft")}
            className="grid size-7 place-items-center rounded-control border border-line bg-surface text-fg-muted transition-colors hover:bg-hover hover:text-fg disabled:cursor-default disabled:opacity-35"
          >
            <ChevronLeft className="size-4" aria-hidden />
          </button>
          <button
            type="button"
            disabled={!canNext}
            onClick={() => scrollByPage(1)}
            aria-label={t("videoApp.home.scrollRight")}
            className="grid size-7 place-items-center rounded-control border border-line bg-surface text-fg-muted transition-colors hover:bg-hover hover:text-fg disabled:cursor-default disabled:opacity-35"
          >
            <ChevronRight className="size-4" aria-hidden />
          </button>
        </div>
        {moreTo ? (
          <Link
            to={moreTo}
            params={moreParams}
            search={moreSearch}
            className="inline-flex shrink-0 items-center gap-0.5 rounded-control py-1 ps-2 pe-1 text-sm font-mid text-fg-muted transition-colors hover:bg-hover hover:text-fg"
          >
            {t("videoApp.home.viewAll")}
            <ChevronRight className="size-3.5" aria-hidden />
          </Link>
        ) : null}
      </header>
      <div
        ref={trackRef}
        onScroll={syncScroll}
        className="no-scrollbar -mx-1 flex snap-x snap-mandatory gap-3.5 overflow-x-auto px-1 pb-1 sm:gap-4"
      >
        {children}
      </div>
    </section>
  );
}
