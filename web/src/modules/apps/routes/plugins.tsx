import { createFileRoute } from "@tanstack/react-router";
import { pluginsQuery } from "@/api/queries/plugins";
import { requireAdmin } from "@/shell/guards";
import { PluginsPage } from "../plugins-page";

export const Route = createFileRoute("/_app/apps/plugins")({
  beforeLoad: ({ context }) => requireAdmin(context),
  loader: ({ context }) => context.queryClient.ensureQueryData(pluginsQuery()),
  component: PluginsPage,
});
