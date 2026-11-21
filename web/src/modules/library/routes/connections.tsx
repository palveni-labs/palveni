import { createFileRoute } from "@tanstack/react-router";
import { ConnectionsPage } from "../connections-page";

export const Route = createFileRoute("/_app/library/connections")({
  component: ConnectionsPage,
});
