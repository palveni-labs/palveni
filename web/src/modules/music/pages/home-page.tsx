import { Button } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { ChevronRight, RefreshCw, Search, Shuffle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { playerStore } from "@/player/store";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { ErrorState, LoadingRows } from "@/ui/states";
import { musicApi, type AlbumSort, type MusicAlbum, type Order } from "../api";
import { AlbumCard } from "../components/album-card";
import { MusicLibraryGuard, MusicLibrarySwitcher } from "../library-context";
import { setSelectedMusicLibrary } from "../library-store";
import { musicKeys } from "../queries";

function AlbumShelf({
  title,
  albums,
  isPending,
  isError,
  error,
  onRetry,
  seeAllSearch,
  extraAction,
}: {
  title: string;
  albums: MusicAlbum[];
  isPending: boolean;
  isError: boolean;
  error: unknown;
  onRetry: () => void;
  seeAllSearch?: { sort: AlbumSort; order?: Order };
  extraAction?: React.ReactNode;
}) {
  const { t } = useTranslation();

  if (!isPending && !isError && albums.length === 0) return null;

  return (
    <section aria-label={title} className="grid min-w-0 gap-4">
      <header className="flex items-center gap-2">
        <h2 className="font-heading text-xl font-title text-heading">{title}</h2>
        <span className="flex-1" />
        {extraAction}
        {seeAllSearch ? (
          <Link
            to="/music/albums"
            search={seeAllSearch}
            className="inline-flex items-center gap-0.5 rounded-control px-2.5 py-1 text-sm text-link transition-colors hover:bg-hover hover:text-fg"
          >
            {t("musicApp.home.seeAll")}
            <ChevronRight className="size-3.5" aria-hidden />
          </Link>
        ) : null}
      </header>

      {isPending ? (
        <LoadingRows rows={2} />
      ) : isError ? (
        <ErrorState compact error={error} onRetry={onRetry} />
      ) : (
        <div className="no-scrollbar -mx-1 flex gap-4 overflow-x-auto px-1 pb-2 sm:mx-0 sm:overflow-visible sm:px-0 sm:pb-0 sm:grid sm:grid-cols-[repeat(auto-fill,minmax(min(100%,clamp(9.75rem,14.5cqi,12.5rem)),1fr))] sm:gap-(--pv-grid-gap,1.125rem)">
          {albums.map((album) => (
            <AlbumCard key={album.id} album={album} className="w-40 shrink-0 sm:w-auto sm:shrink" />
          ))}
        </div>
      )}
    </section>
  );
}

function HomeContent({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const [randomSeed, setRandomSeed] = useState(1);
  const [shufflingAll, setShufflingAll] = useState(false);

  const recentAdded = useQuery({
    queryKey: musicKeys.albums(libraryId, { sort: "added", order: "desc", limit: 12 }),
    queryFn: ({ signal }) => musicApi.albums(libraryId, { sort: "added", order: "desc", limit: 12 }, signal),
  });
  const recentPlayed = useQuery({
    queryKey: musicKeys.albums(libraryId, { sort: "played", order: "desc", limit: 12 }),
    queryFn: ({ signal }) => musicApi.albums(libraryId, { sort: "played", order: "desc", limit: 12 }, signal),
  });
  const mostPlayed = useQuery({
    queryKey: musicKeys.albums(libraryId, { sort: "plays", order: "desc", limit: 12 }),
    queryFn: ({ signal }) => musicApi.albums(libraryId, { sort: "plays", order: "desc", limit: 12 }, signal),
  });
  const randomAlbums = useQuery({
    queryKey: musicKeys.albums(libraryId, { sort: "random", seed: randomSeed, limit: 12 }),
    queryFn: ({ signal }) => musicApi.albums(libraryId, { sort: "random", seed: randomSeed, limit: 12 }, signal),
  });

  const handleShuffleAll = async () => {
    if (shufflingAll) return;
    setShufflingAll(true);
    try {
      const seed = Math.floor(Math.random() * 100_000) + 1;
      const { tracks } = await musicApi.tracks(libraryId, { sort: "random", seed, limit: 100 });
      if (tracks.length > 0) playerStore.playShuffled(tracks);
    } finally {
      setShufflingAll(false);
    }
  };

  return (
    <div className="grid gap-9">
      <div className="pv-glass-panel flex flex-wrap items-center justify-between gap-3 rounded-panel border px-4 py-3">
        <div className="flex flex-wrap items-center gap-2.5">
          <Button size="sm" variant="primary" isDisabled={shufflingAll} onPress={() => void handleShuffleAll()}>
            <Shuffle aria-hidden />
            {t("musicApp.home.shuffleAll")}
          </Button>
          <Link
            to="/music/search"
            className="inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border border-line/80 bg-sunken/70 px-3 text-sm font-mid text-fg-muted transition-colors hover:border-line-strong hover:bg-hover hover:text-fg"
          >
            <Search className="size-3.5" aria-hidden />
            {t("musicApp.search.placeholder")}
          </Link>
        </div>
        <MusicLibrarySwitcher />
      </div>

      <AlbumShelf
        title={t("musicApp.home.recentlyAdded")}
        albums={recentAdded.data?.albums.slice(0, 6) ?? []}
        isPending={recentAdded.isPending}
        isError={recentAdded.isError}
        error={recentAdded.error}
        onRetry={() => void recentAdded.refetch()}
        seeAllSearch={{ sort: "added", order: "desc" }}
      />

      <AlbumShelf
        title={t("musicApp.home.recentlyPlayed")}
        albums={(recentPlayed.data?.albums ?? []).filter((a) => a.lastPlayedAt !== null).slice(0, 6)}
        isPending={recentPlayed.isPending}
        isError={recentPlayed.isError}
        error={recentPlayed.error}
        onRetry={() => void recentPlayed.refetch()}
        seeAllSearch={{ sort: "played", order: "desc" }}
      />

      <AlbumShelf
        title={t("musicApp.home.mostPlayed")}
        albums={(mostPlayed.data?.albums ?? []).filter((a) => a.playCount > 0).slice(0, 6)}
        isPending={mostPlayed.isPending}
        isError={mostPlayed.isError}
        error={mostPlayed.error}
        onRetry={() => void mostPlayed.refetch()}
        seeAllSearch={{ sort: "plays", order: "desc" }}
      />

      <AlbumShelf
        title={t("musicApp.home.randomAlbums")}
        albums={randomAlbums.data?.albums.slice(0, 6) ?? []}
        isPending={randomAlbums.isPending}
        isError={randomAlbums.isError}
        error={randomAlbums.error}
        onRetry={() => void randomAlbums.refetch()}
        seeAllSearch={{ sort: "random" }}
        extraAction={
          <Button size="sm" variant="ghost" onPress={() => setRandomSeed((s) => s + 1)}>
            <RefreshCw className="size-3.5" aria-hidden />
            {t("musicApp.home.refreshRandom")}
          </Button>
        }
      />
    </div>
  );
}

export function MusicHomePage() {
  const { t } = useTranslation();
  const { library: initialLibraryId } = useSearch({ from: "/_app/music/" });

  useEffect(() => {
    if (initialLibraryId) setSelectedMusicLibrary(initialLibraryId);
  }, [initialLibraryId]);

  return (
    <PageFrame>
      <PageHeader title={t("musicApp.title")} />
      <MusicLibraryGuard>{(library) => <HomeContent libraryId={library.id} />}</MusicLibraryGuard>
    </PageFrame>
  );
}
