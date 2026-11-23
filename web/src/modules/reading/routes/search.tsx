import { createFileRoute } from "@tanstack/react-router";
import { ReadingShelfPage } from "../pages/reading-shelf-page";

export const Route = createFileRoute("/_app/reading/search")({
  validateSearch: (raw: Record<string, unknown>): { q?: string } => ({
    q: typeof raw.q === "string" && raw.q ? raw.q : undefined,
  }),
  component: function ReadingSearchRoute() {
    const { q } = Route.useSearch();
    return <ReadingShelfPage key={`search:${q ?? ""}`} mode="search" initialQuery={q} />;
  },
});
