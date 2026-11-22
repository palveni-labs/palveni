import { Button } from "@heroui/react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { ChevronRight, Music2, Play, UsersRound } from "lucide-react";
import { useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import { queryKeys } from "@/api/query-keys";
import { cn } from "@/lib/cn";
import { formatDuration } from "@/lib/format";
import { useElementWidth } from "@/lib/use-element-size";
import { useFeature } from "@/modules/access";
import { Section } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { AlbumCover } from "./album-cover";
import { MusicPlayer } from "./music-player";
import { SearchableMusic } from "./music-search";

function useMusicListing(libraryId: string, artist?: string, album?: string, enabled = true) {
  return useInfiniteQuery({
    queryKey: queryKeys.items.music(libraryId, artist, album),
    queryFn: ({ pageParam, signal }) => api.music.browse(libraryId, artist, album, pageParam, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
    enabled,
  });
}

function ListingBody({ listing, empty, hint, children }: {
  listing: ReturnType<typeof useMusicListing>; empty: boolean; hint?: string; children: ReactNode;
}) {
  const { t } = useTranslation();
  if (listing.isPending) return <LoadingRows />;
  if (listing.isError && empty) return <ErrorState error={listing.error} onRetry={() => void listing.refetch()} />;
  if (empty) return <EmptyState icon={Music2} title={t("music.empty")} detail={hint} />;
  return <>
    {children}
    {listing.isFetchNextPageError ? (
      <ErrorState compact error={listing.error} onRetry={() => void listing.fetchNextPage()} />
    ) : listing.hasNextPage ? (
      <div className="py-3">
        <Button variant="outline" size="sm" isDisabled={listing.isFetchingNextPage} onPress={() => void listing.fetchNextPage()}>{t("music.more")}</Button>
      </div>
    ) : null}
  </>;
}

function TrackDetail({ libraryId, artist, album, artistTitle, albumTitle, artworkId }: {
  libraryId: string; artist: string; album: string; artistTitle: string; albumTitle: string; artworkId?: string;
}) {
  const { t } = useTranslation();
  const artwork = useFeature("musicArtwork");
  const playback = useFeature("personal");
  const [playing, setPlaying] = useState<{ id: string; title: string } | null>(null);
  const listing = useMusicListing(libraryId, artist, album);
  const tracks = listing.data?.pages.flatMap((p) => p.entries).filter((e) => e.kind === "track") ?? [];
  const complete = tracks.length > 0 && !listing.hasNextPage && !listing.isError;
  const duration = complete && tracks.every((e) => e.durationMs != null)
    ? tracks.reduce((sum, e) => sum + (e.durationMs ?? 0), 0) : null;
  return (
    <Section title={t("music.tracks")} bare bodyClassName="px-3 sm:px-4">
      <div className="flex min-w-0 items-center gap-4 border-b border-line py-4">
        <AlbumCover large libraryId={libraryId} id={artwork ? artworkId ?? tracks[0]?.id : undefined} />
        <div className="grid min-w-0 flex-1 gap-1">
          <span className="break-words text-xs font-mid text-fg-muted">{artistTitle}</span>
          <h2 className="break-words font-heading text-xl font-title text-heading">{albumTitle}</h2>
          {complete && <span data-num className="text-xs text-fg-muted tabular-nums">
            {t("music.trackCount", { count: tracks.length })}{duration != null && ` · ${formatDuration(duration)}`}
          </span>}
        </div>
      </div>
      {playing && <MusicPlayer key={playing.id} libraryId={libraryId} id={playing.id} title={playing.title} />}
      <ListingBody listing={listing} empty={!tracks.length}>
        <table aria-label={t("music.tracks")} className="w-full table-fixed border-collapse text-sm">
          <thead><tr className="border-b border-line text-xs text-fg-muted">
            <th scope="col" className="w-8 py-2.5 text-end font-mid"><span aria-hidden>#</span><span className="sr-only">{t("music.trackNumber")}</span></th>
            <th scope="col" className="px-3 py-2.5 text-start font-mid">{t("music.tracks")}</th>
            <th scope="col" className="w-16 py-2.5 text-end font-mid">{t("music.duration")}</th>
            {playback && <th scope="col" className="w-11"><span className="sr-only">{t("music.play")}</span></th>}
          </tr></thead>
          <tbody className="divide-y divide-line">
            {tracks.map((entry) => (
              <tr key={entry.id} className="transition-colors hover:bg-hover">
                <td data-num className="py-2.5 text-end text-fg-muted tabular-nums">{entry.trackNumber ?? "—"}</td>
                <td className="px-3 py-2.5">
                  <div className="wrap-anywhere font-strong text-fg">{entry.title}</div>
                  <div className="flex flex-wrap gap-x-2 text-xs text-fg-muted">
                    <span className="wrap-anywhere">{entry.artist ?? t("music.unknownArtist")}</span>
                    {entry.discNumber != null && <span>{t("music.disc", { number: entry.discNumber })}</span>}
                  </div>
                </td>
                <td data-num className="py-2.5 text-end text-fg-muted tabular-nums">{entry.durationMs != null ? formatDuration(entry.durationMs) : "—"}</td>
                {playback && <td className="text-end">
                  <Button size="sm" isIconOnly variant="ghost" aria-label={t("spoken.play", { title: entry.title })} onPress={() => setPlaying({ id: entry.id, title: entry.title })}>
                    <Play aria-hidden className="size-4" />
                  </Button>
                </td>}
              </tr>
            ))}
          </tbody>
        </table>
      </ListingBody>
    </Section>
  );
}

function MusicGroups({ libraryId, artist, artistName, album, albumName }: {
  libraryId: string; artist?: string; artistName?: string; album?: string; albumName?: string;
}) {
  const { t } = useTranslation();
  const container = useRef<HTMLDivElement>(null);
  const width = useElementWidth(container);
  const split = !!album && width >= 720;
  const listing = useMusicListing(libraryId, artist, undefined, !album || split);
  const groups = listing.data?.pages.flatMap((p) => p.entries).filter((e) => e.kind !== "track") ?? [];
  const selected = groups.find((e) => e.kind === "album" && e.id === album);
  const artwork = useFeature("musicArtwork");
  const artistTitle = artistName || t("music.unknownArtist");
  const albumTitle = selected?.name || albumName || t("music.unknownAlbum");
  const groupTitle = t(artist ? "music.albums" : "music.artists");
  return (
    <div ref={container} className={cn("grid min-w-0 items-start gap-(--pv-gap-section)", split && "grid-cols-[minmax(12rem,0.44fr)_minmax(0,1fr)]")}>
      {(!album || split) && <Section title={groupTitle} count={groups.length > 0 && !listing.hasNextPage ? groups.length : undefined} unframed={!split} bodyClassName={split ? "p-2" : undefined}>
        <ListingBody listing={listing} empty={!groups.length} hint={!artist ? t("music.scanHint") : undefined}>
          <ul className={split ? "grid gap-1" : "grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(min(100%,15rem),1fr))]"}>
            {groups.map((entry) => {
              const name = entry.name ?? t(entry.kind === "artist" ? "music.unknownArtist" : "music.unknownAlbum");
              return <li key={entry.id} className="min-w-0">
                <Link to="/library/$libraryId" params={{ libraryId }}
                  search={entry.kind === "artist" ? { view: "music", artist: entry.id, artistName: entry.name ?? undefined }
                    : { view: "music", artist, artistName, album: entry.id, albumName: entry.name ?? undefined }}
                  aria-current={split && entry.id === album ? "page" : undefined}
                  className={cn("flex min-w-0 items-center gap-3 text-start text-fg transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                    split ? "rounded-tile px-2.5 py-2 aria-[current=page]:bg-sunken aria-[current=page]:ring-1 aria-[current=page]:ring-line-strong" : "rounded-panel border border-line bg-surface p-3.5")}>
                  {entry.kind === "artist" ? <span className="grid size-9 shrink-0 place-items-center rounded-tile bg-sunken text-icon"><UsersRound aria-hidden className="size-4" /></span>
                    : <AlbumCover libraryId={libraryId} id={artwork ? entry.artworkItemId : undefined} />}
                  <span className="grid min-w-0 flex-1 gap-0.5">
                    <span className="truncate font-strong" title={name}>{name}</span>
                    {entry.kind === "album" && !split && <span aria-hidden className="truncate text-xs text-fg-muted">{artistTitle}</span>}
                  </span>
                  {!split && <ChevronRight className="size-4 shrink-0 text-fg-subtle" aria-hidden />}
                </Link>
              </li>;
            })}
          </ul>
        </ListingBody>
      </Section>}
      {artist && album && <TrackDetail key={`${artist}/${album}`} libraryId={libraryId} artist={artist} album={album}
        artistTitle={artistTitle} albumTitle={albumTitle} artworkId={selected?.kind === "album" ? selected.artworkItemId : undefined} />}
    </div>
  );
}

export function MusicBrowse({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const { artist, album, artistName, albumName } = useSearch({ from: "/_app/library/$libraryId" });
  if (!artist) return <SearchableMusic libraryId={libraryId}><MusicGroups libraryId={libraryId} /></SearchableMusic>;
  const crumb = "rounded-control px-1.5 py-0.5 hover:bg-hover hover:text-fg";
  return <>
    <nav aria-label={t("a11y.breadcrumb")} className="flex flex-wrap items-center gap-1.5 text-sm text-fg-muted">
      <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "music" }} className={crumb}>{t("music.artists")}</Link>
      <ChevronRight className="size-3.5 text-fg-subtle" aria-hidden />
      {album ? <>
        <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "music", artist, artistName }} className={crumb}>{artistName || t("music.unknownArtist")}</Link>
        <ChevronRight className="size-3.5 text-fg-subtle" aria-hidden />
        <span aria-current="page" className="font-strong text-fg">{albumName || t("music.unknownAlbum")}</span>
      </> : <span aria-current="page" className="font-strong text-fg">{artistName || t("music.unknownArtist")}</span>}
    </nav>
    <MusicGroups libraryId={libraryId} artist={artist} artistName={artistName} album={album} albumName={albumName} />
  </>;
}
