import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { PhotosTimelinePage } from "../pages/timeline-page";

export interface PhotosTimelineSearch {
  libraryId?: string;
  camera?: string;
  month?: string;
  q?: string;
  grid?: "square" | "aspect";
}

export const Route = createFileRoute("/_app/photos/")({
  validateSearch: (raw: Record<string, unknown>): PhotosTimelineSearch => ({
    libraryId: typeof raw.libraryId === "string" && raw.libraryId.trim() ? raw.libraryId.trim() : undefined,
    camera: typeof raw.camera === "string" && raw.camera.trim() ? raw.camera.trim() : undefined,
    month: typeof raw.month === "string" && raw.month.trim() ? raw.month.trim() : undefined,
    q: typeof raw.q === "string" && raw.q.trim() ? raw.q.trim() : undefined,
    grid: raw.grid === "aspect" ? "aspect" : raw.grid === "square" ? "square" : undefined,
  }),
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: PhotosTimelinePage,
});
