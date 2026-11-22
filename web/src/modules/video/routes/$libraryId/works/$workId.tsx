import { createFileRoute } from "@tanstack/react-router";
import { VideoDetailPage } from "../../../pages/video-detail-page";

interface WorkDetailSearch {
  season?: number;
}

export const Route = createFileRoute("/_app/video/$libraryId/works/$workId")({
  validateSearch: (search: Record<string, unknown>): WorkDetailSearch => {
    const raw = typeof search.season === "number" ? search.season : typeof search.season === "string" && /^\d+$/.test(search.season) ? Number(search.season) : undefined;
    return {
      season: raw !== undefined && Number.isInteger(raw) && raw >= 0 && raw <= 65535 ? raw : undefined,
    };
  },
  component: VideoDetailPage,
});
