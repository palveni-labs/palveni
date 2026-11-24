import { createFileRoute } from "@tanstack/react-router";
import { requireAdmin } from "@/shell/guards";
import { UsersPage } from "../users-page";

export const Route = createFileRoute("/_app/system/users")({
  beforeLoad: ({ context }) => requireAdmin(context),
  component: UsersPage,
});
