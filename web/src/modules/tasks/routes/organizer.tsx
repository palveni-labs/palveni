import { createFileRoute } from "@tanstack/react-router";
import { OrganizerPage } from "../organizer-page";

export const Route = createFileRoute("/_app/tasks/organizer")({
  component: OrganizerPage,
});
