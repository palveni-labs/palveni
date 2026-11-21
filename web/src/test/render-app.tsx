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

function createTestApp(path: string) {
  const queryClient = createQueryClient();
  queryClient.setDefaultOptions({
    queries: { ...queryClient.getDefaultOptions().queries, retry: false },
  });
  const router = createAppRouter(queryClient, createMemoryHistory({ initialEntries: [path] }));
  return { router, queryClient };
}

function mountApp(app: ReturnType<typeof createTestApp>) {
  return { ...render(<App {...app} />), ...app };
}

/** Renders the whole app, including initial navigation and its loading states. */
export function renderApp(path: string) {
  return mountApp(createTestApp(path));
}

/** Runs the real route guards and loads the initial route before mounting its interactive UI. */
export async function renderLoadedApp(path: string) {
  const app = createTestApp(path);
  await app.router.load();
  return mountApp(app);
}
