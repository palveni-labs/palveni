import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { MusicSearchPage, type SearchScope, type SearchTab } from "../pages/search-page";

export const Route = createFileRoute("/_app/music/search")({
  validateSearch: (search: Record<string, unknown>): { q?: string; scope?: SearchScope; tab?: SearchTab } => ({
    q: typeof search.q === "string" && search.q.trim() ? search.q.trim().slice(0, 128) : undefined,
    scope: search.scope === "all" || search.scope === "library" ? search.scope : undefined,
    tab: search.tab === "all" || search.tab === "songs" || search.tab === "albums" || search.tab === "artists" ? search.tab : undefined,
  }),
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicSearchPage,
});
