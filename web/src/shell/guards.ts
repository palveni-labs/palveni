import { redirect } from "@tanstack/react-router";
import type { User } from "@/api/types";

/** Route guard for admin-only pages; members go back to the overview. */
export function requireAdmin(context: { user: User }) {
  if (context.user.role !== "admin") throw redirect({ to: "/", replace: true });
}
