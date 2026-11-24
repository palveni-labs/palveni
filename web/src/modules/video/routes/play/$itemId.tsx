import { createFileRoute } from "@tanstack/react-router";
import { VideoPlayerPage, type VideoPlaySearch } from "../../pages/video-player-page";

function parseNonNegInt(value: unknown): number | undefined {
  const num = typeof value === "number" ? value : typeof value === "string" && /^\d+$/.test(value) ? Number(value) : undefined;
  return num !== undefined && Number.isInteger(num) && num >= 0 ? num : undefined;
}

export const Route = createFileRoute("/_app/video/play/$itemId")({
  validateSearch: (search: Record<string, unknown>): VideoPlaySearch => ({
    libraryId: typeof search.libraryId === "string" && search.libraryId ? search.libraryId : undefined,
    workId: typeof search.workId === "string" && search.workId ? search.workId : undefined,
    season: parseNonNegInt(search.season),
    startMs: parseNonNegInt(search.startMs),
    fromStart: search.fromStart === true || search.fromStart === "true" ? true : undefined,
    resumeDirect: search.resumeDirect === true || search.resumeDirect === "true" ? true : undefined,
    title: typeof search.title === "string" && search.title ? search.title.slice(0, 256) : undefined,
  }),
  component: VideoPlayerPage,
});
