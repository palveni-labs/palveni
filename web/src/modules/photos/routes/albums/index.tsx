import { createFileRoute } from "@tanstack/react-router";
import { PhotoAlbumsPage } from "../../pages/albums-page";

export const Route = createFileRoute("/_app/photos/albums/")({
  component: PhotoAlbumsPage,
});
