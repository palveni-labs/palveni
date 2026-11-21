import { Button, Input } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { SearchX } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import { isApiError } from "@/api/errors";
import type { components } from "@/api/generated";
import { queryKeys } from "@/api/query-keys";
import { useFeature } from "@/modules/access";
import { FeatureUnavailable } from "@/shell/feature-gate";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { MusicSearchKinds, MusicSearchRows, type SearchKind } from "./browse/music-search";

function Results({ q, kind }: { q: string; kind: SearchKind }) {
  const { t } = useTranslation();
  const [cursors, setCursors] = useState<string[]>([]);
  const cursor = cursors.at(-1);
  const result = useQuery({
    queryKey: [...queryKeys.items.globalMusicSearch, kind, q, cursor ?? null],
    queryFn: ({ signal }) => request<components["schemas"]["GlobalMusicSearchPage"]>("/music/search", {
      query: { q, kind, cursor }, signal,
    }),
  });

  if (cursor && isApiError(result.error, "conflict")) {
    setCursors([]);
    return <LoadingRows />;
  }
  if (result.isPending) return <LoadingRows />;
  if (result.isError) return <ErrorState compact error={result.error} onRetry={() => void result.refetch()} />;
  const { entries, nextCursor } = result.data;
  if (!entries.length) return <EmptyState compact icon={SearchX} title={t("musicSearch.noResults", { q })} />;
  return <>
    <MusicSearchRows key={cursor ?? ""} hits={entries} kind={kind} />
    {(cursor || nextCursor) && <div className="flex gap-2">
      <Button size="sm" variant="outline" isDisabled={!cursor} onPress={() => setCursors((pages) => pages.slice(0, -1))}>
        {t("libraryHome.previous")}
      </Button>
      <Button size="sm" variant="outline" isDisabled={!nextCursor || result.isFetching} onPress={() => {
        if (nextCursor) setCursors((pages) => [...pages, nextCursor]);
      }}>{t("libraryHome.next")}</Button>
    </div>}
  </>;
}

export function GlobalMusicSearch() {
  const { t } = useTranslation();
  const enabled = useFeature("globalMusicSearch");
  const { q = "", kind = "track" } = useSearch({ from: "/_app/library/music-search" });
  const navigate = useNavigate({ from: "/library/music-search" });
  if (!enabled) return <FeatureUnavailable titleKey="musicSearch.globalLabel" />;
  return <PageFrame>
    <PageHeader title={t("musicSearch.globalLabel")} />
    <form role="search" className="flex gap-2" onSubmit={(event) => {
      event.preventDefault();
      const input = new FormData(event.currentTarget).get("q");
      if (typeof input === "string") void navigate({ search: { q: input.trim() || undefined, kind } });
    }}>
      <Input key={q} name="q" type="search" defaultValue={q} aria-label={t("musicSearch.globalLabel")}
        placeholder={t("musicSearch.globalLabel")} className="min-w-0 flex-1" />
      <Button type="submit" variant="outline">{t("musicSearch.submit")}</Button>
    </form>
    {q && <>
      <MusicSearchKinds kind={kind} onChange={(category) => void navigate({ search: { q, kind: category } })} />
      <Results key={`${kind}/${q}`} q={q} kind={kind} />
    </>}
  </PageFrame>;
}
