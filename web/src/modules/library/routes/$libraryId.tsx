import { createFileRoute } from "@tanstack/react-router";
import { itemAncestorsQuery, itemsInfiniteQuery } from "@/api/queries/items";
import { libraryQuery } from "@/api/queries/libraries";
import { LibraryBrowsePage } from "../browse/library-browse-page";

interface BrowseSearch {
  /** Folder being browsed; omitted for the library's top level. */
  parent?: string;
}

export const Route = createFileRoute("/_app/library/$libraryId")({
  validateSearch: (search: Record<string, unknown>): BrowseSearch => ({
    parent: typeof search.parent === "string" && search.parent !== "" ? search.parent : undefined,
  }),
  loaderDeps: ({ search }) => ({ parent: search.parent ?? null }),
  loader: async ({ context, params, deps }) => {
    const { queryClient } = context;
    await Promise.all([
      queryClient.ensureQueryData(libraryQuery(queryClient, params.libraryId)),
      queryClient.ensureInfiniteQueryData(itemsInfiniteQuery(params.libraryId, deps.parent)),
      deps.parent ? queryClient.ensureQueryData(itemAncestorsQuery(queryClient, deps.parent)) : null,
    ]);
  },
  component: LibraryBrowsePage,
});
