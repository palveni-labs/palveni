import { QueryClient } from "@tanstack/react-query";
import { createRouter, type RouterHistory } from "@tanstack/react-router";
import { ApiError } from "@/api/errors";
import { PagePending, RouteError, RouteNotFound } from "@/shell/route-states";
import { routeTree } from "./routeTree.gen";

export interface RouterContext {
  queryClient: QueryClient;
}

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        // Client errors are final; transport and server errors get two more tries.
        retry: (failureCount, error) =>
          !(error instanceof ApiError && error.status >= 400 && error.status < 500) && failureCount < 2,
        refetchOnWindowFocus: false,
      },
      mutations: { retry: false },
    },
  });
}

export function createAppRouter(queryClient: QueryClient, history?: RouterHistory) {
  return createRouter({
    routeTree,
    history,
    context: { queryClient },
    defaultPreload: "intent",
    // Freshness is decided by TanStack Query, not by the router.
    defaultPreloadStaleTime: 0,
    defaultPendingComponent: PagePending,
    defaultPendingMs: 150,
    defaultPendingMinMs: 200,
    defaultErrorComponent: RouteError,
    defaultNotFoundComponent: RouteNotFound,
  });
}

export type AppRouter = ReturnType<typeof createAppRouter>;

declare module "@tanstack/react-router" {
  interface Register {
    router: AppRouter;
  }
}
