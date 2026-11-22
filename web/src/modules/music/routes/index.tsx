import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { MusicHomePage } from "../pages/home-page";

interface MusicHomeSearch {
  library?: string;
}

export const Route = createFileRoute("/_app/music/")({
  validateSearch: (search: Record<string, unknown>): MusicHomeSearch => ({
    library: typeof search.library === "string" && search.library.trim() ? search.library.trim() : undefined,
  }),
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicHomePage,
});
