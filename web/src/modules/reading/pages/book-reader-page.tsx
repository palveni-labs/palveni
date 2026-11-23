import { Button, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import {
  ArrowLeft,
  Bookmark,
  BookOpen,
  Check,
  ChevronLeft,
  ChevronRight,
  Download,
  Heart,
  List,
  Trash2,
  Type,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { ToolbarTitle } from "@/ui/page";
import { ErrorState, LoadingRows } from "@/ui/states";
import {
  downloadOriginalReadingFile,
  readingApi,
  readingImagePaths,
  type ReadingBookmark,
} from "../api";
import {
  useCreateReadingBookmark,
  useDeleteReadingBookmark,
  useReadingChapter,
  useReadingInlinePdf,
  useReadingManifest,
  useSaveReadingProgress,
  useToggleReadingFavorite,
} from "../queries";

type ReaderTheme = "skin" | "paper" | "sepia" | "night";
type ColumnWidth = "narrow" | "medium" | "wide";

interface TypographySettings {
  fontSize: number;
  lineHeight: number;
  width: ColumnWidth;
  theme: ReaderTheme;
}

const TYPOGRAPHY_KEY = "palveni.reading.typography";
const DEFAULT_TYPOGRAPHY: TypographySettings = {
  fontSize: 18,
  lineHeight: 1.8,
  width: "medium",
  theme: "skin",
};

function loadTypography(): TypographySettings {
  try {
    const raw = localStorage.getItem(TYPOGRAPHY_KEY);
    if (!raw) return DEFAULT_TYPOGRAPHY;
    const parsed = JSON.parse(raw) as Partial<TypographySettings>;
    return {
      fontSize: typeof parsed.fontSize === "number" ? Math.min(28, Math.max(14, parsed.fontSize)) : 18,
      lineHeight: typeof parsed.lineHeight === "number" ? Math.min(2.2, Math.max(1.4, parsed.lineHeight)) : 1.8,
      width:
        parsed.width === "narrow" || parsed.width === "medium" || parsed.width === "wide"
          ? parsed.width
          : "medium",
      theme:
        parsed.theme === "skin" ||
        parsed.theme === "paper" ||
        parsed.theme === "sepia" ||
        parsed.theme === "night"
          ? parsed.theme
          : "skin",
    };
  } catch {
    return DEFAULT_TYPOGRAPHY;
  }
}

function saveTypography(next: TypographySettings) {
  try {
    localStorage.setItem(TYPOGRAPHY_KEY, JSON.stringify(next));
  } catch {
    // Ignore storage quota errors
  }
}

const THEME_STYLES: Record<ReaderTheme, { surface: string; style?: React.CSSProperties }> = {
  skin: { surface: "bg-surface text-fg ring-1 ring-line" },
  paper: {
    surface: "ring-1 ring-stone-300",
    style: { backgroundColor: "#faf8f3", color: "#1c1917" },
  },
  sepia: {
    surface: "ring-1 ring-amber-900/20",
    style: { backgroundColor: "#f4ecd8", color: "#2b2114" },
  },
  night: {
    surface: "ring-1 ring-neutral-800",
    style: { backgroundColor: "#141619", color: "#e5e7eb" },
  },
};

const WIDTH_CLASS: Record<ColumnWidth, string> = {
  narrow: "max-w-xl",
  medium: "max-w-2xl",
  wide: "max-w-4xl",
};

/**
 * Renders sanitized chapter HTML and hydrates authenticated `data-reading-asset` images
 * via Bearer blob requests without external network calls.
 */
function SanitizedChapterBody({
  itemId,
  html,
  fontSize,
  lineHeight,
}: {
  itemId: string;
  html: string;
  fontSize: number;
  lineHeight: number;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const markup = useMemo(() => ({ __html: html }), [html]);

  useEffect(() => {
    const root = containerRef.current;
    if (!root) return;
    const controller = new AbortController();
    const createdUrls: string[] = [];
    const imgs = Array.from(root.querySelectorAll<HTMLImageElement>("img[data-reading-asset]"));

    for (const img of imgs) {
      const assetPath = img.getAttribute("data-reading-asset");
      if (!assetPath) continue;
      img.removeAttribute("src");
      void readingApi
        .image(readingImagePaths.asset(itemId, assetPath), controller.signal)
        .then((blob) => {
          if (controller.signal.aborted) return;
          const url = URL.createObjectURL(blob);
          createdUrls.push(url);
          img.src = url;
        })
        .catch(() => {
          img.hidden = true;
        });
    }

    return () => {
      controller.abort();
      for (const u of createdUrls) URL.revokeObjectURL(u);
    };
  }, [itemId, html]);

  return (
    <div
      ref={containerRef}
      style={{ fontSize: `${fontSize}px`, lineHeight }}
      className="reading-prose grid gap-4 break-words [&_blockquote]:border-s-2 [&_blockquote]:border-current/30 [&_blockquote]:ps-4 [&_blockquote]:italic [&_code]:rounded [&_code]:bg-black/8 [&_code]:px-1 [&_h1]:font-heading [&_h1]:text-2xl [&_h1]:font-title [&_h2]:font-heading [&_h2]:text-xl [&_h2]:font-title [&_h3]:font-heading [&_h3]:text-lg [&_h3]:font-strong [&_img]:mx-auto [&_img]:max-h-[70vh] [&_img]:rounded-tile [&_li]:ms-5 [&_li]:list-disc [&_pre]:overflow-x-auto [&_pre]:rounded-tile [&_pre]:bg-black/10 [&_pre]:p-3"
      dangerouslySetInnerHTML={markup}
    />
  );
}

export function BookReaderPage({ itemId }: { itemId: string }) {
  const { t } = useTranslation();
  const manifestQuery = useReadingManifest(itemId);
  const manifest = manifestQuery.data;

  const [chapterOverride, setChapterOverride] = useState<number | null>(null);
  const [drawer, setDrawer] = useState<"none" | "toc" | "bookmarks" | "typography">("none");
  const [typography, setTypography] = useState<TypographySettings>(loadTypography);
  const [bookmarkLabel, setBookmarkLabel] = useState("");
  const [bookmarkNote, setBookmarkNote] = useState("");
  const [pdfPageInput, setPdfPageInput] = useState<string>("");

  const saveProgress = useSaveReadingProgress(itemId);
  const toggleFav = useToggleReadingFavorite();
  const createBookmark = useCreateReadingBookmark(itemId);
  const deleteBookmark = useDeleteReadingBookmark(itemId);

  const readerMode = manifest?.readerMode ?? "chapters";
  const chapters = manifest?.chapters ?? [];
  const maxChapter = Math.max(0, chapters.length - 1);
  const activeChapter = Math.max(
    0,
    Math.min(maxChapter, chapterOverride ?? manifest?.entry.state.page ?? 0),
  );

  const chapterQuery = useReadingChapter(
    itemId,
    activeChapter,
    Boolean(manifest) && readerMode === "chapters" && chapters.length > 0,
  );
  const pdfInlineQuery = useReadingInlinePdf(itemId, Boolean(manifest) && readerMode === "pdf");

  const updateTypography = (patch: Partial<TypographySettings>) => {
    setTypography((prev) => {
      const next = { ...prev, ...patch };
      saveTypography(next);
      return next;
    });
  };

  const goToChapter = (nextIdx: number) => {
    if (!manifest || chapters.length === 0) return;
    const clamped = Math.max(0, Math.min(maxChapter, nextIdx));
    setChapterOverride(clamped);
    saveProgress.mutate({
      page: clamped,
      totalPages: chapters.length,
      locator: `chapter:${clamped}`,
      completed: manifest.entry.state.completed || clamped >= maxChapter && chapters.length === 1 ? manifest.entry.state.completed : false,
    });
    window.scrollTo?.({ top: 0, behavior: "instant" as ScrollBehavior });
  };

  // Auto-record initial visit when opening an untouched chaptered book
  const autoSavedRef = useRef<string | null>(null);
  useEffect(() => {
    if (!manifest || readerMode !== "chapters" || chapters.length === 0) return;
    if (autoSavedRef.current === itemId) return;
    autoSavedRef.current = itemId;
    if (!manifest.entry.state.lastReadAt && !manifest.entry.state.completed) {
      saveProgress.mutate({
        page: activeChapter,
        totalPages: chapters.length,
        locator: `chapter:${activeChapter}`,
        completed: false,
      });
    }
  }, [manifest, readerMode, chapters.length, itemId, activeChapter, saveProgress]);

  // Keyboard navigation (`←` / `→`)
  useEffect(() => {
    if (readerMode !== "chapters" || chapters.length <= 1) return;
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
      if (e.key === "ArrowLeft" && activeChapter > 0) {
        e.preventDefault();
        goToChapter(activeChapter - 1);
      } else if (e.key === "ArrowRight" && activeChapter < maxChapter) {
        e.preventDefault();
        goToChapter(activeChapter + 1);
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
  const currentChapterTitle =
    chapterQuery.data?.title ??
    chapters[activeChapter]?.title ??
    t("readingApp.bookReader.chapterPosition", {
      current: activeChapter + 1,
      total: Math.max(1, chapters.length),
    });

  const handleAddBookmark = (e: React.FormEvent) => {
    e.preventDefault();
    const pageNum =
      readerMode === "pdf"
        ? Math.max(0, (Number(pdfPageInput) || entry.state.page + 1) - 1)
        : activeChapter;
    const defaultLabel =
      readerMode === "pdf"
        ? `${entry.title} · P.${pageNum + 1}`
        : currentChapterTitle;
    createBookmark.mutate(
      {
        page: pageNum,
        locator: readerMode === "pdf" ? `pdf:${pageNum}` : `chapter:${pageNum}`,
        label: bookmarkLabel.trim() || defaultLabel,
        note: bookmarkNote.trim() || null,
      },
      {
        onSuccess: () => {
          setBookmarkLabel("");
          setBookmarkNote("");
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleJumpBookmark = (bm: ReadingBookmark) => {
    if (readerMode === "chapters") {
      goToChapter(bm.page);
    } else if (readerMode === "pdf") {
      setPdfPageInput(String(bm.page + 1));
      saveProgress.mutate({
        page: bm.page,
        totalPages: entry.state.totalPages ?? entry.pageCount,
        locator: bm.locator ?? `pdf:${bm.page}`,
        completed: entry.state.completed,
      });
    }
    setDrawer("none");
  };

  const handleToggleCompleted = () => {
    const nextCompleted = !entry.state.completed;
    saveProgress.mutate({
      page: readerMode === "chapters" ? activeChapter : entry.state.page,
      totalPages: readerMode === "chapters" ? chapters.length || null : entry.state.totalPages,
      locator:
        readerMode === "chapters"
          ? `chapter:${activeChapter}`
          : entry.state.locator ?? `page:${entry.state.page}`,
      completed: nextCompleted,
    });
  };

  const themeSpec = THEME_STYLES[typography.theme];

  return (
    <div className="grid gap-4 pb-10">
      <ToolbarTitle title={entry.title} />

      {/* Reader Top Bar */}
      <header className="sticky top-0 z-20 flex flex-wrap items-center justify-between gap-2 rounded-panel bg-surface/95 px-3.5 py-2.5 shadow-xs ring-1 ring-line backdrop-blur-xs">
        <div className="flex min-w-0 items-center gap-2.5">
          <Link
            to="/reading/books"
            aria-label={t("readingApp.bookReader.backToShelf")}
            className="inline-flex size-8 shrink-0 items-center justify-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <ArrowLeft className="size-4" aria-hidden />
          </Link>
          <div className="grid min-w-0">
            <h1 className="truncate font-heading text-base font-title text-heading">{entry.title}</h1>
            <p className="truncate text-xs text-fg-muted">
              {entry.authors.length > 0 ? `${entry.authors.join(" / ")} · ` : ""}
              {readerMode === "chapters" && chapters.length > 0
                ? t("readingApp.bookReader.chapterPosition", {
                    current: activeChapter + 1,
                    total: chapters.length,
                  })
                : entry.format.toUpperCase()}
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {readerMode === "chapters" && chapters.length > 0 ? (
            <button
              type="button"
              aria-expanded={drawer === "toc"}
              onClick={() => setDrawer((d) => (d === "toc" ? "none" : "toc"))}
              className="inline-flex h-8 items-center gap-1.5 rounded-control px-2.5 text-xs font-mid text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <List className="size-4" aria-hidden />
              <span>{t("readingApp.bookReader.toc")}</span>
            </button>
          ) : null}

          <button
            type="button"
            aria-expanded={drawer === "bookmarks"}
            onClick={() => setDrawer((d) => (d === "bookmarks" ? "none" : "bookmarks"))}
            className="inline-flex h-8 items-center gap-1.5 rounded-control px-2.5 text-xs font-mid text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <Bookmark className="size-4" aria-hidden />
            <span>{t("readingApp.bookReader.bookmarks")}</span>
            {bookmarks.length > 0 ? (
              <span className="rounded-chip bg-sunken px-1.5 py-0.2 text-[11px] font-strong tabular-nums">
                {bookmarks.length}
              </span>
            ) : null}
          </button>

          {readerMode === "chapters" ? (
            <button
              type="button"
              aria-expanded={drawer === "typography"}
              onClick={() => setDrawer((d) => (d === "typography" ? "none" : "typography"))}
              className="inline-flex h-8 items-center gap-1.5 rounded-control px-2.5 text-xs font-mid text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <Type className="size-4" aria-hidden />
              <span className="max-sm:sr-only">{t("readingApp.bookReader.typography")}</span>
            </button>
          ) : null}

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
              "inline-flex size-8 items-center justify-center rounded-control transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              entry.state.favorite ? "text-amber-500" : "text-fg-muted",
            )}
          >
            <Heart className={cn("size-4", entry.state.favorite && "fill-current")} aria-hidden />
          </button>

          <button
            type="button"
            aria-pressed={entry.state.completed}
            onClick={handleToggleCompleted}
            className={cn(
              "inline-flex h-8 items-center gap-1 rounded-control px-2.5 text-xs font-mid transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              entry.state.completed ? "text-emerald-600 font-strong" : "text-fg-muted",
            )}
          >
            <Check className="size-3.5" aria-hidden />
            <span className="max-md:sr-only">
              {entry.state.completed
                ? t("readingApp.bookReader.completed")
                : t("readingApp.bookReader.markCompleted")}
            </span>
          </button>

          {entry.kind === "book" ? (
            <button
              type="button"
              aria-label={t("readingApp.bookReader.downloadOriginal")}
              onClick={() => {
                void downloadOriginalReadingFile(entry.itemId).catch((err) =>
                  toast.danger(errorText(t, err)),
                );
              }}
              className="inline-flex size-8 items-center justify-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <Download className="size-4" aria-hidden />
            </button>
          ) : null}
        </div>
      </header>

      {/* Slide-out / Collapsible Drawers */}
      {drawer !== "none" ? (
        <section
          role="dialog"
          aria-label={
            drawer === "toc"
              ? t("readingApp.bookReader.toc")
              : drawer === "bookmarks"
                ? t("readingApp.bookReader.bookmarks")
                : t("readingApp.bookReader.typography")
          }
          className="rounded-panel bg-surface p-4 shadow-raised ring-1 ring-line"
        >
          <div className="mb-3 flex items-center justify-between border-b border-line pb-2.5">
            <h2 className="font-heading text-base font-title text-heading">
              {drawer === "toc"
                ? t("readingApp.bookReader.toc")
                : drawer === "bookmarks"
                  ? t("readingApp.bookReader.bookmarks")
                  : t("readingApp.bookReader.typography")}
            </h2>
            <button
              type="button"
              aria-label={t("readingApp.bookReader.closeDrawer")}
              onClick={() => setDrawer("none")}
              className="grid size-7 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg"
            >
              <X className="size-4" aria-hidden />
            </button>
          </div>

          {drawer === "toc" ? (
            <nav aria-label={t("readingApp.bookReader.toc")} className="max-h-80 overflow-y-auto">
              <ul className="grid gap-1">
                {chapters.map((ch) => {
                  const current = ch.index === activeChapter;
                  return (
                    <li key={ch.index}>
                      <button
                        type="button"
                        aria-current={current ? "step" : undefined}
                        onClick={() => {
                          goToChapter(ch.index);
                          setDrawer("none");
                        }}
                        className={cn(
                          "flex w-full items-center justify-between rounded-control px-3 py-2 text-start text-sm transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                          current
                            ? "bg-action text-action-fg font-strong"
                            : "text-fg hover:bg-hover",
                        )}
                      >
                        <span className="truncate">{ch.title}</span>
                        <span className="ms-2 shrink-0 text-xs opacity-75 tabular-nums">
                          #{ch.index + 1}
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            </nav>
          ) : null}

          {drawer === "bookmarks" ? (
            <div className="grid gap-4">
              <form onSubmit={handleAddBookmark} className="grid gap-2.5 sm:grid-cols-[1fr_1fr_auto]">
                <input
                  type="text"
                  value={bookmarkLabel}
                  onChange={(e) => setBookmarkLabel(e.target.value)}
                  placeholder={t("readingApp.bookReader.bookmarkLabelPlaceholder")}
                  aria-label={t("readingApp.bookReader.bookmarkLabelPlaceholder")}
                  className="h-9 rounded-control border border-line bg-sunken px-3 text-sm text-fg placeholder:text-fg-muted focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                />
                <input
                  type="text"
                  value={bookmarkNote}
                  onChange={(e) => setBookmarkNote(e.target.value)}
                  placeholder={t("readingApp.bookReader.bookmarkNotePlaceholder")}
                  aria-label={t("readingApp.bookReader.bookmarkNotePlaceholder")}
                  className="h-9 rounded-control border border-line bg-sunken px-3 text-sm text-fg placeholder:text-fg-muted focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
                />
                <Button type="submit" size="sm" isPending={createBookmark.isPending}>
                  {t("readingApp.bookReader.saveBookmark")}
                </Button>
              </form>

              {bookmarks.length === 0 ? (
                <p className="py-2 text-sm text-fg-muted">{t("readingApp.bookReader.noBookmarks")}</p>
              ) : (
                <ul className="grid max-h-72 gap-2 overflow-y-auto">
                  {bookmarks.map((bm) => (
                    <li
                      key={bm.id}
                      className="flex items-center justify-between gap-3 rounded-control bg-sunken px-3 py-2"
                    >
                      <button
                        type="button"
                        onClick={() => handleJumpBookmark(bm)}
                        className="grid min-w-0 flex-1 text-start outline-none focus-visible:underline"
                      >
                        <span className="truncate text-sm font-strong text-fg">{bm.label}</span>
                        {bm.note ? (
                          <span className="truncate text-xs text-fg-muted">{bm.note}</span>
                        ) : null}
                      </button>
                      <button
                        type="button"
                        aria-label={`${t("readingApp.bookReader.deleteBookmark")} ${bm.label}`}
                        onClick={() => deleteBookmark.mutate(bm.id)}
                        className="grid size-7 shrink-0 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-failed"
                      >
                        <Trash2 className="size-3.5" aria-hidden />
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ) : null}

          {drawer === "typography" ? (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              {/* Font size */}
              <div className="grid gap-1.5">
                <span className="text-xs font-mid text-fg-muted">{t("readingApp.bookReader.fontSize")}</span>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    aria-label="A-"
                    onPress={() => updateTypography({ fontSize: Math.max(14, typography.fontSize - 2) })}
                  >
                    A-
                  </Button>
                  <span className="min-w-12 text-center text-sm font-strong tabular-nums">
                    {typography.fontSize}px
                  </span>
                  <Button
                    size="sm"
                    variant="outline"
                    aria-label="A+"
                    onPress={() => updateTypography({ fontSize: Math.min(28, typography.fontSize + 2) })}
                  >
                    A+
                  </Button>
                </div>
              </div>

              {/* Line height */}
              <div className="grid gap-1.5">
                <span className="text-xs font-mid text-fg-muted">{t("readingApp.bookReader.lineHeight")}</span>
                <div className="flex items-center gap-1.5">
                  {[1.5, 1.8, 2.1].map((lh) => (
                    <button
                      key={lh}
                      type="button"
                      aria-pressed={typography.lineHeight === lh}
                      onClick={() => updateTypography({ lineHeight: lh })}
                      className={cn(
                        "h-8 rounded-control px-2.5 text-xs font-mid tabular-nums ring-1 ring-line",
                        typography.lineHeight === lh
                          ? "bg-action text-action-fg font-strong"
                          : "bg-sunken text-fg",
                      )}
                    >
                      {lh.toFixed(1)}
                    </button>
                  ))}
                </div>
              </div>

              {/* Column width */}
              <div className="grid gap-1.5">
                <span className="text-xs font-mid text-fg-muted">{t("readingApp.bookReader.columnWidth")}</span>
                <div className="flex items-center gap-1.5">
                  {(
                    [
                      ["narrow", t("readingApp.bookReader.widthNarrow")],
                      ["medium", t("readingApp.bookReader.widthMedium")],
                      ["wide", t("readingApp.bookReader.widthWide")],
                    ] as const
                  ).map(([w, label]) => (
                    <button
                      key={w}
                      type="button"
                      aria-pressed={typography.width === w}
                      onClick={() => updateTypography({ width: w })}
                      className={cn(
                        "h-8 rounded-control px-2.5 text-xs font-mid ring-1 ring-line",
                        typography.width === w
                          ? "bg-action text-action-fg font-strong"
                          : "bg-sunken text-fg",
                      )}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>

              {/* Theme */}
              <div className="grid gap-1.5">
                <span className="text-xs font-mid text-fg-muted">{t("readingApp.bookReader.theme")}</span>
                <div className="flex flex-wrap items-center gap-1.5">
                  {(
                    [
                      ["skin", t("readingApp.bookReader.themeSkin")],
                      ["paper", t("readingApp.bookReader.themePaper")],
                      ["sepia", t("readingApp.bookReader.themeSepia")],
                      ["night", t("readingApp.bookReader.themeNight")],
                    ] as const
                  ).map(([th, label]) => (
                    <button
                      key={th}
                      type="button"
                      aria-pressed={typography.theme === th}
                      onClick={() => updateTypography({ theme: th })}
                      className={cn(
                        "h-8 rounded-control px-2.5 text-xs font-mid ring-1 ring-line",
                        typography.theme === th
                          ? "bg-action text-action-fg font-strong"
                          : "bg-sunken text-fg",
                      )}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          ) : null}
        </section>
      ) : null}

      {/* Main Reader Surface */}
      {readerMode === "pdf" ? (
        <div className="grid gap-3">
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-panel bg-surface px-4 py-2.5 ring-1 ring-line">
            <div className="flex items-center gap-2 text-sm">
              <label htmlFor="pdf-page-input" className="text-fg-muted">
                {t("readingApp.bookReader.pdfPageLabel")}
              </label>
              <input
                id="pdf-page-input"
                type="number"
                min={1}
                value={pdfPageInput || String(entry.state.page + 1)}
                onChange={(e) => setPdfPageInput(e.target.value)}
                onBlur={() => {
                  const num = Math.max(1, Number(pdfPageInput) || entry.state.page + 1);
                  saveProgress.mutate({
                    page: num - 1,
                    totalPages: entry.state.totalPages ?? entry.pageCount,
                    locator: `pdf:${num - 1}`,
                    completed: entry.state.completed,
                  });
                }}
                className="h-8 w-20 rounded-control border border-line bg-sunken px-2 text-center text-sm font-strong tabular-nums text-fg"
              />
            </div>
            {entry.summary ? (
              <p className="truncate text-xs text-fg-muted">{entry.summary}</p>
            ) : null}
          </div>

          {pdfInlineQuery.isLoading ? (
            <LoadingRows rows={5} />
          ) : pdfInlineQuery.error || !pdfInlineQuery.data ? (
            <ErrorState
              error={pdfInlineQuery.error}
              onRetry={() => void pdfInlineQuery.refetch()}
            />
          ) : (
            <iframe
              title={`${entry.title} · ${t("readingApp.bookReader.pdfInlineTitle")}`}
              src={pdfInlineQuery.data.url}
              className="h-[78vh] w-full rounded-panel border border-line bg-neutral-900"
            />
          )}
        </div>
      ) : readerMode === "download_only" ? (
        <div className="mx-auto flex max-w-xl flex-col items-center gap-4 rounded-panel bg-surface p-8 text-center ring-1 ring-line">
          <BookOpen className="size-10 text-fg-muted" strokeWidth={1.5} aria-hidden />
          <div className="grid gap-1.5">
            <h2 className="font-heading text-lg font-title text-heading">
              {t("readingApp.bookReader.downloadOnlyTitle")}
            </h2>
            <p className="text-sm text-fg-muted">{t("readingApp.bookReader.downloadOnlyDetail")}</p>
            {entry.summary ? <p className="mt-2 text-sm text-fg">{entry.summary}</p> : null}
          </div>
          <Button
            onPress={() => {
              void downloadOriginalReadingFile(entry.itemId).catch((err) =>
                toast.danger(errorText(t, err)),
              );
            }}
          >
            <Download aria-hidden />
            {t("readingApp.bookReader.downloadOriginal")}
          </Button>
        </div>
      ) : (
        <div
          style={themeSpec.style}
          className={cn(
            "mx-auto w-full rounded-panel px-5 py-8 shadow-xs transition-colors sm:px-10 sm:py-12",
            WIDTH_CLASS[typography.width],
            themeSpec.surface,
          )}
        >
          {chapterQuery.isLoading ? (
            <LoadingRows rows={8} />
          ) : chapterQuery.error ? (
            <ErrorState error={chapterQuery.error} onRetry={() => void chapterQuery.refetch()} />
          ) : chapterQuery.data ? (
            <article aria-label={chapterQuery.data.title} className="grid gap-6">
              <header className="border-b border-current/12 pb-4">
                <p className="text-xs opacity-70 tabular-nums">
                  {t("readingApp.bookReader.chapterPosition", {
                    current: activeChapter + 1,
                    total: Math.max(1, chapters.length),
                  })}
                </p>
                <h2 className="mt-1 font-heading text-2xl font-title">{chapterQuery.data.title}</h2>
              </header>

              <SanitizedChapterBody
                itemId={itemId}
                html={chapterQuery.data.html}
                fontSize={typography.fontSize}
                lineHeight={typography.lineHeight}
              />

              <footer className="mt-6 flex items-center justify-between gap-3 border-t border-current/12 pt-6">
                <Button
                  variant="outline"
                  isDisabled={activeChapter <= 0}
                  onPress={() => goToChapter(activeChapter - 1)}
                >
                  <ChevronLeft aria-hidden />
                  {t("readingApp.bookReader.prevChapter")}
                </Button>
                <span className="text-xs opacity-75 tabular-nums">
                  {activeChapter + 1} / {Math.max(1, chapters.length)}
                </span>
                <Button
                  variant="outline"
                  isDisabled={activeChapter >= maxChapter}
                  onPress={() => goToChapter(activeChapter + 1)}
                >
                  {t("readingApp.bookReader.nextChapter")}
                  <ChevronRight aria-hidden />
                </Button>
              </footer>
            </article>
          ) : null}
        </div>
      )}
    </div>
  );
}
