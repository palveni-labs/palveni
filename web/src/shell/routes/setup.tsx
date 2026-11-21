import { createFileRoute, redirect } from "@tanstack/react-router";
import { SetupPage } from "../auth/setup-page";

export const Route = createFileRoute("/setup")({
  beforeLoad: ({ context }) => {
    // The wizard only exists while the server has no users.
    if (context.systemInfo.setupCompleted) throw redirect({ to: "/", replace: true });
  },
  component: SetupPage,
});
