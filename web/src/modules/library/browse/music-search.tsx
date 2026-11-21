import { Button } from "@heroui/react";
import { useInfiniteQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Disc3, Play, Search, SearchX, UsersRound } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import { isApiError } from "@/api/errors";
import type { components } from "@/api/generated";
import { queryKeys } from "@/api/query-keys";
import { useFeature } from "@/modules/access";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { MusicPlayer } from "./music-player";

export type SearchKind = components["schemas"]["MusicSearchKind"];
type SearchHit = components["schemas"]["MusicSearchHit"];
type SearchPage = components["schemas"]["MusicSearchPage"];
const categories = { track: "music.tracks", artist: "music.artists", album: "music.albums" } as const;

type SourcedHit = SearchHit & { libraryId: string; libraryName?: string };

export function MusicSearchKinds({ kind, onChange }: { kind: SearchKind; onChange: (kind: SearchKind) => void }) {
  const { t } = useTranslation();
  return (
    <div role="group" aria-label={t("musicSearch.category")} className="flex flex-wrap gap-2">
      {(["track", "artist", "album"] as const).map((category) => (
        <Button size="sm" key={category} variant={kind === category ? "primary" : "outline"}
          aria-pressed={kind === category} onPress={() => onChange(category)}>
          {t(categories[category])}
        </Button>
      ))}
    </div>
  );
}

export function MusicSearchRows({ hits, kind }: { hits: SourcedHit[]; kind: SearchKind }) {
  const { t } = useTranslation();
  const [playing, setPlaying] = useState<string>();
  const playback = useFeature("music");
  const personal = useFeature("personal");
  const canPlay = playback && personal;
  return (
    <ul className="divide-y divide-line">
      {hits.map((hit) => (
        <li key={`${hit.libraryId}/${hit.artistId ?? ""}/${hit.id}`}>
          {kind === "track" ? <>
            <div className="flex min-w-0 items-center gap-3 py-3">
              <div className="min-w-0 flex-1">
                <div className="break-words font-strong">{hit.title}</div>
                <div className="break-words text-sm text-fg-muted">
                  {[hit.libraryName, hit.artist, hit.album].filter(Boolean).join(" · ")}
                </div>
              </div>
              {canPlay && <Button isIconOnly variant="ghost" onPress={() => setPlaying(hit.id)}
                aria-label={[t("spoken.play", { title: hit.title }), hit.libraryName].filter(Boolean).join(" · ")}>
                <Play aria-hidden />
              </Button>}
            </div>
            {canPlay && playing === hit.id && <MusicPlayer libraryId={hit.libraryId} id={hit.id} title={hit.title} />}
          </> : (
            <Link to="/library/$libraryId" params={{ libraryId: hit.libraryId }}
              search={kind === "artist" ? { view: "music", artist: hit.id, artistName: hit.title }
                : { view: "music", artist: hit.artistId ?? undefined, artistName: hit.artist ?? undefined, album: hit.id, albumName: hit.title }}
              className="flex min-w-0 items-center gap-3 rounded-control px-2 py-3 hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)">
              {kind === "artist" ? <UsersRound aria-hidden className="size-5 shrink-0" /> : <Disc3 aria-hidden className="size-5 shrink-0" />}
              <span className="min-w-0 break-words">
                <span className="block font-strong">{hit.title}</span>{" "}
                <span className="text-sm text-fg-muted">{[hit.libraryName, hit.artist].filter(Boolean).join(" · ")}</span>
              </span>
            </Link>
          )}
        </li>
      ))}
    </ul>
  );
}

function Results({ libraryId, q, kind }: { libraryId: string; q: string; kind: SearchKind }) {
  const { t } = useTranslation();
  const client = useQueryClient();
  const key = [...queryKeys.items.libraryLists(libraryId), "search", kind, q] as const;
  const listing = useInfiniteQuery({
    queryKey: key,
    queryFn: ({ pageParam, signal }) => request<SearchPage>(`/libraries/${encodeURIComponent(libraryId)}/music/search`, {
      query: { q, kind, cursor: pageParam }, signal,
    }),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
    retry: false,
  });
  const stalePage = listing.isFetchNextPageError && isApiError(listing.error, "conflict");
  useEffect(() => {
    if (stalePage) void client.resetQueries({ queryKey: [...queryKeys.items.libraryLists(libraryId), "search", kind, q], exact: true });
  }, [client, kind, libraryId, q, stalePage]);

  if (listing.isPending || stalePage) return <LoadingRows />;
  const hits = listing.data?.pages.flatMap((page) => page.entries) ?? [];
  if (listing.isError && !hits.length) return <ErrorState compact error={listing.error} onRetry={() => void listing.refetch()} />;
  if (!hits.length) return <EmptyState compact icon={SearchX} title={t("musicSearch.noResults", { q })} />;
  return <>
    <MusicSearchRows hits={hits.map((hit) => ({ ...hit, libraryId }))} kind={kind} />
    {listing.isFetchNextPageError && <ErrorState compact error={listing.error} onRetry={() => void listing.fetchNextPage()} />}
    {listing.hasNextPage && !listing.isFetchNextPageError && <Button variant="outline" size="sm" isDisabled={listing.isFetchingNextPage} onPress={() => void listing.fetchNextPage()}>{t("music.more")}</Button>}
  </>;
}

export function SearchableMusic({ libraryId, children }: { libraryId: string; children: ReactNode }) {
  const { t } = useTranslation();
  const available = useFeature("musicSearch");
  const [input, setInput] = useState("");
  const [q, setQuery] = useState("");
  const [kind, setKind] = useState<SearchKind>("track");
  const trimmed = input.trim();
  useEffect(() => {
    const timeout = window.setTimeout(() => setQuery(trimmed), 230);
    return () => window.clearTimeout(timeout);
  }, [trimmed]);
  if (!available) return children;
  return <>
    <div role="search" className="grid gap-3">
      <label className="flex items-center gap-2 rounded-control border border-line bg-surface px-3 focus-within:outline-2 focus-within:outline-(--pv-focus)">
        <Search aria-hidden className="size-4 shrink-0 text-fg-muted" />
        <input type="search" aria-label={t("musicSearch.label")} placeholder={t("musicSearch.label")} value={input}
          onChange={(event) => setInput(event.target.value)} className="min-w-0 flex-1 bg-transparent py-2 text-fg placeholder:text-fg-muted focus:outline-none" />
      </label>
      {trimmed && <MusicSearchKinds kind={kind} onChange={setKind} />}
    </div>
    {!trimmed ? children : q !== trimmed ? <LoadingRows /> : <Results key={`${kind}/${q}`} libraryId={libraryId} q={q} kind={kind} />}
  </>;
}
