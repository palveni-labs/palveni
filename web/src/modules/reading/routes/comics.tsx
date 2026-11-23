import { createFileRoute } from "@tanstack/react-router";
import { ReadingShelfPage } from "../pages/reading-shelf-page";

export const Route = createFileRoute("/_app/reading/comics")({
  validateSearch: (raw: Record<string, unknown>): { library?: string; series?: string } => ({
    library: typeof raw.library === "string" && raw.library ? raw.library : undefined,
    series: typeof raw.series === "string" && raw.series ? raw.series : undefined,
  }),
  component: function ComicsShelfRoute() {
    const { library, series } = Route.useSearch();
    return (
      <ReadingShelfPage
        key={`comics:${library ?? ""}:${series ?? ""}`}
        mode="comics"
        initialLibraryId={library}
        initialSeries={series}
      />
    );
  },
});
