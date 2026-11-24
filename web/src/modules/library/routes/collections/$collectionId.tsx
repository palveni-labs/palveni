import { createFileRoute } from "@tanstack/react-router";
import { CollectionDetailPage } from "../../collection-detail-page";

export const Route = createFileRoute("/_app/library/collections/$collectionId")({
  component: CollectionDetailPage,
});
