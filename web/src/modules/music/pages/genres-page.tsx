import { Input, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Disc3, LayoutGrid, List, Music2, Play, Tag } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { hashString } from "@/lib/format";
import { readLocal, writeLocal } from "@/lib/storage";
import { playerStore } from "@/player/store";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { musicApi, type MusicGenre } from "../api";
import { MusicLibraryGuard, MusicLibrarySwitcher } from "../library-context";
import { musicKeys } from "../queries";

const VIEW_KEY = "palveni.music.genres.view";

/** Editorial non-purple palette ( §3 forbids purple hues). */
const GENRE_SWATCHES = [
  "#8c2f24",
  "#9e4d23",
  "#8a641b",
  "#35613a",
  "#1e5a54",
  "#234f73",
  "#2f4261",
  "#5a4834",
  "#783b2c",
  "#2c5d4c",
] as const;

function swatchFor(name: string): string {
  return GENRE_SWATCHES[hashString(name) % GENRE_SWATCHES.length]!;
}

function GenreCard({ libraryId, genre }: { libraryId: string; genre: MusicGenre }) {
  const { t } = useTranslation();
  const [loadingPlay, setLoadingPlay] = useState(false);

  const playGenre = async () => {
    if (loadingPlay) return;
    setLoadingPlay(true);
    try {
      const { tracks } = await musicApi.tracks(libraryId, { genre: genre.name, sort: "random", seed: Date.now() % 10000, limit: 100 });
      if (tracks.length > 0) playerStore.playQueue(tracks, 0, { shuffle: false });
    } finally {
      setLoadingPlay(false);
    }
  };

  return (
    <article
      style={{ backgroundColor: swatchFor(genre.name) }}
      className="group relative flex min-h-34 flex-col justify-between overflow-hidden rounded-panel p-4 text-white shadow-panel transition-transform duration-200 hover:-translate-y-0.5"
    >
      <div className="flex items-start justify-between gap-2">
        <Link
          to="/music/albums"
          search={{ genre: genre.name }}
          className="font-heading text-xl font-title text-white hover:underline focus-visible:outline-2 focus-visible:outline-white"
        >
          {genre.name}
        </Link>
        <button
          type="button"
          disabled={loadingPlay}
          onClick={() => void playGenre()}
          aria-label={`${t("musicApp.genres.playGenre")} ${genre.name}`}
          className="grid size-9 shrink-0 place-items-center rounded-full bg-white/95 text-[#121316] opacity-90 shadow-raised transition-all group-hover:opacity-100 hover:scale-105 active:scale-95 focus-visible:outline-2 focus-visible:outline-white"
        >
          <Play className="ms-0.5 size-4 fill-current" aria-hidden />
        </button>
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-2 text-xs font-mid">
        <Link
          to="/music/albums"
          search={{ genre: genre.name }}
          className="inline-flex items-center gap-1 rounded-full bg-black/28 px-2.5 py-1 text-white transition-colors hover:bg-black/45"
        >
          <Disc3 className="size-3.5" aria-hidden />
          <span data-num className="tabular-nums">{t("musicApp.albums.count", { count: genre.albumCount })}</span>
        </Link>
        <Link
          to="/music/songs"
          search={{ genre: genre.name }}
          className="inline-flex items-center gap-1 rounded-full bg-black/28 px-2.5 py-1 text-white transition-colors hover:bg-black/45"
        >
          <Music2 className="size-3.5" aria-hidden />
          <span data-num className="tabular-nums">{t("musicApp.songs.count", { count: genre.trackCount })}</span>
        </Link>
      </div>
    </article>
  );
}

function GenresContent({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const [q, setQ] = useState("");
  const [view, setView] = useState<"grid" | "list">(() => (readLocal(VIEW_KEY) === "list" ? "list" : "grid"));

  const genresQuery = useQuery({
    queryKey: musicKeys.genres(libraryId),
    queryFn: ({ signal }) => musicApi.genres(libraryId, signal),
  });

  const needle = q.trim().toLowerCase();
  const genres = (genresQuery.data?.genres ?? []).filter((g) => !needle || g.name.toLowerCase().includes(needle));

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          value={q}
          onChange={(event) => setQ(event.target.value)}
          aria-label={t("musicApp.genres.searchPlaceholder")}
          placeholder={t("musicApp.genres.searchPlaceholder")}
          className="h-(--pv-control-h-sm) w-52 max-sm:w-full"
        />
        <span className="flex-1" />
        <span data-num className="text-xs text-fg-muted tabular-nums max-sm:hidden">
          {t("musicApp.genres.count", { count: genres.length })}
        </span>
        <ToggleButtonGroup
          aria-label={t("browse.view")}
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={[view]}
          onSelectionChange={(keys) => {
            const next = [...keys][0];
            if (next === "grid" || next === "list") {
              setView(next);
              writeLocal(VIEW_KEY, next);
            }
          }}
          className="pv-segmented"
        >
          <ToggleButton id="grid" aria-label={t("musicApp.actions.viewGrid")}>
            <LayoutGrid aria-hidden />
          </ToggleButton>
          <ToggleButton id="list" aria-label={t("musicApp.actions.viewList")}>
            <List aria-hidden />
          </ToggleButton>
        </ToggleButtonGroup>
        <MusicLibrarySwitcher />
      </div>

      {genresQuery.isPending ? (
        <LoadingRows rows={6} />
      ) : genresQuery.isError ? (
        <ErrorState error={genresQuery.error} onRetry={() => void genresQuery.refetch()} />
      ) : genres.length === 0 ? (
        <EmptyState icon={Tag} title={t("musicApp.genres.empty")} />
      ) : view === "grid" ? (
        <div className="pv-grid-card">
          {genres.map((genre) => (
            <GenreCard key={genre.name} libraryId={libraryId} genre={genre} />
          ))}
        </div>
      ) : (
        <ul className="divide-y divide-line overflow-hidden rounded-panel border border-line bg-surface">
          {genres.map((genre) => (
            <li key={genre.name} className="flex flex-wrap items-center gap-3 px-4 py-3 hover:bg-hover">
              <span className="size-3 shrink-0 rounded-full" style={{ backgroundColor: swatchFor(genre.name) }} aria-hidden />
              <Link to="/music/albums" search={{ genre: genre.name }} className="min-w-0 flex-1 truncate font-strong text-fg hover:underline">
                {genre.name}
              </Link>
              <Link to="/music/albums" search={{ genre: genre.name }} className="text-sm text-fg-muted hover:text-fg hover:underline tabular-nums">
                {t("musicApp.albums.count", { count: genre.albumCount })}
              </Link>
              <Link to="/music/songs" search={{ genre: genre.name }} className="text-sm text-fg-muted hover:text-fg hover:underline tabular-nums">
                {t("musicApp.songs.count", { count: genre.trackCount })}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function MusicGenresPage() {
  const { t } = useTranslation();
  return (
    <PageFrame>
      <PageHeader title={t("musicApp.genres.title")} />
      <MusicLibraryGuard>{(library) => <GenresContent libraryId={library.id} />}</MusicLibraryGuard>
    </PageFrame>
  );
}
