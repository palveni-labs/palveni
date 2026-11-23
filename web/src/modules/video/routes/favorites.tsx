import { createFileRoute } from "@tanstack/react-router";
import { VideoFavoritesPage } from "../pages/video-favorites-page";

export const Route = createFileRoute("/_app/video/favorites")({
  component: VideoFavoritesPage,
});
