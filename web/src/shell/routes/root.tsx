import { createRootRouteWithContext, redirect } from "@tanstack/react-router";
import { systemInfoQuery } from "@/api/queries/system";
import type { RouterContext } from "@/router";
import { BootError, RootLayout, Splash } from "../root-layout";

export const Route = createRootRouteWithContext<RouterContext>()({
  beforeLoad: async ({ context, location }) => {
    const info = await context.queryClient.ensureQueryData(systemInfoQuery());
    const onSetup = location.pathname === "/setup";
    if (!info.setupCompleted && !onSetup) throw redirect({ to: "/setup", replace: true });
    return { systemInfo: info };
  },
  component: RootLayout,
  pendingComponent: Splash,
  errorComponent: BootError,
});
