import { createFileRoute } from "@tanstack/react-router";
import { MusicPlaylistDetailPage } from "../../pages/playlist-detail-page";

export const Route = createFileRoute("/_app/music/playlists/$playlistId")({
  component: MusicPlaylistDetailPage,
});
