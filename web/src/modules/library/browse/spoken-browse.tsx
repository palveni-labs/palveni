import { Button } from "@heroui/react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { ChevronRight, Headphones, Play, Podcast } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { queryKeys } from "@/api/query-keys";
import { formatDuration } from "@/lib/format";
import { Section } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { SpokenPlayer } from "./spoken-player";

type BrowseProps = { libraryId: string; kind: "audiobook" | "podcast" };

export function SpokenBrowse(props: BrowseProps) {
  const { collection, collectionName } = useSearch({ from: "/_app/library/$libraryId" });
  return <SpokenCollection key={`${props.libraryId}:${collection ?? ""}`} {...props} collection={collection} collectionName={collectionName} />;
}

function SpokenCollection({ libraryId, kind, collection, collectionName }: BrowseProps & { collection?: string; collectionName?: string }) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<{ id: string; title: string } | null>(null);
  const result = useInfiniteQuery({
    queryKey: [...queryKeys.items.libraryLists(libraryId), "spoken", collection],
    queryFn: ({ pageParam, signal }) => request<components["schemas"]["SpokenPage"]>(
      `/libraries/${encodeURIComponent(libraryId)}/spoken`, { signal, query: { collection, cursor: pageParam } },
    ),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });
  const entries = result.data?.pages.flatMap((page) => page.entries) ?? [];
  const Icon = kind === "audiobook" ? Headphones : Podcast;
  const shelf = t(kind === "audiobook" ? "spoken.shelf" : "spoken.shows");
  const title = t(kind === "audiobook" ? "spoken.chapters" : "spoken.episodes");
  return (
    <>
      {collection && <nav aria-label={t("a11y.breadcrumb")} className="flex items-center gap-2 text-sm text-fg-muted">
        <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "spoken" }}>{shelf}</Link>
        <ChevronRight className="size-4 shrink-0" aria-hidden /><span aria-current="page">{collectionName || title}</span>
      </nav>}
      {selected && <SpokenPlayer key={selected.id} id={selected.id} title={selected.title} audiobook={kind === "audiobook"} />}
      <Section title={collection ? title : shelf} bare>
        {result.isPending ? <LoadingRows /> : result.isError && !entries.length ? (
          <ErrorState error={result.error} onRetry={() => void result.refetch()} />
        ) : !entries.length ? (
          <EmptyState icon={Icon} title={t("spoken.empty")} detail={collection ? undefined : t("music.scanHint")} />
        ) : (
          <ul className="divide-y divide-line">
            {entries.map((entry) => <li key={entry.id}>
              {entry.kind === "book" || entry.kind === "show" ? (
                <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "spoken", collection: entry.id, collectionName: entry.title }}
                  className="flex min-w-0 items-center gap-3 rounded-control px-2 py-3 text-start hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)">
                  <Icon className="size-5 shrink-0 text-fg-muted" aria-hidden />
                  <span className="min-w-0 flex-1 break-words font-mid">{entry.title}</span><ChevronRight aria-hidden className="size-4 text-fg-muted" />
                </Link>
              ) : (
                <div className="flex min-w-0 items-center gap-3 px-2 py-3">
                  {entry.number != null && <span className="text-sm tabular-nums text-fg-muted">{entry.number}</span>}
                  <div className="min-w-0 flex-1"><div className="break-words font-mid">{entry.title}</div>
                    {entry.artist && <div className="text-sm text-fg-muted">{entry.artist}</div>}
                  </div>
                  {entry.durationMs != null && <span className="text-sm tabular-nums text-fg-muted">{formatDuration(entry.durationMs)}</span>}
                  <Button variant="ghost" size="sm" isIconOnly aria-label={t("spoken.play", { title: entry.title })}
                    onPress={() => setSelected({ id: entry.id, title: entry.title })}>
                    <Play className="size-4" aria-hidden />
                  </Button>
                </div>
              )}
            </li>)}
          </ul>
        )}
        {result.isFetchNextPageError && <ErrorState error={result.error} onRetry={() => void result.fetchNextPage()} compact />}
        {result.hasNextPage && !result.isFetchNextPageError && <Button variant="outline" size="sm" isDisabled={result.isFetchingNextPage} onPress={() => void result.fetchNextPage()}>{t("music.more")}</Button>}
      </Section>
    </>
  );
}
