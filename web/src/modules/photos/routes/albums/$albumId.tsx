import { createFileRoute } from "@tanstack/react-router";
import { PhotoAlbumDetailPage } from "../../pages/album-detail-page";

export const Route = createFileRoute("/_app/photos/albums/$albumId")({
  component: PhotoAlbumDetailPage,
});
