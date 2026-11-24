import { createFileRoute } from "@tanstack/react-router";
import { PhotoCleanupPage } from "../pages/cleanup-page";

export const Route = createFileRoute("/_app/photos/cleanup")({
  component: PhotoCleanupPage,
});
