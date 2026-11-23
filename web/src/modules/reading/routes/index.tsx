import { createFileRoute } from "@tanstack/react-router";
import { ReadingHomePage } from "../pages/reading-home-page";

export const Route = createFileRoute("/_app/reading/")({
  validateSearch: (raw: Record<string, unknown>): { library?: string } => ({
    library: typeof raw.library === "string" && raw.library ? raw.library : undefined,
  }),
  component: function ReadingHomeRoute() {
    const { library } = Route.useSearch();
    return <ReadingHomePage libraryId={library} />;
  },
});
