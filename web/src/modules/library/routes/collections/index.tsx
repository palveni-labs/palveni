import { createFileRoute } from "@tanstack/react-router";
import { CollectionsPage } from "../../collections-page";

export const Route = createFileRoute("/_app/library/collections/")({
  component: CollectionsPage,
});
