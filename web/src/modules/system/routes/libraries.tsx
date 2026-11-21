import { createFileRoute } from "@tanstack/react-router";
import { jobsQuery } from "@/api/queries/jobs";
import { librariesQuery } from "@/api/queries/libraries";
import { requireAdmin } from "@/shell/guards";
import { AdminLibrariesPage } from "../admin-libraries-page";

interface AdminLibrariesSearch {
  /** Opens the create dialog (links from empty states and the setup wizard). */
  create?: boolean;
}

export const Route = createFileRoute("/_app/system/libraries")({
  validateSearch: (search: Record<string, unknown>): AdminLibrariesSearch => ({
    create: search.create === true || search.create === "1" || search.create === 1 ? true : undefined,
  }),
  beforeLoad: ({ context }) => requireAdmin(context),
  loader: async ({ context }) => {
    void context.queryClient.prefetchQuery(jobsQuery());
    await context.queryClient.ensureQueryData(librariesQuery());
  },
  component: AdminLibrariesPage,
});
