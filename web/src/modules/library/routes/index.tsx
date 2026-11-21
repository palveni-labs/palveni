import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { LibrariesPage } from "../libraries-page";

export const Route = createFileRoute("/_app/library/")({
  loader: ({ context }) => context.queryClient.ensureQueryData(librariesQuery()),
  component: LibrariesPage,
});
