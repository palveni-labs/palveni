import { createFileRoute } from "@tanstack/react-router";
import { NotificationsPage } from "../notifications-page";

export const Route = createFileRoute("/_app/system/notifications")({
  component: NotificationsPage,
});
