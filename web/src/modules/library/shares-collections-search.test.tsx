import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { request } from "@/api/client";
import type { ShareLinkSummary } from "@/api/queries/shares-collections";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { ALL_FEATURES, installMockDb } from "@/test/msw-server";
import { renderApp, renderLoadedApp, signIn } from "@/test/render-app";

const TRACK_B_FEATURES = [
  ...ALL_FEATURES,
  "video",
  "music",
  "reading",
  "photos",
  "shares",
  "collections",
  "globalSearch",
];

describe("shares, cross-media collections, and global search", () => {
  it("allows unauthenticated visitors to unlock a password-protected share and hides stripped GPS", async () => {
    const db = installMockDb({ features: TRACK_B_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);

    // Create a password-protected photo album share with `stripGps: true`
    const created = await request<ShareLinkSummary>("/shares", {
      method: "POST",
      body: {
        targetKind: "photo_album",
        targetId: "album-kyushu",
        title: "九州巡礼相册",
        password: "4321",
        stripGps: true,
        allowDownload: false,
      },
    });

    // Clear session to simulate an anonymous external recipient opening `/s/$token`
    localStorage.clear();
    const user = userEvent.setup();
    renderApp(`/s/${created.token}`);

    expect(await screen.findByRole("heading", { name: "九州巡礼相册" })).toBeInTheDocument();
    expect(screen.getByText("请输入分享者提供的访问口令以查看内容。")).toBeInTheDocument();

    // Wrong password displays error without unlocking
    await user.type(screen.getByLabelText("访问口令"), "9999");
    await user.click(screen.getByRole("button", { name: "解锁查看" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("口令不正确");

    // Correct password unlocks share and shows GPS-stripped privacy badge
    await user.clear(screen.getByLabelText("访问口令"));
    await user.type(screen.getByLabelText("访问口令"), "4321");
    await user.click(screen.getByRole("button", { name: "解锁查看" }));

    expect(await screen.findByText("位置信息已隐藏")).toBeInTheDocument();
    const photoThumb = await screen.findByAltText("巡礼风景 01");
    await user.click(photoThumb.closest("button")!);
    const dialog = await screen.findByRole("dialog", { name: "巡礼风景 01" });
    expect(within(dialog).getByText("SONY ILCE-7C")).toBeInTheDocument();
    // Stripped GPS coordinates are never rendered
    expect(within(dialog).queryByText(/33\.2794/)).not.toBeInTheDocument();
  });

  it("manages and revokes share links at /library/shares", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "shares"] });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const created = await request<ShareLinkSummary>("/shares", {
      method: "POST",
      body: {
        targetKind: "video_work",
        targetId: "work_1",
        title: "沙丘 2",
      },
    });

    const user = userEvent.setup();
    await renderLoadedApp("/library/shares");

    expect(await screen.findByText("沙丘 2")).toBeInTheDocument();
    expect(screen.getByText("生效中")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "撤销" }));
    expect(await screen.findByText("已撤销")).toBeInTheDocument();

    // Visiting the revoked link shows the expired/revoked screen
    const { unmount } = renderApp(`/s/${created.token}`);
    expect(await screen.findByText("分享链接已失效或已撤销")).toBeInTheDocument();
    unmount();
  });

  it("creates a cross-media collection and adds auto-discovered related works", async () => {
    const db = installMockDb({ features: TRACK_B_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = await renderLoadedApp("/library/collections");

    await user.click(await screen.findAllByRole("button", { name: "新建合集" }).then((btns) => btns[0]!));
    const modal = await screen.findByRole("dialog", { name: "新建跨媒体合集" });
    await user.type(within(modal).getByLabelText("合集名称"), "铃芽之旅");
    await user.click(within(modal).getByRole("button", { name: "创建" }));

    await waitFor(() => expect(router.state.location.pathname).toMatch(/^\/library\/collections\//));
    expect(await screen.findByText("跨媒体自动关联")).toBeInTheDocument();
    expect(screen.getByText("铃芽之旅 原声带")).toBeInTheDocument();
    expect(screen.getByText("铃芽之旅（官方小说）")).toBeInTheDocument();

    // Add an auto-discovered work into the manual collection items
    const addButtons = screen.getAllByRole("button", { name: "加入合集" });
    await user.click(addButtons[0]!);
    await waitFor(() => expect(screen.getAllByText(/1 项/).length).toBeGreaterThan(0));
  });

  it("queries live cross-domain search in ⌘K Command Palette and navigates on selection", async () => {
    const db = installMockDb({ features: TRACK_B_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = await renderLoadedApp("/");

    await user.keyboard("{Control>}k{/Control}");
    const palette = await screen.findByRole("dialog");
    await user.type(within(palette).getByRole("searchbox"), "铃芽");

    expect(await within(palette).findByText("全库与家居检索")).toBeInTheDocument();
    expect(within(palette).getByText("铃芽之旅 原声带")).toBeInTheDocument();
    expect(within(palette).getByText("铃芽之旅（官方小说）")).toBeInTheDocument();

    await user.click(within(palette).getByText("铃芽之旅（官方小说）"));
    await waitFor(() => expect(router.state.location.pathname).toMatch(/^\/reading\/book\//));
  });
});
