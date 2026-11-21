import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/system/")({
  beforeLoad: ({ context }) => {
    throw redirect({ to: context.user.role === "admin" ? "/system/libraries" : "/system/settings", replace: true });
  },
});
