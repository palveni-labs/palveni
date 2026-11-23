import { createFileRoute } from "@tanstack/react-router";
import { ComicReaderPage } from "../../pages/comic-reader-page";

export const Route = createFileRoute("/_app/reading/comic/$itemId")({
  component: function ComicReaderRoute() {
    const { itemId } = Route.useParams();
    return <ComicReaderPage key={itemId} itemId={itemId} />;
  },
});
