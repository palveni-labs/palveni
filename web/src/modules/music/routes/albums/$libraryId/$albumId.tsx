import { createFileRoute } from "@tanstack/react-router";
import { MusicAlbumDetailPage } from "../../../pages/album-detail-page";

export const Route = createFileRoute("/_app/music/albums/$libraryId/$albumId")({
  component: MusicAlbumDetailPage,
});
