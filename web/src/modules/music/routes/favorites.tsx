import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { MusicFavoritesPage, type FavoritesTab } from "../pages/favorites-page";

export const Route = createFileRoute("/_app/music/favorites")({
  validateSearch: (search: Record<string, unknown>): { tab?: FavoritesTab } => ({
    tab: search.tab === "albums" || search.tab === "artists" || search.tab === "songs" ? search.tab : undefined,
  }),
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicFavoritesPage,
});
