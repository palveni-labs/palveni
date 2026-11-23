import { createFileRoute } from "@tanstack/react-router";
import { BookReaderPage } from "../../pages/book-reader-page";

export const Route = createFileRoute("/_app/reading/book/$itemId")({
  component: function BookReaderRoute() {
    const { itemId } = Route.useParams();
    return <BookReaderPage key={itemId} itemId={itemId} />;
  },
});
