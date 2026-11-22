import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import type { Order, TrackSort } from "../api";
import { MusicSongsPage, type SongsSearchParams } from "../pages/songs-page";

const SORTS = new Set<TrackSort>(["title", "artist", "album", "year", "added", "duration", "plays", "played", "rating", "random"]);

export const Route = createFileRoute("/_app/music/songs")({
  validateSearch: (search: Record<string, unknown>): SongsSearchParams => {
    const sort = typeof search.sort === "string" && SORTS.has(search.sort as TrackSort) ? (search.sort as TrackSort) : undefined;
    const order: Order | undefined = search.order === "asc" || search.order === "desc" ? search.order : undefined;
    const genre = typeof search.genre === "string" && search.genre.trim() ? search.genre.trim() : undefined;
    const favorite = search.favorite === true || search.favorite === "true" ? true : undefined;
    const q = typeof search.q === "string" && search.q.trim() ? search.q.trim().slice(0, 128) : undefined;
    return { sort, order, genre, favorite, q };
  },
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicSongsPage,
});
