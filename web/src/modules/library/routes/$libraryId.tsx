import { createFileRoute } from "@tanstack/react-router";
import { itemAncestorsQuery, itemsInfiniteQuery } from "@/api/queries/items";
import { libraryQuery } from "@/api/queries/libraries";
import { LibraryBrowsePage } from "../browse/library-browse-page";

interface BrowseSearch {
  /** Folder being browsed; omitted for the library's top level. */
  parent?: string;
  view?: "music" | "works" | "spoken" | "files";
  order?: "title";
  collection?: string;
  collectionName?: string;
  series?: string;
  season?: number;
  seriesName?: string;
  artist?: string;
  album?: string;
  artistName?: string;
  albumName?: string;
}

export const Route = createFileRoute("/_app/library/$libraryId")({
  validateSearch: (search: Record<string, unknown>): BrowseSearch => ({
    view: search.view === "music" || search.view === "works" || search.view === "spoken" || search.view === "files" ? search.view : undefined,
    order: search.order === "title" ? "title" : undefined,
    collection: typeof search.collection === "string" ? search.collection : undefined,
    collectionName: typeof search.collectionName === "string" ? search.collectionName.slice(0, 1024) : undefined,
    series: typeof search.series === "string" && /^[0-9a-f]{64}$/.test(search.series) ? search.series : undefined,
    seriesName: typeof search.seriesName === "string" ? search.seriesName.slice(0, 1024) : undefined,
    season: typeof search.series === "string" && /^[0-9a-f]{64}$/.test(search.series) && typeof search.season === "number" && Number.isInteger(search.season) && search.season >= 0 && search.season <= 65535 ? search.season : undefined,
    artist: typeof search.artist === "string" ? search.artist : undefined,
    album: typeof search.album === "string" && typeof search.artist === "string" ? search.album : undefined,
    artistName: typeof search.artistName === "string" ? search.artistName.slice(0, 1024) : undefined,
    albumName: typeof search.albumName === "string" ? search.albumName.slice(0, 1024) : undefined,
    parent: typeof search.parent === "string" && search.parent !== "" ? search.parent : undefined,
  }),
  loaderDeps: ({ search }) => ({ parent: search.parent ?? null, view: search.view }),
  loader: async ({ context, params, deps }) => {
    const { queryClient } = context;
    const library = await queryClient.ensureQueryData(libraryQuery(queryClient, params.libraryId));
    const fileView = deps.view === "files" || !!deps.parent || (!deps.view && !["video", "audiobook", "podcast"].includes(library.kind));
    await Promise.all([
      fileView ? queryClient.ensureInfiniteQueryData(itemsInfiniteQuery(params.libraryId, deps.parent)) : null,
      deps.parent ? queryClient.ensureQueryData(itemAncestorsQuery(queryClient, deps.parent)) : null,
    ]);
  },
  component: LibraryBrowsePage,
});
