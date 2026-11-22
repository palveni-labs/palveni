import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import type { AlbumSort, Order } from "../../api";
import { MusicAlbumsPage, type AlbumsSearchParams } from "../../pages/albums-page";

const SORTS = new Set<AlbumSort>(["title", "artist", "year", "added", "played", "plays", "rating", "random"]);

export const Route = createFileRoute("/_app/music/albums/")({
  validateSearch: (search: Record<string, unknown>): AlbumsSearchParams => {
    const sort = typeof search.sort === "string" && SORTS.has(search.sort as AlbumSort) ? (search.sort as AlbumSort) : undefined;
    const order: Order | undefined = search.order === "asc" || search.order === "desc" ? search.order : undefined;
    const genre = typeof search.genre === "string" && search.genre.trim() ? search.genre.trim() : undefined;
    const yearFrom = typeof search.yearFrom === "number" && Number.isInteger(search.yearFrom) ? search.yearFrom : typeof search.yearFrom === "string" && /^\d{4}$/.test(search.yearFrom) ? Number(search.yearFrom) : undefined;
    const yearTo = typeof search.yearTo === "number" && Number.isInteger(search.yearTo) ? search.yearTo : typeof search.yearTo === "string" && /^\d{4}$/.test(search.yearTo) ? Number(search.yearTo) : undefined;
    const favorite = search.favorite === true || search.favorite === "true" ? true : undefined;
    const q = typeof search.q === "string" && search.q.trim() ? search.q.trim().slice(0, 128) : undefined;
    return { sort, order, genre, yearFrom, yearTo, favorite, q };
  },
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicAlbumsPage,
});
