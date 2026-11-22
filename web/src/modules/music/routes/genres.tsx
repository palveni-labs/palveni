import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { MusicGenresPage } from "../pages/genres-page";

export const Route = createFileRoute("/_app/music/genres")({
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: MusicGenresPage,
});
