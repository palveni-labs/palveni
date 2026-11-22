import { createFileRoute } from "@tanstack/react-router";
import { MusicArtistDetailPage } from "../../../pages/artist-detail-page";

export const Route = createFileRoute("/_app/music/artists/$libraryId/$artistId")({
  component: MusicArtistDetailPage,
});
