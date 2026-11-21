import { createFileRoute } from "@tanstack/react-router";
import { librariesQuery } from "@/api/queries/libraries";
import { OverviewPage } from "../overview-page";

export const Route = createFileRoute("/_app/")({
  loader: ({ context }) => {
    void context.queryClient.prefetchQuery(librariesQuery());
  },
  component: OverviewPage,
});
