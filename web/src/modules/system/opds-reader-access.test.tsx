import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import { M0_FEATURES } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, selectUploadedSkin, signIn } from "@/test/render-app";

it("reveals reader credentials on demand inside account settings and hides them again", async () => {
  await selectUploadedSkin();
  const db = installMockDb({ features: [...M0_FEATURES, "opds"] });
  signIn(db, "family");
  let issued = 0;
  server.use(http.get("/api/v1/me/opds-access", ({ request }) => {
    expect(request.headers.get("Authorization")).toMatch(/^Bearer /);
    issued++;
    return HttpResponse.json({ username: "reader-fixture", password: "reader-only-secret" });
  }));
  renderApp("/system/settings");
  const show = await screen.findByRole("button", { name: "显示阅读器连接信息" }, { timeout: 5000 });
  expect(issued).toBe(0);
  const surface = show.closest("[data-secure-surface=host]");
  await userEvent.click(show);
  expect(surface).toContainElement(await screen.findByDisplayValue("reader-only-secret"));
  expect(surface).toContainElement(screen.getByDisplayValue(window.location.origin + "/opds/"));
  await userEvent.click(screen.getByRole("button", { name: "关闭" }));
  expect(screen.queryByDisplayValue("reader-only-secret")).toBeNull();
  expect(issued).toBe(1);
});
