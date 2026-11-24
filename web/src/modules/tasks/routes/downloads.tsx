import { createFileRoute } from "@tanstack/react-router";
import { DownloadsPage } from "../downloads-page";

export const Route = createFileRoute("/_app/tasks/downloads")({
  component: DownloadsPage,
});
