import { createFileRoute } from "@tanstack/react-router";
import { HomePage } from "../home-page";

interface HomeSearch {
  /** Shows one room's devices. */
  room?: string;
}

export const Route = createFileRoute("/_app/home/")({
  validateSearch: (search: Record<string, unknown>): HomeSearch => ({
    room: typeof search.room === "string" && search.room ? search.room : undefined,
  }),
  component: HomePage,
});
