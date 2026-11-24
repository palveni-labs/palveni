import { createFileRoute } from "@tanstack/react-router";
import { SharesPage } from "../shares-page";

export const Route = createFileRoute("/_app/library/shares")({
  component: SharesPage,
});
