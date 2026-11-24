import { useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
  ArrowLeft,
  Bookmark,
  BookOpen,
  Check,
  ChevronLeft,
  ChevronRight,
  Columns2,
  Heart,
  Layers,
  Maximize2,
  Minimize2,
  PencilLine,
  ScrollText,
  Share2,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApiError } from "@/api/errors";
import { useIsAdmin } from "@/api/queries/auth";
import { cn } from "@/lib/cn";
import { useFeature } from "@/modules/access";
import { AddToCollectionDialog } from "@/ui/add-to-collection-dialog";
import { MetadataEditModal } from "@/ui/metadata-edit-modal";
import { ToolbarTitle } from "@/ui/page";
import { ShareDialog } from "@/ui/share-dialog";
import { ErrorState, LoadingRows } from "@/ui/states";
import { readingImagePaths } from "../api";
import { ReadingImage } from "../components/reading-image";
import {
  readingKeys,
  useCreateReadingBookmark,
  useDeleteReadingBookmark,
  usePrefetchComicPages,
  useReadingInlinePdf,
  useReadingManifest,
  useSaveReadingProgress,
  useToggleReadingFavorite,
} from "../queries";

export type ComicViewMode = "single" | "spread" | "webtoon";
export type ComicDirection = "ltr" | "rtl";
export type ComicFit = "width" | "height" | "original";

interface ComicPrefs {
  mode: ComicViewMode;
  direction: ComicDirection;
  fit: ComicFit;
}

const COMIC_PREFS_KEY = "palveni.reading.comicPrefs";

function loadComicPrefs(): ComicPrefs {
  try {
    const raw = localStorage.getItem(COMIC_PREFS_KEY);
    if (!raw) return { mode: "single", direction: "ltr", fit: "height" };
    const parsed = JSON.parse(raw) as Partial<ComicPrefs>;
    return {
      mode: parsed.mode === "spread" || parsed.mode === "webtoon" ? parsed.mode : "single",
      direction: parsed.direction === "rtl" ? "rtl" : "ltr",
      fit: parsed.fit === "width" || parsed.fit === "original" ? parsed.fit : "height",
    };
  } catch {
    return { mode: "single", direction: "ltr", fit: "height" };
  }
}

function saveComicPrefs(prefs: ComicPrefs) {
  try {
    localStorage.setItem(COMIC_PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // Ignore storage errors
  }
}

export function ComicReaderPage({ itemId }: { itemId: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const isAdmin = useIsAdmin();
  const stageRef = useRef<HTMLDivElement>(null);
  const manifestQuery = useReadingManifest(itemId);
  const manifest = manifestQuery.data;

  const [metadataOpen, setMetadataOpen] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const [collectionOpen, setCollectionOpen] = useState(false);
  const sharesEnabled = useFeature("shares");
  const collectionsEnabled = useFeature("collections");
  const [prefs, setPrefs] = useState<ComicPrefs>(loadComicPrefs);
  const [pageOverride, setPageOverride] = useState<number | null>(null);
  const [showBookmarks, setShowBookmarks] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [unsupportedError, setUnsupportedError] = useState<unknown>(null);

  const saveProgress = useSaveReadingProgress(itemId);
  const toggleFav = useToggleReadingFavorite();
  const createBookmark = useCreateReadingBookmark(itemId);
  const deleteBookmark = useDeleteReadingBookmark(itemId);

  const totalPages = Math.max(1, manifest?.entry.pageCount ?? manifest?.entry.state.totalPages ?? 1);
  const maxPage = Math.max(0, totalPages - 1);
  const currentPage = Math.max(
    0,
    Math.min(maxPage, pageOverride ?? manifest?.entry.state.page ?? 0),
  );

  const prefetchPage = usePrefetchComicPages(itemId, totalPages);
  const isPdf = manifest?.readerMode === "pdf";
  const isDownloadOnly = manifest?.readerMode === "download_only";
  const pdfInlineQuery = useReadingInlinePdf(itemId, Boolean(manifest) && isPdf);

  const updatePrefs = (patch: Partial<ComicPrefs>) => {
    setPrefs((prev) => {
      const next = { ...prev, ...patch };
      saveComicPrefs(next);
      return next;
    });
  };

  // Preload next 2 pages automatically
  useEffect(() => {
    if (!manifest || isPdf || isDownloadOnly || prefs.mode === "webtoon") return;
    prefetchPage(currentPage + 1);
    prefetchPage(currentPage + 2);
  }, [manifest, isPdf, isDownloadOnly, prefs.mode, currentPage, prefetchPage]);

  // Sync fullscreen state
  useEffect(() => {
    const onFsChange = () => setIsFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener("fullscreenchange", onFsChange);
    return () => document.removeEventListener("fullscreenchange", onFsChange);
  }, []);

  const goToPage = (target: number) => {
    if (!manifest) return;
    const clamped = Math.max(0, Math.min(maxPage, target));
    setPageOverride(clamped);
    const reachedEnd = clamped >= maxPage && totalPages > 1;
    saveProgress.mutate({
      page: clamped,
      totalPages,
      locator: `page:${clamped}`,
      completed: manifest.entry.state.completed || reachedEnd,
    });
  };

  const stepSize = prefs.mode === "spread" ? 2 : 1;
  const goForward = () => goToPage(currentPage + stepSize);
  const goBackward = () => goToPage(currentPage - stepSize);

  const toggleFullscreen = () => {
    const el = stageRef.current;
    if (!el) return;
    if (!document.fullscreenElement && el.requestFullscreen) {
      void el.requestFullscreen().catch(() => {});
    } else if (document.exitFullscreen) {
      void document.exitFullscreen().catch(() => {});
    }
  };

  // Keyboard shortcuts (`←/→` respecting LTR/RTL, `Space`, `F`, `M`)
  useEffect(() => {
    if (!manifest || isPdf || isDownloadOnly) return;
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT" ||
          target.isContentEditable)
      ) {
        return;
      }
      if (e.key === "ArrowRight") {
        e.preventDefault();
        if (prefs.direction === "rtl") goBackward();
        else goForward();
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        if (prefs.direction === "rtl") goForward();
        else goBackward();
      } else if (e.key === " ") {
        if (prefs.mode !== "webtoon") {
          e.preventDefault();
          goForward();
        }
      } else if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        toggleFullscreen();
      } else if (e.key === "m" || e.key === "M") {
        e.preventDefault();
        const order: ComicViewMode[] = ["single", "spread", "webtoon"];
        const nextMode = order[(order.indexOf(prefs.mode) + 1) % order.length] ?? "single";
        updatePrefs({ mode: nextMode });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  if (manifestQuery.isLoading) {
    return <LoadingRows rows={6} />;
  }
  if (manifestQuery.error || !manifest) {
    return <ErrorState error={manifestQuery.error} onRetry={() => void manifestQuery.refetch()} />;
  }

  const { entry, bookmarks } = manifest;
  const unsupported =
    isDownloadOnly || (unsupportedError instanceof ApiError && unsupportedError.status === 415);

  const fitImgClass =
    prefs.fit === "width"
      ? "w-full h-auto max-w-4xl object-contain"
      : prefs.fit === "original"
        ? "w-auto h-auto max-w-none object-contain"
        : "max-h-[76vh] w-auto object-contain";

  // Spread pages: pair `[currentPage, currentPage + 1]`, reversed visually in Manga RTL mode
  const spreadPair: number[] =
    currentPage + 1 < totalPages ? [currentPage, currentPage + 1] : [currentPage];
  const orderedSpreadPages = prefs.direction === "rtl" ? [...spreadPair].reverse() : spreadPair;

  return (
    <div
      ref={stageRef}
      data-testid="comic-reader-stage"
      data-direction={prefs.direction}
      data-view-mode={prefs.mode}
      className="flex min-h-[84vh] flex-col justify-between overflow-hidden rounded-panel bg-neutral-950 text-neutral-100 shadow-raised ring-1 ring-white/12"
    >
      <ToolbarTitle title={entry.title} />

      {/* Top Dark Control Bar */}
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-white/10 bg-neutral-900/90 px-3.5 py-2.5 backdrop-blur-xs">
        <div className="flex min-w-0 items-center gap-2.5">
          <Link
            to="/reading/comics"
            aria-label={t("readingApp.comicReader.backToShelf")}
            className="inline-flex size-8 shrink-0 items-center justify-center rounded-control text-neutral-300 transition-colors hover:bg-white/10 hover:text-white focus-visible:outline-2 focus-visible:outline-amber-400"
          >
            <ArrowLeft className="size-4" aria-hidden />
          </Link>
          <div className="grid min-w-0">
            <h1 className="truncate font-heading text-sm font-title text-white sm:text-base">
              {entry.title}
            </h1>
            <p className="truncate text-xs text-neutral-400 tabular-nums">
              {entry.seriesName ? `${entry.seriesName} · ` : ""}
              {!unsupported && !isPdf
                ? t("readingApp.comicReader.pageIndicator", {
                    current: currentPage + 1,
                    total: totalPages,
                  })
                : entry.format.toUpperCase()}
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {!unsupported && !isPdf ? (
            <>
              {/* View mode selector: Single / Spread / Webtoon */}
              <div className="inline-flex rounded-control bg-white/8 p-0.5">
                {(
                  [
                    ["single", t("readingApp.comicReader.modeSingle"), Square],
                    ["spread", t("readingApp.comicReader.modeSpread"), Columns2],
                    ["webtoon", t("readingApp.comicReader.modeWebtoon"), ScrollText],
                  ] as const
                ).map(([modeKey, label, Icon]) => (
                  <button
                    key={modeKey}
                    type="button"
                    aria-pressed={prefs.mode === modeKey}
                    onClick={() => updatePrefs({ mode: modeKey })}
                    className={cn(
                      "inline-flex h-7 items-center gap-1 rounded px-2 text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-amber-400",
                      prefs.mode === modeKey
                        ? "bg-white text-neutral-950 font-strong"
                        : "text-neutral-300 hover:text-white",
                    )}
                  >
                    <Icon className="size-3.5" aria-hidden />
                    <span className="max-md:sr-only">{label}</span>
                  </button>
                ))}
              </div>

              {/* LTR vs Manga RTL toggle */}
              {prefs.mode !== "webtoon" ? (
                <button
                  type="button"
                  aria-pressed={prefs.direction === "rtl"}
                  onClick={() =>
                    updatePrefs({ direction: prefs.direction === "rtl" ? "ltr" : "rtl" })
                  }
                  className={cn(
                    "h-8 rounded-control px-2.5 text-xs font-strong ring-1 ring-white/15 transition-colors focus-visible:outline-2 focus-visible:outline-amber-400",
                    prefs.direction === "rtl"
                      ? "bg-amber-400 text-neutral-950"
                      : "bg-white/8 text-neutral-200 hover:bg-white/14",
                  )}
                >
                  {prefs.direction === "rtl"
                    ? t("readingApp.comicReader.directionRtl")
                    : t("readingApp.comicReader.directionLtr")}
                </button>
              ) : null}

              {/* Zoom / Fit */}
              <select
                value={prefs.fit}
                onChange={(e) => updatePrefs({ fit: e.target.value as ComicFit })}
                aria-label={t("readingApp.comicReader.fitHeight")}
                className="h-8 rounded-control border border-white/15 bg-neutral-900 px-2 text-xs text-neutral-100 focus-visible:outline-2 focus-visible:outline-amber-400"
              >
                <option value="height">{t("readingApp.comicReader.fitHeight")}</option>
                <option value="width">{t("readingApp.comicReader.fitWidth")}</option>
                <option value="original">{t("readingApp.comicReader.fitOriginal")}</option>
              </select>
            </>
          ) : null}

          {/* Bookmark current page */}
          <button
            type="button"
            aria-label={t("readingApp.comicReader.addBookmark")}
            onClick={() =>
              createBookmark.mutate({
                page: currentPage,
                locator: `page:${currentPage}`,
                label: `P.${currentPage + 1}`,
                note: null,
              })
            }
            className="inline-flex h-8 items-center gap-1 rounded-control bg-white/8 px-2.5 text-xs text-neutral-200 transition-colors hover:bg-white/14 focus-visible:outline-2 focus-visible:outline-amber-400"
          >
            <Bookmark className="size-3.5" aria-hidden />
            <span className="max-lg:sr-only">{t("readingApp.comicReader.addBookmark")}</span>
          </button>

          {bookmarks.length > 0 ? (
            <button
              type="button"
              aria-expanded={showBookmarks}
              onClick={() => setShowBookmarks((s) => !s)}
              className="h-8 rounded-control bg-white/8 px-2 text-xs font-strong text-amber-300 tabular-nums hover:bg-white/14"
            >
              {t("readingApp.comicReader.bookmarks")} ({bookmarks.length})
            </button>
          ) : null}

          {/* Favorite */}
          <button
            type="button"
            aria-pressed={entry.state.favorite}
            aria-label={
              entry.state.favorite
                ? t("readingApp.bookReader.favorited")
                : t("readingApp.bookReader.favorite")
            }
            onClick={() => toggleFav.mutate({ itemId: entry.itemId, favorite: !entry.state.favorite })}
            className={cn(
              "grid size-8 place-items-center rounded-control transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-amber-400",
              entry.state.favorite ? "text-amber-400" : "text-neutral-300",
            )}
          >
            <Heart className={cn("size-4", entry.state.favorite && "fill-current")} aria-hidden />
          </button>

          {collectionsEnabled ? (
            <button
              type="button"
              aria-label={t("collectionsApp.addToCollection")}
              onClick={() => setCollectionOpen(true)}
              className="grid size-8 place-items-center rounded-control text-neutral-300 transition-colors hover:bg-white/12 hover:text-white focus-visible:outline-2 focus-visible:outline-amber-400"
            >
              <Layers className="size-4" aria-hidden />
            </button>
          ) : null}

          {sharesEnabled ? (
            <button
              type="button"
              aria-label={t("sharesApp.button")}
              onClick={() => setShareOpen(true)}
              className="grid size-8 place-items-center rounded-control text-neutral-300 transition-colors hover:bg-white/12 hover:text-white focus-visible:outline-2 focus-visible:outline-amber-400"
            >
              <Share2 className="size-4" aria-hidden />
            </button>
          ) : null}

          {isAdmin ? (
            <button
              type="button"
              onClick={() => setMetadataOpen(true)}
              className="inline-flex h-8 items-center gap-1 rounded-control bg-white/8 px-2.5 text-xs text-neutral-200 transition-colors hover:bg-white/14 focus-visible:outline-2 focus-visible:outline-amber-400"
            >
              <PencilLine className="size-3.5" aria-hidden />
              <span className="max-lg:sr-only">{t("metadataModal.trigger")}</span>
            </button>
          ) : null}

          {/* Completed */}
          <button
            type="button"
            aria-pressed={entry.state.completed}
            aria-label={
              entry.state.completed
                ? t("readingApp.comicReader.completed")
                : t("readingApp.comicReader.markCompleted")
            }
            onClick={() =>
              saveProgress.mutate({
                page: currentPage,
                totalPages,
                locator: `page:${currentPage}`,
                completed: !entry.state.completed,
              })
            }
            className={cn(
              "grid size-8 place-items-center rounded-control transition-colors hover:bg-white/12 focus-visible:outline-2 focus-visible:outline-amber-400",
              entry.state.completed ? "text-emerald-400" : "text-neutral-300",
            )}
          >
            <Check className="size-4" aria-hidden />
          </button>

          {/* Fullscreen */}
          <button
            type="button"
            aria-label={
              isFullscreen
                ? t("readingApp.comicReader.exitFullscreen")
                : t("readingApp.comicReader.fullscreen")
            }
            onClick={toggleFullscreen}
            className="grid size-8 place-items-center rounded-control text-neutral-300 transition-colors hover:bg-white/12 hover:text-white focus-visible:outline-2 focus-visible:outline-amber-400"
          >
            {isFullscreen ? (
              <Minimize2 className="size-4" aria-hidden />
            ) : (
              <Maximize2 className="size-4" aria-hidden />
            )}
          </button>
        </div>
      </header>

      {/* Bookmarks Overlay List */}
      {showBookmarks && bookmarks.length > 0 ? (
        <div
          role="dialog"
          aria-label={t("readingApp.comicReader.bookmarks")}
          className="border-b border-white/10 bg-neutral-900 px-4 py-3"
        >
          <div className="mb-2 flex items-center justify-between">
            <span className="text-xs font-strong text-neutral-300">
              {t("readingApp.comicReader.bookmarks")}
            </span>
            <button
              type="button"
              onClick={() => setShowBookmarks(false)}
              className="text-neutral-400 hover:text-white"
            >
              <X className="size-4" aria-hidden />
            </button>
          </div>
          <div className="flex flex-wrap gap-2">
            {bookmarks.map((bm) => (
              <span
                key={bm.id}
                className="inline-flex items-center gap-1.5 rounded-chip bg-white/10 px-2.5 py-1 text-xs"
              >
                <button
                  type="button"
                  onClick={() => {
                    goToPage(bm.page);
                    setShowBookmarks(false);
                  }}
                  className="font-strong text-white hover:underline"
                >
                  {bm.label}
                </button>
                <button
                  type="button"
                  aria-label={`${t("readingApp.bookReader.deleteBookmark")} ${bm.label}`}
                  onClick={() => deleteBookmark.mutate(bm.id)}
                  className="text-neutral-400 hover:text-red-400"
                >
                  <Trash2 className="size-3" aria-hidden />
                </button>
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {/* Viewport */}
      <div className="relative flex flex-1 items-center justify-center overflow-auto p-3 sm:p-5">
        {isPdf ? (
          pdfInlineQuery.data ? (
            <iframe
              title={entry.title}
              src={pdfInlineQuery.data.url}
              className="h-[75vh] w-full rounded border border-white/10 bg-neutral-900"
            />
          ) : (
            <LoadingRows rows={4} />
          )
        ) : unsupported ? (
          <div className="mx-auto grid max-w-md justify-items-center gap-3 p-8 text-center">
            <BookOpen className="size-10 text-neutral-400" aria-hidden />
            <h2 className="font-heading text-lg font-title text-white">
              {t("readingApp.comicReader.unsupportedTitle")}
            </h2>
            <p className="text-sm text-neutral-400">
              {t("readingApp.comicReader.unsupportedDetail")}
            </p>
          </div>
        ) : prefs.mode === "webtoon" ? (
          <div
            data-testid="comic-webtoon-scroll"
            className="mx-auto flex max-h-[76vh] w-full max-w-3xl flex-col items-center gap-1 overflow-y-auto"
          >
            {Array.from({ length: totalPages }, (_, idx) => (
              <ReadingImage
                key={idx}
                path={readingImagePaths.page(itemId, idx)}
                alt={`${entry.title} P.${idx + 1}`}
                decorative={false}
                eager={idx < 3}
                onErrorStatus={setUnsupportedError}
                className="min-h-64 w-full bg-neutral-900"
                imgClassName="relative inset-auto w-full h-auto object-contain"
              />
            ))}
          </div>
        ) : prefs.mode === "spread" ? (
          <div
            data-testid="comic-spread-pair"
            className="flex items-center justify-center gap-1 sm:gap-2"
          >
            {orderedSpreadPages.map((pNum) => (
              <ReadingImage
                key={pNum}
                path={readingImagePaths.page(itemId, pNum)}
                alt={`${entry.title} P.${pNum + 1}`}
                decorative={false}
                eager
                onErrorStatus={setUnsupportedError}
                className="min-h-[55vh] min-w-[32vw] bg-neutral-950"
                imgClassName={cn("relative inset-auto", fitImgClass)}
              />
            ))}
          </div>
        ) : (
          <ReadingImage
            key={currentPage}
            path={readingImagePaths.page(itemId, currentPage)}
            alt={`${entry.title} P.${currentPage + 1}`}
            decorative={false}
            eager
            onErrorStatus={setUnsupportedError}
            className="min-h-[60vh] min-w-[40vw] bg-neutral-950"
            imgClassName={cn("relative inset-auto", fitImgClass)}
          />
        )}
      </div>

      {/* Bottom Scrubber Bar */}
      {!unsupported && !isPdf ? (
        <footer className="flex items-center gap-3 border-t border-white/10 bg-neutral-900/90 px-4 py-2.5 backdrop-blur-xs">
          <button
            type="button"
            aria-label={t("readingApp.comicReader.prevPage")}
            disabled={currentPage <= 0}
            onClick={goBackward}
            className="inline-flex size-8 shrink-0 items-center justify-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 disabled:opacity-35 focus-visible:outline-2 focus-visible:outline-amber-400"
          >
            <ChevronLeft className="size-4" aria-hidden />
          </button>

          <input
            type="range"
            min={1}
            max={totalPages}
            value={currentPage + 1}
            aria-label={t("readingApp.comicReader.pageSlider")}
            onChange={(e) => goToPage(Number(e.target.value) - 1)}
            style={{ direction: prefs.direction === "rtl" ? "rtl" : "ltr" }}
            className="h-1.5 flex-1 cursor-pointer accent-amber-400"
          />

          <span
            data-testid="comic-page-counter"
            className="min-w-16 text-center font-mono text-xs font-strong text-neutral-200 tabular-nums"
          >
            {t("readingApp.comicReader.pageIndicator", {
              current: currentPage + 1,
              total: totalPages,
            })}
          </span>

          <button
            type="button"
            aria-label={t("readingApp.comicReader.nextPage")}
            disabled={currentPage >= maxPage}
            onClick={goForward}
            className="inline-flex size-8 shrink-0 items-center justify-center rounded-control text-neutral-200 transition-colors hover:bg-white/12 disabled:opacity-35 focus-visible:outline-2 focus-visible:outline-amber-400"
          >
            <ChevronRight className="size-4" aria-hidden />
          </button>
        </footer>
      ) : null}
      {shareOpen ? (
        <ShareDialog
          isOpen
          onClose={() => setShareOpen(false)}
          targetKind="item"
          targetId={entry.itemId}
          defaultTitle={entry.title}
        />
      ) : null}
      <AddToCollectionDialog
        isOpen={collectionOpen}
        onClose={() => setCollectionOpen(false)}
        item={{
          entryKind: "comic",
          targetId: entry.itemId,
          title: entry.title,
          subtitle: entry.seriesName ?? entry.authors[0] ?? entry.format.toUpperCase(),
          artworkId: entry.hasCover ? `rd_${entry.itemId}` : null,
        }}
      />
      {isAdmin ? (
        <MetadataEditModal
          isOpen={metadataOpen}
          onClose={() => setMetadataOpen(false)}
          itemId={entry.itemId}
          defaultMediaType="book"
          initialValues={{
            title: entry.title,
            genres: entry.tags,
            overview: entry.summary,
          }}
          onSaved={() => {
            void queryClient.invalidateQueries({ queryKey: readingKeys.manifest(itemId) });
            void queryClient.invalidateQueries({ queryKey: readingKeys.all });
          }}
          onReset={() => {
            void queryClient.invalidateQueries({ queryKey: readingKeys.manifest(itemId) });
            void queryClient.invalidateQueries({ queryKey: readingKeys.all });
          }}
        />
      ) : null}
    </div>
  );
}
