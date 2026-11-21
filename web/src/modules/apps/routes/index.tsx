import { createFileRoute, redirect } from "@tanstack/react-router";
import { systemInfoQuery } from "@/api/queries/system";

export const Route = createFileRoute("/_app/apps/")({
  beforeLoad: async ({ context }) => {
    const canManage = context.user.role === "admin" && (await context.queryClient.ensureQueryData(systemInfoQuery())).features.includes("plugins");
    throw redirect({ to: canManage ? "/apps/plugins" : "/apps/themes", replace: true });
  },
});
