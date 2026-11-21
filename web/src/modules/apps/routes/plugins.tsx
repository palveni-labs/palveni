import { createFileRoute } from "@tanstack/react-router";
import { requireAdmin } from "@/shell/guards";
import { PluginsPage } from "../plugins-page";

export const Route = createFileRoute("/_app/apps/plugins")({
  beforeLoad: ({ context }) => requireAdmin(context),
  component: PluginsPage,
});
