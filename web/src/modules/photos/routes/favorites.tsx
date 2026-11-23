import { createFileRoute } from "@tanstack/react-router";
import { PhotoFavoritesPage } from "../pages/favorites-page";

export const Route = createFileRoute("/_app/photos/favorites")({
  component: PhotoFavoritesPage,
});
