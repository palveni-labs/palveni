import { useQueries } from "@tanstack/react-query";
import { Heart } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLibraries } from "@/api/queries/libraries";
import { cn } from "@/lib/cn";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { videoApi, type VideoEntry } from "../api";
import { PosterCard } from "../components/poster-card";
import { videoKeys } from "../queries";

export function VideoFavoritesPage() {
  const { t } = useTranslation();
  const librariesQuery = useLibraries();
  const [kind, setKind] = useState<"movie" | "series" | undefined>(undefined);
  const [libraryId, setLibraryId] = useState<string>("");

  const videoLibraries = useMemo(
    () => (librariesQuery.data?.libraries ?? []).filter((lib) => lib.kind === "video"),
    [librariesQuery.data],
  );
  const activeLibraries = useMemo(
    () => (libraryId ? videoLibraries.filter((l) => l.id === libraryId) : videoLibraries),
    [videoLibraries, libraryId],
  );

  const favQueries = useQueries({
    queries: activeLibraries.map((lib) => ({
      queryKey: videoKeys.list(lib.id, { favorite: true, kind, order: "added", dir: "desc", limit: 200 }),
      queryFn: ({ signal }: { signal: AbortSignal }) =>
        videoApi.list(lib.id, { favorite: true, kind, order: "added", dir: "desc", limit: 200 }, signal),
      staleTime: 20_000,
    })),
  });

  const isPending = librariesQuery.isPending || favQueries.some((q) => q.isPending);
  const firstError = favQueries.find((q) => q.isError)?.error;

  const entries = useMemo(() => {
    const merged: VideoEntry[] = [];
    for (const q of favQueries) {
      if (q.data) merged.push(...q.data.entries);
    }
    return merged.sort((a, b) => b.addedAt.localeCompare(a.addedAt));
  }, [favQueries]);

  return (
    <div className="pv-enter pv-page-frame mx-auto flex w-full max-w-(--pv-layout-max) flex-col gap-5 px-(--pv-layout-gutter) pt-(--pv-layout-top) pb-(--pv-layout-bottom)">
      <PageHeader
        title={t("videoApp.favorites.title")}
        summary={!isPending && entries.length > 0 ? t("videoApp.favorites.count", { count: entries.length }) : undefined}
      />

      <div className="flex flex-wrap items-center gap-2">
        <div className="pv-segmented w-fit">
          {([
            [undefined, "all"],
            ["movie", "movie"],
            ["series", "series"],
          ] as const).map(([val, key]) => {
            const selected = kind === val;
            return (
              <button
                key={key}
                type="button"
                aria-pressed={selected}
                onClick={() => setKind(val)}
                className={cn(
                  "flex h-[calc(var(--pv-control-h-sm)-4px)] items-center rounded-control px-2.5 text-xs transition-colors",
                  selected ? "bg-raised font-strong text-fg shadow-raised" : "font-mid text-fg-muted hover:text-fg",
                )}
              >
                {t(`videoApp.library.tabs.${key}`)}
              </button>
            );
          })}
        </div>

        {videoLibraries.length > 1 ? (
          <select
            aria-label={t("videoApp.home.libraries")}
            value={libraryId}
            onChange={(e) => setLibraryId(e.target.value)}
            className="h-(--pv-control-h-sm) rounded-control border border-line bg-surface px-2.5 text-xs text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <option value="">{t("videoApp.search.allLibraries")}</option>
            {videoLibraries.map((lib) => (
              <option key={lib.id} value={lib.id}>
                {lib.name}
              </option>
            ))}
          </select>
        ) : null}
      </div>

      {isPending ? (
        <LoadingRows rows={6} />
      ) : firstError && entries.length === 0 ? (
        <ErrorState
          error={firstError}
          onRetry={() => {
            for (const q of favQueries) void q.refetch();
          }}
        />
      ) : entries.length === 0 ? (
        <EmptyState
          icon={Heart}
          title={t("videoApp.favorites.empty")}
          detail={t("videoApp.favorites.emptyDetail")}
        />
      ) : (
        <div
          role="list"
          className="pv-grid-poster"
        >
          {entries.map((entry) => (
            <div key={`${entry.libraryId}:${entry.id}`} role="listitem" className="min-w-0">
              <PosterCard entry={entry} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
