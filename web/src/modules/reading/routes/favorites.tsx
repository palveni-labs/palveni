import { createFileRoute } from "@tanstack/react-router";
import { ReadingShelfPage } from "../pages/reading-shelf-page";

export const Route = createFileRoute("/_app/reading/favorites")({
  component: function ReadingFavoritesRoute() {
    return <ReadingShelfPage mode="favorites" />;
  },
});
