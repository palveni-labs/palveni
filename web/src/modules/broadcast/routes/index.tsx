import { createFileRoute } from "@tanstack/react-router";
import { BroadcastRoute } from "../broadcast-page";
import type { BroadcastKind } from "../api";

export const Route = createFileRoute("/_app/broadcast/")({
  validateSearch: (search: Record<string, unknown>): { kind?: BroadcastKind } => ({
    kind:
      search.kind === "iptv" || search.kind === "podcast" || search.kind === "radio"
        ? search.kind
        : undefined,
  }),
  component: BroadcastRoute,
});
