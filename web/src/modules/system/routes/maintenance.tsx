import { createFileRoute } from "@tanstack/react-router";
import { requireAdmin } from "@/shell/guards";
import { MaintenancePage } from "../maintenance-page";

export const Route = createFileRoute("/_app/system/maintenance")({
  beforeLoad: ({ context }) => requireAdmin(context),
  component: MaintenancePage,
});
