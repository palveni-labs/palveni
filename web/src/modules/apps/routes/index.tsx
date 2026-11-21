import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/apps/")({
  beforeLoad: () => {
    throw redirect({ to: "/apps/themes", replace: true });
  },
});
