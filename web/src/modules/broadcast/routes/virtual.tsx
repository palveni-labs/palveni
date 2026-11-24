import { createFileRoute } from "@tanstack/react-router";
import { VirtualTvPage } from "../virtual-tv-page";

export const Route = createFileRoute("/_app/broadcast/virtual")({
  component: VirtualTvPage,
});
