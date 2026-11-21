import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { useEffect } from "react";
import { setUnauthorizedHandler } from "@/api/client";
import type { AppRouter } from "@/router";

/** Sends the user to the login page when a signed-in request comes back 401. */
function useUnauthorizedRedirect(router: AppRouter, queryClient: QueryClient) {
  useEffect(() => {
    setUnauthorizedHandler(() => {
      queryClient.removeQueries({ queryKey: ["auth"] });
      const { pathname, href } = router.state.location;
      if (pathname === "/login" || pathname === "/setup") return;
      void router.navigate({
        to: "/login",
        search: { redirect: href === "/" ? undefined : href },
        replace: true,
      });
    });
    return () => setUnauthorizedHandler(null);
  }, [router, queryClient]);
}

export function App({ router, queryClient }: { router: AppRouter; queryClient: QueryClient }) {
  useUnauthorizedRedirect(router, queryClient);
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}
