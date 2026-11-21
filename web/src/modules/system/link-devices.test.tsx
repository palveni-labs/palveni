import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import { M0_FEATURES } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";

it("lets a member cancel or revoke one of two identically named devices", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "link"] });
  signIn(db, "family");
  let devices = [
    { id: "first", name: "Reader", nodeId: "a".repeat(64), connected: true },
    { id: "second", name: "Reader", nodeId: "b".repeat(64), connected: false },
  ];
  const removed: string[] = [];
  server.use(
    http.get("/api/v1/link/devices", () => HttpResponse.json({ devices })),
    http.delete("/api/v1/link/devices/:id", ({ params }) => {
      const id = String(params.id);
      removed.push(id);
      devices = devices.filter(device => device.id !== id);
      return new HttpResponse(null, { status: 204 });
    }),
  );
  await renderLoadedApp("/system/settings");
  const user = userEvent.setup();
  const fingerprint = "b".repeat(12);
  const row = within((await screen.findByText(fingerprint, {}, { timeout: 5000 })).closest("li")!);
  const revoke = () => user.click(row.getByRole("button", { name: "撤销配对" }));
  await revoke();
  await user.click(row.getByRole("button", { name: "取消" }));
  expect(removed).toEqual([]);
  await revoke();
  await revoke();
  await waitFor(() => expect(screen.queryByText(fingerprint)).toBeNull());
  expect(screen.getByText("a".repeat(12))).toBeInTheDocument();
  expect(removed).toEqual(["second"]);
});
