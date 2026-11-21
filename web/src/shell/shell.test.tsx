import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("shell", () => {
  it("follows the skin's slots: topbar tabs for the built-in skin, no sidebar", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    renderApp("/");
    const navs = await screen.findAllByRole("navigation", { name: "导航" });
    expect(navs.length).toBeGreaterThan(0);
    expect(document.documentElement.dataset.skin).toBe("workcolor");
    expect(document.documentElement.getAttribute("data-slot-nav")).toBe("topbar");
    expect(screen.getAllByText(/^(已连接|正在连接|连接中断，正在重连)$/).length).toBeGreaterThan(0);
  });

  it("builds navigation from the registry and opens the command palette with ⌘K", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = renderApp("/");

    const nav = (await screen.findAllByRole("navigation", { name: "导航" }))[0] as HTMLElement;
    for (const label of ["概览", "资料", "任务", "应用", "系统"]) {
      expect(within(nav).getByRole("link", { name: new RegExp(label) })).toBeInTheDocument();
    }

    await user.keyboard("{Control>}k{/Control}");
    const palette = await screen.findByRole("dialog");
    await user.type(within(palette).getByRole("searchbox"), "硬件");
    await user.click(await within(palette).findByText("系统 › 硬件与转码"));
    await waitFor(() => expect(router.state.location.pathname).toBe("/system/hardware"));
  });

});
