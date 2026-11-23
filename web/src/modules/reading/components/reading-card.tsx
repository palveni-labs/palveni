import { Link } from "@tanstack/react-router";
import { BookOpen, Check, Heart, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { readingImagePaths, type ReadingEntry } from "../api";
import { useToggleReadingFavorite } from "../queries";
import { ReadingImage } from "./reading-image";

function readingProgressFraction(entry: ReadingEntry): number {
  if (entry.state.completed) return 1;
  const total = entry.state.totalPages ?? entry.pageCount ?? 0;
  if (total <= 0) return 0;
  if (entry.state.page <= 0 && !entry.state.locator) return 0;
  return Math.min(0.98, Math.max(0.03, (entry.state.page + 1) / total));
}

export function ReadingCard({ entry, className }: { entry: ReadingEntry; className?: string }) {
  const { t } = useTranslation();
  const toggleFav = useToggleReadingFavorite();

  const isComic = entry.kind === "comic" && entry.format !== "pdf";
  const coverPath = entry.hasCover ? readingImagePaths.cover(entry.itemId, 480) : null;
  const fraction = readingProgressFraction(entry);
  const authorLine =
    entry.authors.length > 0
      ? entry.authors.join(" / ")
      : entry.seriesName
        ? `${entry.seriesName}${entry.seriesIndex != null ? ` #${entry.seriesIndex}` : ""}`
        : entry.format.toUpperCase();

  return (
    <article className={cn("group relative flex flex-col gap-2", className)}>
      <div className="relative aspect-[2/3] w-full overflow-hidden rounded-tile bg-sunken shadow-art ring-1 ring-line/60 transition-transform duration-150 motion-safe:group-hover:-translate-y-0.5">
        <Link
          to={isComic ? "/reading/comic/$itemId" : "/reading/book/$itemId"}
          params={{ itemId: entry.itemId }}
          tabIndex={-1}
          aria-hidden="true"
          className="block size-full outline-none"
        >
          <ReadingImage
            path={coverPath}
            alt={entry.title}
            fallbackIcon={entry.kind === "comic" ? Layers : BookOpen}
            fallbackTitle={entry.title}
            fallbackSubtitle={entry.authors[0] ?? entry.format.toUpperCase()}
            className="size-full"
          />
        </Link>

        {/* Format chip */}
        <span className="pointer-events-none absolute start-2 top-2 rounded-chip bg-black/75 px-1.5 py-0.5 font-mono text-[11px] font-mid uppercase text-white backdrop-blur-xs">
          {entry.format}
        </span>

        {/* Completed badge */}
        {entry.state.completed ? (
          <span className="pointer-events-none absolute end-2 top-2 inline-flex items-center gap-0.5 rounded-chip bg-black/80 px-1.5 py-0.5 text-[11px] font-strong text-emerald-300 backdrop-blur-xs">
            <Check className="size-3" aria-hidden />
            {t("readingApp.shelf.completedBadge")}
          </span>
        ) : null}

        {/* Favorite button */}
        <button
          type="button"
          aria-label={
            entry.state.favorite
              ? `${entry.title} · ${t("readingApp.bookReader.favorited")}`
              : `${entry.title} · ${t("readingApp.bookReader.favorite")}`
          }
          aria-pressed={entry.state.favorite}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            toggleFav.mutate({ itemId: entry.itemId, favorite: !entry.state.favorite });
          }}
          className={cn(
            "absolute end-2 bottom-2.5 z-10 grid size-7 place-items-center rounded-full bg-black/72 text-white transition-opacity focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
            entry.state.favorite ? "opacity-100 text-amber-400" : "opacity-0 group-hover:opacity-100",
          )}
        >
          <Heart className={cn("size-3.5", entry.state.favorite && "fill-current")} aria-hidden />
        </button>

        {/* Bottom reading progress bar */}
        {fraction > 0 && !entry.state.completed ? (
          <div className="pointer-events-none absolute inset-x-0 bottom-0 h-1 bg-black/55" aria-hidden>
            <div
              className="h-full bg-amber-400 transition-[width]"
              style={{ width: `${Math.round(fraction * 100)}%` }}
            />
          </div>
        ) : null}
      </div>

      <div className="grid min-w-0 gap-0.5 px-0.5">
        <Link
          to={isComic ? "/reading/comic/$itemId" : "/reading/book/$itemId"}
          params={{ itemId: entry.itemId }}
          className="truncate text-sm font-strong text-fg outline-none group-hover:underline focus-visible:underline"
        >
          {entry.title}
        </Link>
        <p className="truncate text-xs text-fg-muted">{authorLine}</p>
      </div>
    </article>
  );
}
