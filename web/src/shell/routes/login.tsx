import { createFileRoute, isRedirect, redirect } from "@tanstack/react-router";
import { meQuery } from "@/api/queries/auth";
import { getToken } from "@/api/session";
import { LoginPage } from "../auth/login-page";
import { safeRedirect } from "../auth/redirect";

interface LoginSearch {
  redirect?: string;
}

export const Route = createFileRoute("/login")({
  validateSearch: (search: Record<string, unknown>): LoginSearch => ({
    redirect: safeRedirect(search.redirect),
  }),
  beforeLoad: async ({ context, search }) => {
    if (!getToken()) return;
    try {
      await context.queryClient.ensureQueryData(meQuery());
      throw redirect({ href: search.redirect ?? "/", replace: true });
    } catch (error) {
      if (isRedirect(error)) throw error;
      // Invalid token: stay on the login page.
    }
  },
  component: LoginPage,
});
