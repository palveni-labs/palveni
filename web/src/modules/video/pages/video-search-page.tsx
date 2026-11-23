import { useQueries } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Film, Search, SearchX } from "lucide-react";
import { useMemo, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useLibraries } from "@/api/queries/libraries";
import { cn } from "@/lib/cn";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { videoApi, type VideoEntry } from "../api";
import { PosterCard } from "../components/poster-card";
import { videoKeys } from "../queries";

export interface VideoSearchParams {
  q?: string;
  libraryId?: string;
  kind?: "movie" | "series";
}

export function VideoSearchPage() {
  const { t } = useTranslation();
  const search = useSearch({ from: "/_app/video/search" });
  const navigate = useNavigate({ from: "/video/search" });
  const librariesQuery = useLibraries();

  const videoLibraries = useMemo(
    () => (librariesQuery.data?.libraries ?? []).filter((lib) => lib.kind === "video"),
    [librariesQuery.data],
  );

  const queryText = (search.q ?? "").trim();
  const activeLibraries = useMemo(
    () => (search.libraryId ? videoLibraries.filter((l) => l.id === search.libraryId) : videoLibraries),
    [videoLibraries, search.libraryId],
  );

  const resultQueries = useQueries({
    queries: activeLibraries.map((lib) => ({
      queryKey: videoKeys.list(lib.id, { q: queryText, kind: search.kind, order: "title", dir: "asc", limit: 60 }),
      queryFn: ({ signal }: { signal: AbortSignal }) =>
        videoApi.list(lib.id, { q: queryText, kind: search.kind, order: "title", dir: "asc", limit: 60 }, signal),
      enabled: queryText.length > 0,
      staleTime: 20_000,
    })),
  });

  const isPending = queryText.length > 0 && resultQueries.some((q) => q.isPending);
  const firstError = resultQueries.find((q) => q.isError)?.error;

  const { entries, total } = useMemo(() => {
    const merged: VideoEntry[] = [];
    let sum = 0;
    for (const q of resultQueries) {
      if (q.data) {
        merged.push(...q.data.entries);
        sum += q.data.total;
      }
    }
    return { entries: merged, total: sum };
  }, [resultQueries]);

  const updateSearch = (patch: Partial<VideoSearchParams>) => {
    void navigate({
      search: (prev) => {
        const next: VideoSearchParams = { ...prev, ...patch };
        if (!next.q) delete next.q;
        if (!next.libraryId) delete next.libraryId;
        if (!next.kind) delete next.kind;
        return next;
      },
      replace: true,
    });
  };

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const raw = new FormData(event.currentTarget).get("q");
    const trimmed = typeof raw === "string" ? raw.trim() : "";
    updateSearch({ q: trimmed || undefined });
  };

  return (
    <div className="pv-enter mx-auto flex w-full max-w-[88rem] flex-col gap-5 px-(--pv-gutter) pt-5 pb-14 max-md:px-4 max-md:pt-4">
      <PageHeader
        title={t("videoApp.search.title")}
        summary={queryText && !isPending ? t("videoApp.search.resultsCount", { count: total }) : undefined}
      />

      <div className="flex flex-col gap-3">
        <form role="search" onSubmit={handleSubmit} className="relative flex w-full max-w-xl items-center">
          <Search className="pointer-events-none absolute left-3 size-4 text-fg-subtle" aria-hidden />
          <input
            key={search.q ?? ""}
            type="search"
            name="q"
            defaultValue={search.q ?? ""}
            autoFocus={!search.q}
            onChange={(e) => {
              if (!e.target.value.trim() && search.q) updateSearch({ q: undefined });
            }}
            aria-label={t("videoApp.search.placeholder")}
            placeholder={t("videoApp.search.placeholder")}
            className="h-(--pv-control-h) w-full rounded-field border border-line bg-surface ps-9 pe-3 text-sm text-fg placeholder:text-fg-subtle focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          />
        </form>

        <div className="flex flex-wrap items-center gap-2">
          <div className="pv-segmented w-fit">
            {([
              [undefined, "all"],
              ["movie", "movie"],
              ["series", "series"],
            ] as const).map(([kindVal, key]) => {
              const selected = search.kind === kindVal;
              return (
                <button
                  key={key}
                  type="button"
                  aria-pressed={selected}
                  onClick={() => updateSearch({ kind: kindVal })}
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
              value={search.libraryId ?? ""}
              onChange={(e) => updateSearch({ libraryId: e.target.value || undefined })}
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
      </div>

      {!queryText ? (
        <EmptyState
          icon={Film}
          title={t("videoApp.search.prompt")}
          detail={t("videoApp.search.promptDetail")}
        />
      ) : isPending ? (
        <LoadingRows rows={6} />
      ) : firstError && entries.length === 0 ? (
        <ErrorState
          error={firstError}
          onRetry={() => {
            for (const q of resultQueries) void q.refetch();
          }}
        />
      ) : entries.length === 0 ? (
        <EmptyState icon={SearchX} title={t("videoApp.search.empty")} />
      ) : (
        <div
          role="list"
          className="grid grid-cols-2 gap-3 sm:grid-cols-3 sm:gap-4 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6"
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
