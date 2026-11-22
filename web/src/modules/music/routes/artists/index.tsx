import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import type { ArtistSort, Order } from "../../api";
import { MusicArtistsPage, type ArtistsSearchParams } from "../../pages/artists-page";

const SORTS = new Set<ArtistSort>(["name", "albums", "tracks", "plays", "random"]);

export const Route = createFileRoute("/_app/music/artists/")({
  validateSearch: (search: Record<string, unknown>): ArtistsSearchParams => {
    const sort = typeof search.sort === "string" && SORTS.has(search.sort as ArtistSort) ? (search.sort as ArtistSort) : undefined;
    const order: Order | undefined = search.order === "asc" || search.order === "desc" ? search.order : undefined;
    const favorite = search.favorite === true || search.favorite === "true" ? true : undefined;
    const q = typeof search.q === "string" && search.q.trim() ? search.q.trim().slice(0, 128) : undefined;
    return { sort, order, favorite, q };
  },
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicArtistsPage,
});
