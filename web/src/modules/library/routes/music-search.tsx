import { createFileRoute } from "@tanstack/react-router";
import type { SearchKind } from "../browse/music-search";
import { GlobalMusicSearch } from "../global-music-search";

export const Route = createFileRoute("/_app/library/music-search")({
  validateSearch: (search: Record<string, unknown>): { q?: string; kind?: SearchKind } => ({
    q: typeof search.q === "string" ? search.q.trim() || undefined
      : typeof search.q === "number" || typeof search.q === "boolean" ? String(search.q) : undefined,
    kind: search.kind === "artist" || search.kind === "album" ? search.kind : undefined,
  }),
  component: GlobalMusicSearch,
});
