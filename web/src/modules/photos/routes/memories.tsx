import { createFileRoute } from "@tanstack/react-router";
import { PhotoMemoriesPage } from "../pages/memories-page";

export const Route = createFileRoute("/_app/photos/memories")({
  component: PhotoMemoriesPage,
});
