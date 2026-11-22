import { createFileRoute } from "@tanstack/react-router";
import { VideoHomePage } from "../pages/video-home-page";

export const Route = createFileRoute("/_app/video/")({
  component: VideoHomePage,
});
