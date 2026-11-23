import { createFileRoute } from "@tanstack/react-router";
import { PhotoPlacesPage } from "../pages/places-page";

export interface PlacesSearch {
  place?: string;
}

export const Route = createFileRoute("/_app/photos/places")({
  validateSearch: (raw: Record<string, unknown>): PlacesSearch => ({
    place: typeof raw.place === "string" && raw.place.trim() ? raw.place.trim() : undefined,
  }),
  component: PhotoPlacesPage,
});
