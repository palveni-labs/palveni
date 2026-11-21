import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { moveNavModule, resetNavPrefs, toggleNavModuleHidden } from "@/shell/nav-prefs";
import { resetSkinStore, setNav } from "@/skins/store";
import { installMockDb } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";

afterEach(() => {
  resetSkinStore();
  resetNavPrefs();
});

describe("shell", () => {
  it("follows the skin's slots: collapsible sidebar for the built-in skin with secondary menus", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    await renderLoadedApp("/");
    const navs = await screen.findAllByRole("navigation", { name: "导航" });
    expect(navs.length).toBeGreaterThan(0);
    expect(document.documentElement.dataset.skin).toBe("workcolor");
    expect(document.documentElement.getAttribute("data-slot-nav")).toBe("sidebar");
    expect(document.documentElement.getAttribute("data-slot-page-title")).toBe("toolbar");
    expect(screen.getAllByText(/^(已连接|正在连接|连接中断，正在重连)$/).length).toBeGreaterThan(0);
    const sidebar = navs[0] as HTMLElement;
    expect(within(sidebar).getByRole("link", { name: "媒体库管理" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("link", { name: "硬件与转码" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("link", { name: "设置" })).toBeInTheDocument();
    const toggle = within(sidebar).getByRole("button", { name: "展开或收起「系统」子菜单" });
    await userEvent.setup().click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(within(sidebar).queryByRole("link", { name: "硬件与转码" })).not.toBeInTheDocument();
  });

  it("lets the user override navigation layout to topbar with dropdown secondary menus", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    setNav("topbar");
    const { router } = await renderLoadedApp("/");

    expect(document.documentElement.getAttribute("data-slot-nav")).toBe("topbar");
    expect(document.documentElement.getAttribute("data-slot-page-title")).toBe("content");
    const nav = (await screen.findAllByRole("navigation", { name: "导航" }))[0] as HTMLElement;
    await user.click(within(nav).getByRole("button", { name: /系统/ }));
    const menu = await screen.findByRole("menu", { name: "系统" });
    await user.click(within(menu).getByRole("menuitem", { name: "硬件与转码" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/system/hardware"));
  });

  it("opens a member's library from the top menu while keeping admin destinations hidden", async () => {
    const db = installMockDb();
    const library = db.createLibrary("Listening shelf", "audiobook", ["/media/spoken"]);
    signIn(db, MOCK_ACCOUNTS.user.username);
    setNav("topbar");
    const user = userEvent.setup();
    const { router } = await renderLoadedApp("/");
    const [nav] = await screen.findAllByRole("navigation", { name: "导航" });
    expect(within(nav!).getByRole("link", { name: "系统" })).toHaveAttribute("href", "/system");
    expect(within(nav!).queryByRole("button", { name: "系统" })).not.toBeInTheDocument();
    await user.click(await within(nav!).findByRole("button", { name: "资料" }));
    const menu = await screen.findByRole("menu");
    await user.click(within(menu).getByRole("menuitem", { name: /Listening shelf/ }));
    await waitFor(() => expect(router.state.location.pathname).toBe(`/library/${library.id}`));
  });

  it("displays the cross-library search title in the desktop toolbar and retains the page heading", async () => {
    const match = window.matchMedia.bind(window);
    vi.spyOn(window, "matchMedia").mockImplementation((query) => ({
      ...match(query), matches: query === "(min-width: 768px)",
    }));
    const db = installMockDb({ features: [...M0_FEATURES, "globalMusicSearch"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    await renderLoadedApp("/library/music-search");
    const heading = await screen.findByRole("heading", { level: 1, name: "搜索所有音乐库" });
    expect(heading).toHaveClass("sr-only");
    expect(within(screen.getByRole("banner")).getByText("搜索所有音乐库")).toBeInTheDocument();
  });

  it("supports right-docked sidebar and custom navigation item ordering and visibility", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    setNav("sidebar-right");
    toggleNavModuleHidden("broadcast");
    toggleNavModuleHidden("overview");
    toggleNavModuleHidden("system");
    moveNavModule("tasks", -1, ["overview", "library", "tasks", "broadcast", "apps", "system"]);
    await renderLoadedApp("/");

    expect(document.documentElement.dataset.navSide).toBe("right");
    expect(document.documentElement.getAttribute("data-slot-nav")).toBe("sidebar");
    const nav = (await screen.findAllByRole("navigation", { name: "导航" }))[0] as HTMLElement;
    expect(nav.closest("aside")).toHaveAttribute("data-sidebar-side", "right");
    expect(within(nav).queryByRole("link", { name: "电台与订阅" })).not.toBeInTheDocument();
    expect(within(nav).getByRole("link", { name: "概览" })).toBeInTheDocument();
    expect(within(nav).getByRole("link", { name: "系统" })).toBeInTheDocument();

    const links = within(nav).getAllByRole("link").map((el) => el.textContent);
    expect(links.indexOf("任务")).toBeLessThan(links.indexOf("资料"));
  });

  it("builds navigation from the registry and opens the command palette with ⌘K", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = await renderLoadedApp("/");

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
