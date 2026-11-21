import { createFileRoute } from "@tanstack/react-router";
import { itemAncestorsQuery, itemsInfiniteQuery } from "@/api/queries/items";
import { libraryQuery } from "@/api/queries/libraries";
import { LibraryBrowsePage } from "../browse/library-browse-page";

interface BrowseSearch {
  /** Folder being browsed; omitted for the library's top level. */
  parent?: string;
  view?: "music";
  artist?: string;
  album?: string;
  artistName?: string;
  albumName?: string;
}

export const Route = createFileRoute("/_app/library/$libraryId")({
  validateSearch: (search: Record<string, unknown>): BrowseSearch => ({
    view: search.view === "music" ? "music" : undefined,
    artist: typeof search.artist === "string" ? search.artist : undefined,
    album: typeof search.album === "string" && typeof search.artist === "string" ? search.album : undefined,
    artistName: typeof search.artistName === "string" ? search.artistName.slice(0, 1024) : undefined,
    albumName: typeof search.albumName === "string" ? search.albumName.slice(0, 1024) : undefined,
    parent: typeof search.parent === "string" && search.parent !== "" ? search.parent : undefined,
  }),
  loaderDeps: ({ search }) => ({ parent: search.parent ?? null, view: search.view }),
  loader: async ({ context, params, deps }) => {
    const { queryClient } = context;
    await Promise.all([
      queryClient.ensureQueryData(libraryQuery(queryClient, params.libraryId)),
      deps.view === "music" ? null : queryClient.ensureInfiniteQueryData(itemsInfiniteQuery(params.libraryId, deps.parent)),
      deps.parent ? queryClient.ensureQueryData(itemAncestorsQuery(queryClient, deps.parent)) : null,
    ]);
  },
  component: LibraryBrowsePage,
});
