import { createFileRoute } from "@tanstack/react-router";
import type { VideoOrder } from "../../api";
import { VideoLibraryPage, type VideoLibrarySearch, type VideoLibraryTab } from "../../pages/video-library-page";

const VALID_TABS: readonly VideoLibraryTab[] = ["all", "movie", "series", "genres"];
const VALID_ORDERS: readonly VideoOrder[] = ["added", "title", "year", "rating", "runtime", "poster", "random"];

function parseIntOpt(value: unknown): number | undefined {
  if (typeof value === "number" && Number.isInteger(value)) return value;
  if (typeof value === "string" && /^\d+$/.test(value)) return Number(value);
  return undefined;
}

export const Route = createFileRoute("/_app/video/$libraryId/")({
  validateSearch: (search: Record<string, unknown>): VideoLibrarySearch => {
    const tab = VALID_TABS.includes(search.tab as VideoLibraryTab) ? (search.tab as VideoLibraryTab) : undefined;
    const order = VALID_ORDERS.includes(search.order as VideoOrder) ? (search.order as VideoOrder) : undefined;
    const dir = search.dir === "asc" || search.dir === "desc" ? search.dir : undefined;
    const played = search.played === "played" || search.played === "unplayed" ? search.played : undefined;
    const favorite = search.favorite === true || search.favorite === "true" ? true : undefined;
    const q = typeof search.q === "string" && search.q.trim() ? search.q.trim().slice(0, 128) : undefined;
    const genre = typeof search.genre === "string" && search.genre.trim() ? search.genre.trim().slice(0, 64) : undefined;
    const startsWith =
      typeof search.startsWith === "string" && (/^[A-Z]$/.test(search.startsWith.toUpperCase()) || search.startsWith === "#")
        ? search.startsWith.toUpperCase()
        : undefined;
    const yearFrom = parseIntOpt(search.yearFrom);
    const yearTo = parseIntOpt(search.yearTo);
    const seed = parseIntOpt(search.seed);
    return { tab, order, dir, q, genre, yearFrom, yearTo, played, favorite, startsWith, seed };
  },
  component: VideoLibraryPage,
});
