import { createFileRoute } from "@tanstack/react-router";
import { PhotoPeoplePage } from "../pages/people-page";

export const Route = createFileRoute("/_app/photos/people")({
  component: PhotoPeoplePage,
});
