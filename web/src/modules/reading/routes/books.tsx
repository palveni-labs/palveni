import { createFileRoute } from "@tanstack/react-router";
import { ReadingShelfPage } from "../pages/reading-shelf-page";

export const Route = createFileRoute("/_app/reading/books")({
  validateSearch: (raw: Record<string, unknown>): { library?: string; series?: string } => ({
    library: typeof raw.library === "string" && raw.library ? raw.library : undefined,
    series: typeof raw.series === "string" && raw.series ? raw.series : undefined,
  }),
  component: function BooksShelfRoute() {
    const { library, series } = Route.useSearch();
    return (
      <ReadingShelfPage
        key={`books:${library ?? ""}:${series ?? ""}`}
        mode="books"
        initialLibraryId={library}
        initialSeries={series}
      />
    );
  },
});
