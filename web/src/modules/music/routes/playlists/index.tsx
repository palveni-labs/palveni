import { createFileRoute } from "@tanstack/react-router";
import { MusicPlaylistsPage } from "../../pages/playlists-page";

export const Route = createFileRoute("/_app/music/playlists/")({
  component: MusicPlaylistsPage,
});
