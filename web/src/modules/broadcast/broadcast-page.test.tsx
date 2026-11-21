import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { HttpResponse, http } from "msw";
import { describe, expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

const source = {
  id: "019a3eed-0210-71c0-8af9-5f3ee36fbd79",
  name: "City FM",
  kind: "radio",
  entryCount: 1,
  version: 1,
  feedUrl: null,
  guideUrl: null,
};
const station = {
  id: "00".repeat(32),
  title: "City broadcast",
  group: "Local",
  streams: ["https://media.example/city", "https://other.example/city"],
  favorite: false,
  current: null,
  next: null,
  publishedAt: null,
};

describe("broadcast subscriptions", () => {
  it("imports text into the selected media section and lists the result", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "broadcast"] });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    let saved: { kind: string; expectedVersion: number; content: string } | undefined;
    server.use(
      http.get("*/api/v1/broadcast/sources", ({ request }) =>
        HttpResponse.json({
          sources:
            saved && new URL(request.url).searchParams.get("kind") === "radio" ? [source] : [],
        }),
      ),
      http.put("*/api/v1/broadcast/sources/:id", async ({ request }) => {
        saved = (await request.json()) as typeof saved;
        return HttpResponse.json(source);
      }),
      http.get("*/api/v1/broadcast/sources/:id/entries", () =>
        HttpResponse.json({ entries: [station], nextOffset: null }),
      ),
    );
    const user = userEvent.setup();
    renderApp("/broadcast?kind=radio");
    await user.click(await screen.findByRole("button", { name: "添加订阅" }));
    await user.type(screen.getByLabelText("名称"), source.name);
    fireEvent.change(screen.getByLabelText("频道清单或播客订阅", { selector: "textarea" }), {
      target: { value: "City,https://media.example/city" },
    });
    await user.click(screen.getByRole("button", { name: "保存" }));
    expect(await screen.findByText(station.title)).toBeVisible();
    expect(saved).toMatchObject({
      kind: "radio",
      expectedVersion: 0,
      content: "City,https://media.example/city",
    });
  });

  it("keeps the channel catalogue and podcast list separate for household members", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "broadcast"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    server.use(
      http.get("*/api/v1/broadcast/sources", ({ request }) =>
        HttpResponse.json({
          sources: new URL(request.url).searchParams.get("kind") === "radio" ? [source] : [],
        }),
      ),
      http.get("*/api/v1/broadcast/sources/:id/entries", () =>
        HttpResponse.json({
          entries: [{ ...station, streams: ["file:///home/private", ...station.streams] }],
          nextOffset: null,
        }),
      ),
    );
    const user = userEvent.setup();
    renderApp("/broadcast?kind=radio");
    await user.click(await screen.findByRole("button", { name: "播放 City broadcast" }));
    expect(screen.getByRole("link", { name: "打开媒体地址" })).toHaveAttribute(
      "href",
      station.streams[0],
    );
    await user.selectOptions(screen.getByRole("combobox", { name: "播放线路" }), "1");
    expect(screen.getByRole("link", { name: "打开媒体地址" })).toHaveAttribute(
      "href",
      station.streams[1],
    );
    expect(screen.queryByRole("button", { name: "添加订阅" })).toBeNull();
    await user.click(screen.getByRole("link", { name: "播客" }));
    expect(await screen.findByText("还没有订阅")).toBeVisible();
    expect(screen.queryByText(station.title)).toBeNull();
  });
});
