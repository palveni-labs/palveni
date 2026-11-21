import { createFileRoute } from "@tanstack/react-router";
import { AppearancePage } from "../themes-page";

/** 应用 › 外观 (the path keeps its historical name). */
export const Route = createFileRoute("/_app/apps/themes")({
  component: AppearancePage,
});
