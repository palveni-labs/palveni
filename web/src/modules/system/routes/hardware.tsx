import { createFileRoute } from "@tanstack/react-router";
import { hardwareQuery } from "@/api/queries/system";
import { requireAdmin } from "@/shell/guards";
import { HardwarePage } from "../hardware-page";

export const Route = createFileRoute("/_app/system/hardware")({
  beforeLoad: ({ context }) => requireAdmin(context),
  loader: ({ context }) => context.queryClient.ensureQueryData(hardwareQuery()),
  component: HardwarePage,
});
