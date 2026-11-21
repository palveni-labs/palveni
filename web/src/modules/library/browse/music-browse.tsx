import { Button } from "@heroui/react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { ChevronRight, Disc3, Music2, UsersRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import { queryKeys } from "@/api/query-keys";
import { formatDuration } from "@/lib/format";
import { Section } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

export function MusicBrowse({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const { artist, album, artistName, albumName } = useSearch({ from: "/_app/library/$libraryId" });
  const listing = useInfiniteQuery({
    queryKey: queryKeys.items.music(libraryId, artist, album),
    queryFn: ({ pageParam, signal }) => api.music.browse(libraryId, artist, album, pageParam, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
  });
  const entries = listing.data?.pages.flatMap((page) => page.entries) ?? [];
  const group = album ? t("music.tracks") : artist ? t("music.albums") : t("music.artists");
  const artistTitle = artistName || t("music.unknownArtist");
  const albumTitle = albumName || t("music.unknownAlbum");
  const rowClass = "flex min-w-0 items-center gap-3 rounded-control px-2 py-3 text-start text-fg hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)";

  return (
    <>
      {artist && <nav aria-label={t("a11y.breadcrumb")} className="flex flex-wrap items-center gap-2 text-sm text-fg-muted">
        <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "music" }}>{t("music.artists")}</Link>
        <ChevronRight className="size-4" aria-hidden />
        {album ? <>
          <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "music", artist, artistName }}>{artistTitle}</Link>
          <ChevronRight className="size-4" aria-hidden /><span aria-current="page">{albumTitle}</span>
        </> : <span aria-current="page">{artistTitle}</span>}
      </nav>}
      <Section title={group} bare>
        {listing.isPending ? <LoadingRows /> : listing.isError && entries.length === 0 ? (
          <ErrorState error={listing.error} onRetry={() => void listing.refetch()} />
        ) : entries.length === 0 ? (
          <EmptyState icon={Music2} title={t("music.empty")} detail={artist ? undefined : t("music.scanHint")} />
        ) : (
          <ul className="divide-y divide-line">
            {entries.map((entry) => (
              <li key={entry.id}>
                {entry.kind === "track" ? <div className="flex min-w-0 items-center gap-3 px-2 py-3">
                  <span className="w-8 shrink-0 text-end text-sm tabular-nums text-fg-muted">{entry.trackNumber ?? "—"}</span>
                  <div className="min-w-0 flex-1"><div className="break-words font-strong">{entry.title}</div>
                    <div className="flex flex-wrap gap-x-2 text-sm text-fg-muted"><span>{entry.artist ?? t("music.unknownArtist")}</span>{entry.discNumber != null && <span>{t("music.disc", { number: entry.discNumber })}</span>}</div>
                  </div>
                  {entry.durationMs != null && <span className="text-sm tabular-nums text-fg-muted">{formatDuration(entry.durationMs)}</span>}
                </div> : (
                  <Link to="/library/$libraryId" params={{ libraryId }} search={entry.kind === "artist"
                    ? { view: "music", artist: entry.id, artistName: entry.name ?? undefined }
                    : { view: "music", artist, artistName, album: entry.id, albumName: entry.name ?? undefined }}
                    className={rowClass}>
                    {entry.kind === "artist" ? <UsersRound className="size-5 shrink-0 text-fg-muted" aria-hidden /> : <Disc3 className="size-5 shrink-0 text-fg-muted" aria-hidden />}
                    <span className="min-w-0 flex-1 break-words">{entry.name ?? t(entry.kind === "artist" ? "music.unknownArtist" : "music.unknownAlbum")}</span>
                    <ChevronRight className="size-4 shrink-0 text-fg-muted" aria-hidden />
                  </Link>
                )}
              </li>
            ))}
          </ul>
        )}
        {listing.isFetchNextPageError && <ErrorState error={listing.error} onRetry={() => void listing.fetchNextPage()} compact />}
        {listing.hasNextPage && !listing.isFetchNextPageError && <Button variant="outline" size="sm" isDisabled={listing.isFetchingNextPage} onPress={() => void listing.fetchNextPage()}>{t("music.more")}</Button>}
      </Section>
    </>
  );
}
