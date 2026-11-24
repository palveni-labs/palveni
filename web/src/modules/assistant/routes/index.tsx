import { createFileRoute } from "@tanstack/react-router";
import { AssistantPage } from "../assistant-page";

export const Route = createFileRoute("/_app/assistant/")({
  component: AssistantPage,
});
