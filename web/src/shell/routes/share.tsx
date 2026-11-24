import { createFileRoute } from "@tanstack/react-router";
import { ShareViewerPage } from "../share-viewer-page";

export const Route = createFileRoute("/s/$token")({
  component: ShareViewerPage,
});
