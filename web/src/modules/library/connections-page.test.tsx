import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";

it("lets a member browse nested entries and plays through the server-scoped URL", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "connections"] });
  signIn(db, MOCK_ACCOUNTS.user.username);
  const source = { id: "019a3eed-0210-71c0-8af9-5f3ee36fbd79", name: "Family NAS", kind: "smb" };
  const base = "*/api/v1/connections";
  const requested: unknown[] = [];
  server.use(
    http.get(base, () => HttpResponse.json({ connections: [source] })),
    http.get(base + "/:id/entries", ({ request }) => {
      const parent = new URL(request.url).searchParams.get("parent");
      return HttpResponse.json({
        entries:
          parent === "room"
            ? [{ id: "recording", name: "Recording", kind: "audio" }]
            : [{ id: "room", name: "Concerts", kind: "folder" }],
        nextOffset: null,
      });
    }),
    http.post(base + "/:id/playback", async ({ request }) => {
      requested.push(await request.json());
      return HttpResponse.json({
        mode: "direct",
        url: "/api/v1/connections/" + source.id + "/stream?entry=recording&ticket=sample",
        expiresAt: "2099-01-01T00:00:00Z",
      });
    }),
  );
  const user = userEvent.setup();
  await renderLoadedApp("/library/connections");
  await user.click(await screen.findByRole("button", { name: /Family NAS/ }));
  await user.click(await screen.findByRole("button", { name: "Concerts" }));
  await user.click(await screen.findByRole("button", { name: "Recording" }));
  await waitFor(() =>
    expect(screen.getByLabelText("Recording", { selector: "audio" })).toHaveAttribute(
      "src",
      expect.stringContaining("ticket=sample"),
    ),
  );
  expect(requested).toEqual([{ entry: "recording" }]);
  expect(screen.queryByRole("button", { name: "移除连接" })).toBeNull();
  expect(screen.queryByRole("button", { name: "添加来源" })).toBeNull();
});
