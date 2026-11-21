import { createMemoryHistory } from "@tanstack/react-router";
import { render } from "@testing-library/react";
import { setSession } from "@/api/session";
import { App } from "@/app";
import type { MockDb } from "@/mocks/db";
import { createAppRouter, createQueryClient } from "@/router";

/** Signs a seeded account in by writing a token the mock accepts. */
export function signIn(db: MockDb, username: string) {
  const user = db.users.find((entry) => entry.username === username);
  if (!user) throw new Error(`no mock user ${username}`);
  setSession({ accessToken: db.issueToken(user), expiresAt: null });
}

/** Renders the whole app (router, guards, shell) at `path` against the MSW mock. */
export function renderApp(path: string) {
  const queryClient = createQueryClient();
  queryClient.setDefaultOptions({
    queries: { ...queryClient.getDefaultOptions().queries, retry: false },
  });
  const router = createAppRouter(queryClient, createMemoryHistory({ initialEntries: [path] }));
  const view = render(<App router={router} queryClient={queryClient} />);
  return { ...view, router, queryClient };
}
