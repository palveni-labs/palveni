import { Button, Input, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Disc3, Search, SearchX, UserRound } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useFeature } from "@/modules/access";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { lookupTracks, musicApi, type MusicAlbum, type MusicArtist, type MusicTrack } from "../api";
import { AlbumCard } from "../components/album-card";
import { ArtistCard } from "../components/artist-card";
import { TrackRow } from "../components/track-row";
import { MusicLibraryGuard, MusicLibrarySwitcher } from "../library-context";
import { useMusicLibraries } from "../library-store";
import { musicKeys } from "../queries";

export type SearchScope = "all" | "library";
export type SearchTab = "all" | "songs" | "albums" | "artists";

interface SearchBundle {
  tracks: MusicTrack[];
  albums: Array<{ id: string; libraryId: string; libraryName?: string; name: string; artist: string | null; artistId: string | null; albumObj?: MusicAlbum }>;
  artists: Array<{ id: string; libraryId: string; libraryName?: string; name: string; artistObj?: MusicArtist }>;
}

function SearchResults({
  q,
  scope,
  tab,
  libraryId,
}: {
  q: string;
  scope: SearchScope;
  tab: SearchTab;
  libraryId: string;
}) {
  const { t } = useTranslation();
  const globalSearchEnabled = useFeature("globalMusicSearch");
  const useCrossLibrary = scope === "all" && globalSearchEnabled;

  const query = useQuery({
    queryKey: musicKeys.search(useCrossLibrary ? "all" : libraryId, q, "bundle"),
    queryFn: async ({ signal }): Promise<SearchBundle> => {
      if (useCrossLibrary) {
        const [trackHits, albumHits, artistHits] = await Promise.all([
          musicApi.searchAll(q, "track", 30, null, signal),
          musicApi.searchAll(q, "album", 18, null, signal),
          musicApi.searchAll(q, "artist", 18, null, signal),
        ]);
        const trackIds = trackHits.entries.map((e) => e.id);
        const hydrated = trackIds.length > 0 ? await lookupTracks(trackIds, signal) : [];
        return {
          tracks: hydrated,
          albums: albumHits.entries.map((entry) => ({
            id: entry.id,
            libraryId: entry.libraryId,
            libraryName: entry.libraryName,
            name: entry.title,
            artist: entry.artist,
            artistId: entry.artistId,
          })),
          artists: artistHits.entries.map((entry) => ({
            id: entry.id,
            libraryId: entry.libraryId,
            libraryName: entry.libraryName,
            name: entry.title,
          })),
        };
      }
      // Single-library rich catalogue search: returns full MusicTrack / MusicAlbum / MusicArtist objects with covers!
      const [tracksPage, albumsPage, artistsPage] = await Promise.all([
        musicApi.tracks(libraryId, { q, limit: 30 }, signal),
        musicApi.albums(libraryId, { q, limit: 18 }, signal),
        musicApi.artists(libraryId, { q, limit: 18 }, signal),
      ]);
      return {
        tracks: tracksPage.tracks,
        albums: albumsPage.albums.map((album) => ({
          id: album.id,
          libraryId: album.libraryId,
          name: album.name ?? t("musicApp.albums.unknown"),
          artist: album.artist,
          artistId: album.artistId,
          albumObj: album,
        })),
        artists: artistsPage.artists.map((artist) => ({
          id: artist.id,
          libraryId: artist.libraryId,
          name: artist.name ?? t("musicApp.artists.unknown"),
          artistObj: artist,
        })),
      };
    },
    enabled: q.length > 0,
  });

  if (query.isPending) return <LoadingRows rows={6} />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />;

  const { tracks, albums, artists } = query.data;
  const showTracks = (tab === "all" || tab === "songs") && tracks.length > 0;
  const showAlbums = (tab === "all" || tab === "albums") && albums.length > 0;
  const showArtists = (tab === "all" || tab === "artists") && artists.length > 0;

  if (!showTracks && !showAlbums && !showArtists) {
    return <EmptyState icon={SearchX} title={t("musicApp.search.empty", { q })} />;
  }

  return (
    <div className="grid gap-8">
      {showArtists ? (
        <section aria-label={t("musicApp.artists.title")} className="grid gap-3">
          <h2 className="font-heading text-lg font-title text-heading">{t("musicApp.artists.title")}</h2>
          {artists[0]?.artistObj ? (
            <div className="pv-grid-square">
              {artists.slice(0, tab === "all" ? 6 : 18).map((entry) => (
                <ArtistCard key={`${entry.libraryId}:${entry.id}`} artist={entry.artistObj!} />
              ))}
            </div>
          ) : (
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {artists.map((entry) => (
                <Link
                  key={`${entry.libraryId}:${entry.id}`}
                  to="/music/artists/$libraryId/$artistId"
                  params={{ libraryId: entry.libraryId, artistId: entry.id }}
                  className="flex items-center gap-3 rounded-tile border border-line bg-surface p-3 transition-colors hover:bg-hover"
                >
                  <span className="grid size-10 shrink-0 place-items-center rounded-full bg-sunken text-icon">
                    <UserRound className="size-4" aria-hidden />
                  </span>
                  <span className="grid min-w-0 flex-1">
                    <span className="truncate text-sm font-strong text-fg">{entry.name}</span>
                    {entry.libraryName ? <span className="truncate text-xs text-fg-muted">{entry.libraryName}</span> : null}
                  </span>
                </Link>
              ))}
            </div>
          )}
        </section>
      ) : null}

      {showAlbums ? (
        <section aria-label={t("musicApp.albums.title")} className="grid gap-3">
          <h2 className="font-heading text-lg font-title text-heading">{t("musicApp.albums.title")}</h2>
          {albums[0]?.albumObj ? (
            <div className="pv-grid-square">
              {albums.slice(0, tab === "all" ? 6 : 18).map((entry) => (
                <AlbumCard key={`${entry.libraryId}:${entry.id}`} album={entry.albumObj!} />
              ))}
            </div>
          ) : (
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {albums.map((entry) => (
                <Link
                  key={`${entry.libraryId}:${entry.id}`}
                  to="/music/albums/$libraryId/$albumId"
                  params={{ libraryId: entry.libraryId, albumId: entry.id }}
                  className="flex items-center gap-3 rounded-tile border border-line bg-surface p-3 transition-colors hover:bg-hover"
                >
                  <span className="grid size-10 shrink-0 place-items-center rounded-tile bg-sunken text-icon">
                    <Disc3 className="size-4" aria-hidden />
                  </span>
                  <span className="grid min-w-0 flex-1">
                    <span className="truncate text-sm font-strong text-fg">{entry.name}</span>
                    <span className="truncate text-xs text-fg-muted">
                      {[entry.artist, entry.libraryName].filter(Boolean).join(" · ")}
                    </span>
                  </span>
                </Link>
              ))}
            </div>
          )}
        </section>
      ) : null}

      {showTracks ? (
        <section aria-label={t("musicApp.songs.title")} className="grid gap-2">
          <h2 className="font-heading text-lg font-title text-heading">{t("musicApp.songs.title")}</h2>
          <div className="divide-y divide-line/60">
            {tracks.slice(0, tab === "all" ? 10 : 30).map((track, index) => (
              <TrackRow key={track.id} track={track} indexLabel={index + 1} contextTracks={tracks} showCover showAlbum />
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}

export function MusicSearchPage() {
  const { t } = useTranslation();
  const { q = "", scope = "library", tab = "all" } = useSearch({ from: "/_app/music/search" });
  const navigate = useNavigate({ from: "/music/search" });
  const { libraries } = useMusicLibraries();
  const globalSearchEnabled = useFeature("globalMusicSearch");
  const [draft, setDraft] = useState<{ base: string; value: string }>(() => ({ base: q, value: q }));
  const input = draft.base === q ? draft.value : q;
  const setInput = (value: string) => setDraft({ base: q, value });

  const effectiveScope: SearchScope = libraries.length > 1 && globalSearchEnabled ? scope : "library";

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    const trimmed = input.trim() || undefined;
    void navigate({ search: (prev) => ({ ...prev, q: trimmed }), replace: true });
  };

  useEffect(() => {
    const timer = setTimeout(() => {
      const trimmed = input.trim();
      if (trimmed !== q) {
        void navigate({ search: (prev) => ({ ...prev, q: trimmed || undefined }), replace: true });
      }
    }, 260);
    return () => clearTimeout(timer);
  }, [input, q, navigate]);

  return (
    <PageFrame>
      <PageHeader title={t("musicApp.search.title")} />
      <MusicLibraryGuard>
        {(library) => (
          <div className="grid gap-5">
            <form role="search" onSubmit={onSubmit} className="flex flex-wrap items-center gap-2">
              <Input
                autoFocus
                type="search"
                value={input}
                onChange={(event) => setInput(event.target.value)}
                aria-label={t("musicApp.search.title")}
                placeholder={t("musicApp.search.placeholder")}
                className="min-w-52 flex-1"
              />
              <Button type="submit" variant="primary" size="sm">
                <Search aria-hidden />
                {t("musicApp.nav.search")}
              </Button>
              {libraries.length > 1 && globalSearchEnabled ? (
                <ToggleButtonGroup
                  aria-label={t("musicApp.library.switch")}
                  selectionMode="single"
                  disallowEmptySelection
                  selectedKeys={[effectiveScope]}
                  onSelectionChange={(keys) => {
                    const next = [...keys][0];
                    if (next === "all" || next === "library") {
                      void navigate({ search: (prev) => ({ ...prev, scope: next }), replace: true });
                    }
                  }}
                  className="pv-segmented"
                >
                  <ToggleButton id="all">{t("musicApp.search.scopeAll")}</ToggleButton>
                  <ToggleButton id="library">{t("musicApp.search.scopeCurrent")}</ToggleButton>
                </ToggleButtonGroup>
              ) : null}
              {effectiveScope === "library" ? <MusicLibrarySwitcher /> : null}
            </form>

            {q ? (
              <>
                <ToggleButtonGroup
                  aria-label={t("musicApp.search.title")}
                  selectionMode="single"
                  disallowEmptySelection
                  selectedKeys={[tab]}
                  onSelectionChange={(keys) => {
                    const next = [...keys][0];
                    if (next === "all" || next === "songs" || next === "albums" || next === "artists") {
                      void navigate({ search: (prev) => ({ ...prev, tab: next }), replace: true });
                    }
                  }}
                  className="pv-segmented w-fit"
                >
                  <ToggleButton id="all">{t("musicApp.search.tabAll")}</ToggleButton>
                  <ToggleButton id="songs">{t("musicApp.search.tabSongs")}</ToggleButton>
                  <ToggleButton id="albums">{t("musicApp.search.tabAlbums")}</ToggleButton>
                  <ToggleButton id="artists">{t("musicApp.search.tabArtists")}</ToggleButton>
                </ToggleButtonGroup>
                <SearchResults q={q} scope={effectiveScope} tab={tab} libraryId={library.id} />
              </>
            ) : (
              <EmptyState icon={Search} title={t("musicApp.search.prompt")} />
            )}
          </div>
        )}
      </MusicLibraryGuard>
    </PageFrame>
  );
}
