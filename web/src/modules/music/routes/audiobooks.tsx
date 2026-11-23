import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { MusicAudiobooksPage } from "../pages/audiobooks-page";

export interface AudiobooksSearch {
  library?: string;
  collection?: string;
  title?: string;
}

export const Route = createFileRoute("/_app/music/audiobooks")({
  validateSearch: (search: Record<string, unknown>): AudiobooksSearch => ({
    library: typeof search.library === "string" && search.library ? search.library : undefined,
    collection: typeof search.collection === "string" && search.collection ? search.collection : undefined,
    title: typeof search.title === "string" && search.title ? search.title : undefined,
  }),
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicAudiobooksPage,
});
