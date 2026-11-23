import { createFileRoute } from "@tanstack/react-router";
import { VideoSearchPage, type VideoSearchParams } from "../pages/video-search-page";

export const Route = createFileRoute("/_app/video/search")({
  validateSearch: (search: Record<string, unknown>): VideoSearchParams => {
    const q = typeof search.q === "string" && search.q.trim() ? search.q.trim().slice(0, 128) : undefined;
    const libraryId = typeof search.libraryId === "string" && search.libraryId.trim() ? search.libraryId.trim() : undefined;
    const kind = search.kind === "movie" || search.kind === "series" ? search.kind : undefined;
    return { q, libraryId, kind };
  },
  component: VideoSearchPage,
});
