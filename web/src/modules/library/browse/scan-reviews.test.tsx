import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import { queryKeys } from "@/api/query-keys";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";
import { ScanReviews } from "./scan-reviews";

const scope = "/media/collection";
const samplePath = scope + "/01.flac";
const snapshot = { scope, revision: "first", missingCount: 51, samplePaths: [samplePath] };
const wait = { timeout: 5000 };

it("reviews pages from the library and requires a fresh decision after a conflicting confirmation", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "scanReviews"] });
  const library = db.createLibrary("音乐示例", "music", [scope]);
  signIn(db, MOCK_ACCOUNTS.admin.username);
  const base = `/api/v1/libraries/${library.id}/scan/reviews`;
  let latest: typeof snapshot | null = snapshot;
  const confirmations: string[] = [];
  const pages: string[] = [];
  server.use(
    http.get(base, () => HttpResponse.json({ reviews: latest ? [latest] : [] })),
    http.get(base + "/:revision/items", ({ params, request }) => {
      const query = new URL(request.url).searchParams;
      const offset = Number(query.get("offset"));
      const limit = Number(query.get("limit"));
      const revision = String(params.revision);
      pages.push(revision + "/" + offset);
      const total = latest?.missingCount ?? 0;
      return HttpResponse.json({ total, paths: Array.from({ length: Math.min(limit, total - offset) }, (_, i) => `${scope}/${offset+i+1}.flac`) });
    }),
    http.post(base + "/:revision/confirm", ({ params }) => {
      const revision = String(params.revision);
      confirmations.push(revision);
      if (revision === "first") {
        latest = { ...snapshot, revision: "second", missingCount: 1 };
        return HttpResponse.json({ error: { code: "conflict", message: "Review changed" } }, { status: 409 });
      }
      latest = null;
      return HttpResponse.json({ jobId: "review-job" }, { status: 202 });
    }),
  );
  const user = userEvent.setup();
  const { queryClient } = renderApp(`/library/${library.id}`);
  const open = () => screen.findByRole("button", { name: `复核 ${scope} 下的缺失文件` }, wait);
  await user.click(await open());
  let dialog = within(await screen.findByRole("alertdialog"));
  await dialog.findByText(scope + "/1.flac", {}, wait);
  await user.click(dialog.getByRole("button", { name: "下一页" }));
  await dialog.findByText(scope + "/51.flac", {}, wait);
  expect(pages).toContain("first/50");
  await user.click(dialog.getByRole("button", { name: "取消" }));
  expect(confirmations).toEqual([]);

  await user.click(await open());
  dialog = within(await screen.findByRole("alertdialog"));
  await dialog.findByText(scope + "/1.flac", {}, wait);
  await user.click(dialog.getByRole("button", { name: "移除 51 个媒体库条目" }));
  await dialog.findByRole("alert", {}, wait);
  expect(dialog.queryByRole("button", { name: /移除/ })).not.toBeInTheDocument();
  expect(confirmations).toEqual(["first"]);
  await user.click(dialog.getByRole("button", { name: "取消" }));

  await user.click(await open());
  dialog = within(await screen.findByRole("alertdialog"));
  await dialog.findByText(scope + "/1.flac", {}, wait);
  await user.click(dialog.getByRole("button", { name: "移除 1 个媒体库条目" }));
  await waitFor(() => expect(queryClient.getQueryData(queryKeys.libraries.scanJob(library.id))).toBe("review-job"), wait);
  expect(confirmations).toEqual(["first", "second"]);
  await waitFor(() => expect(screen.queryByText("缺失的媒体条目")).not.toBeInTheDocument(), wait);
}, 15000);

it("keeps review paths and permissions out of a member's view, even with cached administrator data", () => {
  const db = installMockDb({ features: [...M0_FEATURES, "scanReviews"] });
  const account = db.users.find((u) => u.username === MOCK_ACCOUNTS.user.username)!;
  signIn(db, account.username);
  const library = db.createLibrary("家庭媒体", "music", [scope]);
  const client = new QueryClient();
  client.setQueryData(queryKeys.auth.me, account);
  client.setQueryData(queryKeys.system.info, { features: ["scanReviews"] });
  client.setQueryData(queryKeys.libraries.scanReviews(library.id), { reviews: [snapshot] });
  let requested = false;
  server.use(http.get(`/api/v1/libraries/${library.id}/scan/reviews`, () => {
    requested = true;
    return new HttpResponse(null, { status: 403 });
  }));
  const { container } = render(<QueryClientProvider client={client}><ScanReviews library={library} /></QueryClientProvider>);
  expect(client.getQueryCache().find({ queryKey: queryKeys.libraries.scanReviews(library.id) })?.getObserversCount()).toBe(0);
  expect(container).toBeEmptyDOMElement();
  expect(requested).toBe(false);
});

it("refreshes a retired revision on page load and prevents confirmation while a scan is active", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "scanReviews"] });
  const account = db.users.find((u) => u.username === MOCK_ACCOUNTS.admin.username)!;
  signIn(db, account.username);
  const library = db.createLibrary("音乐", "music", [scope]);
  const base = `/api/v1/libraries/${library.id}/scan/reviews`;
  let fresh = false;
  let requests = 0;
  server.use(
    http.get(base, () => {
      requests += 1;
      return HttpResponse.json({ reviews: [{ ...snapshot, revision: fresh ? "replacement" : "old" }] });
    }),
    http.get(base + "/old/items", () => {
      fresh = true;
      return HttpResponse.json({ error: { code: "conflict", message: "Review changed" } }, { status: 409 });
    }),
    http.get(base + "/replacement/items", () => HttpResponse.json({ paths: snapshot.samplePaths, total: snapshot.missingCount })),
  );
  const client = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity, retry: false } } });
  client.setQueryData(queryKeys.auth.me, account);
  client.setQueryData(queryKeys.system.info, { features: ["scanReviews", "jobs"] });
  const active = { id: "running", kind: "library.scan", subject: { type: "library", id: library.id }, state: "running" };
  server.use(http.get("/api/v1/jobs", () => HttpResponse.json({ jobs: [active] })));
  const user = userEvent.setup();
  render(<QueryClientProvider client={client}><ScanReviews library={library} /></QueryClientProvider>);
  const open = () => screen.findByRole("button", { name: `复核 ${scope} 下的缺失文件` }, wait);
  await user.click(await open());
  let dialog = within(await screen.findByRole("alertdialog"));
  await dialog.findByRole("alert", {}, wait);
  expect(dialog.queryByRole("button", { name: /移除/ })).not.toBeInTheDocument();
  await waitFor(() => expect(requests).toBeGreaterThan(1), wait);
  await user.click(dialog.getByRole("button", { name: "取消" }));
  await user.click(await open());
  dialog = within(await screen.findByRole("alertdialog"));
  await dialog.findByText(samplePath, {}, wait);
  expect(dialog.getByRole("button", { name: "移除 51 个媒体库条目" })).toBeDisabled();
  expect(dialog.getByText(/扫描正在进行/)).toBeVisible();
}, 15000);
