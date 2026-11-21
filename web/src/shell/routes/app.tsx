import { createFileRoute, redirect } from "@tanstack/react-router";
import { ApiError } from "@/api/errors";
import { meQuery } from "@/api/queries/auth";
import { clearSession, getToken } from "@/api/session";
import { AppShell } from "../app-shell";
import { safeRedirect } from "../auth/redirect";

/** Authenticated area: a valid token and a loaded user, otherwise the login page. */
export const Route = createFileRoute("/_app")({
  beforeLoad: async ({ context, location }) => {
    const toLogin = () =>
      redirect({
        to: "/login",
        search: { redirect: location.href === "/" ? undefined : safeRedirect(location.href) },
        replace: true,
      });
    if (!getToken()) throw toLogin();
    try {
      const user = await context.queryClient.ensureQueryData(meQuery());
      return { user };
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        clearSession();
        throw toLogin();
      }
      throw error;
    }
  },
  component: AppShell,
});
