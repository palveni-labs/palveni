import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import type { PluginList } from "@/api/types";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("应用 › 插件", () => {
  it("explains the empty list an M0 server returns and points to skins", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    renderApp("/apps/plugins");
    expect(await screen.findByText("还没有安装插件", undefined, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "前往外观" })).toHaveAttribute("href", "/apps/themes");
  });

  it("lists each plugin with its identifier, runtime in words, extension points and state", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const list: PluginList = {
      plugins: [
        { id: "org.example.tmdb", name: "TMDB", version: "1.2.0", runtime: "wasm", enabled: true, provides: ["metadata", "sync"] },
        { id: "org.example.whisper", name: "Whisper", version: "2.0.0", runtime: "sidecar", enabled: false, provides: ["x-custom"] },
      ],
    };
    server.use(http.get("/api/v1/plugins", () => HttpResponse.json(list)));
    renderApp("/apps/plugins");

    const section = await screen.findByRole("region", { name: "已安装的插件" }, { timeout: 5000 });
    await within(section).findByRole("heading", { name: /TMDB/ });
    const [tmdb, whisper] = within(section).getAllByRole("listitem").filter((item) => item.querySelector("h3"));
    expect(within(tmdb!).getByRole("heading", { name: /TMDB/ })).toBeInTheDocument();
    expect(within(tmdb!).getByText("org.example.tmdb")).toBeInTheDocument();
    expect(within(tmdb!).getByText("WASM 组件")).toBeInTheDocument();
    expect(within(tmdb!).getByText("元数据")).toBeInTheDocument();
    expect(within(tmdb!).getByText("已启用")).toBeInTheDocument();
    expect(within(whisper!).getByText(/不在沙箱里/)).toBeInTheDocument();
    expect(within(whisper!).getByText("x-custom")).toBeInTheDocument();
    expect(within(whisper!).getByText("已停用")).toBeInTheDocument();
  });

  it("opens the store tab from Apps, installs an official skin and manages it in Appearance", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = renderApp("/apps");

    await user.click(await screen.findByRole("tab", { name: "配方商店" }, { timeout: 5000 }));
    const available = await screen.findByRole("region", { name: "官方目录中可安装的插件" });
    const search = screen.getByRole("searchbox", { name: "搜索插件" });
    expect(screen.getByRole("link", { name: "向 GitHub 插件仓投稿" })).toHaveAttribute(
      "href",
      "https://github.com/palveni-labs/plugins#submit-a-plugin",
    );
    expect(screen.queryByRole("region", { name: "已安装的插件" })).not.toBeInTheDocument();
    await user.type(search, "ink");
    await user.click(await within(available).findByRole("button", { name: "安装「纸墨」" }));
    expect(within(available).queryByRole("heading", { name: "原生克制" })).not.toBeInTheDocument();
    const dialog = await screen.findByRole("alertdialog", { name: "安装「纸墨」？" });
    await user.click(within(dialog).getByRole("button", { name: "安装" }));
    await waitFor(() => expect(within(available).queryByRole("button", { name: "安装「纸墨」" })).not.toBeInTheDocument());
    await user.clear(search);
    await user.click(screen.getByRole("tab", { name: "已安装" }));
    const installed = await screen.findByRole("region", { name: "已安装的插件" });
    expect(await within(installed).findByRole("heading", { name: /纸墨/ })).toBeInTheDocument();
    await user.click(within(installed).getByRole("link", { name: "在外观中管理" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/apps/themes"));
    expect(await screen.findByRole("button", { name: "卸载「纸墨」" })).toBeInTheDocument();
  });

  it("enforces permission review gate on recipe install, edits settings via drawer, rolls back and uninstalls", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/apps/plugins");

    // Switch to Recipe Store tab
    await user.click(await screen.findByRole("tab", { name: "配方商店" }, { timeout: 5000 }));
    const storeSection = await screen.findByRole("region", { name: "官方配方与小组件商店" });
    const categoryGroup = screen.getByRole("group", { name: "插件类型" });
    await user.click(within(categoryGroup).getByRole("button", { name: /概览小组件/ }));
    expect(within(storeSection).queryByRole("heading", { name: "TMDB Metadata Recipe" })).not.toBeInTheDocument();
    expect(within(storeSection).getByRole("heading", { name: "Home Overview Clock & Media Summary Widget" })).toBeInTheDocument();
    await user.click(within(categoryGroup).getByRole("button", { name: /全部类型/ }));

    const tmdbHeading = await within(storeSection).findByRole("heading", { name: "TMDB Metadata Recipe" });
    const tmdbCard = tmdbHeading.closest("article")!;
    expect(within(tmdbCard).getByText("中风险")).toBeInTheDocument();

    // Click Install -> Permission Risk Review Modal opens with confirm disabled
    await user.click(within(tmdbCard).getByRole("button", { name: "安装" }));
    const reviewModal = await screen.findByRole("dialog", { name: /权限与风险审查：TMDB Metadata Recipe/ });
    const confirmBtn = within(reviewModal).getByRole("button", { name: "同意授权并安装" });
    expect(confirmBtn).toBeDisabled();

    // Check only first permission -> still disabled
    const checkboxes = within(reviewModal).getAllByRole("checkbox");
    expect(checkboxes.length).toBe(2);
    await user.click(checkboxes[0]!);
    expect(confirmBtn).toBeDisabled();

    // Check second permission -> enabled -> confirm install
    await user.click(checkboxes[1]!);
    expect(confirmBtn).toBeEnabled();
    await user.click(confirmBtn);
    await waitFor(() => expect(screen.queryByRole("dialog", { name: /权限与风险审查/ })).not.toBeInTheDocument());

    // Switch back to Installed tab -> TMDB card appears with Switch, Settings, Rollback, Uninstall
    await user.click(screen.getByRole("tab", { name: "已安装" }));
    const installedSection = await screen.findByRole("region", { name: "已安装的插件" });
    expect(await within(installedSection).findByRole("heading", { name: /TMDB Metadata Recipe/ })).toBeInTheDocument();

    // Open Settings drawer -> change language enum & boolean -> save
    await user.click(within(installedSection).getByRole("button", { name: "配置" }));
    const settingsDrawer = await screen.findByRole("dialog", { name: /插件配置：TMDB Metadata Recipe/ });
    const langSelect = within(settingsDrawer).getByLabelText("Metadata Language");
    await user.selectOptions(langSelect, "en-US");
    await user.click(within(settingsDrawer).getByRole("button", { name: "保存配置" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: /插件配置/ })).not.toBeInTheDocument());
    expect(db.plugins.find((p) => p.id === "palveni.recipe.tmdb")?.settings?.language).toBe("en-US");

    // Toggle Enable/Disable switch
    const toggle = within(installedSection).getByRole("switch", { name: /TMDB Metadata Recipe/ });
    expect(toggle).toHaveAttribute("aria-checked", "true");
    await user.click(toggle);
    await waitFor(() => expect(toggle).toHaveAttribute("aria-checked", "false"));

    // Rollback from 1.2.0 to 1.1.0
    await user.click(within(installedSection).getByRole("button", { name: "回滚至 1.1.0" }));
    expect(await within(installedSection).findByText("版本 1.1.0")).toBeInTheDocument();
    expect(within(installedSection).queryByRole("button", { name: /回滚至/ })).not.toBeInTheDocument();

    // Uninstall
    await user.click(within(installedSection).getByRole("button", { name: "卸载" }));
    expect(await screen.findByText("还没有安装插件")).toBeInTheDocument();
  }, 15_000);

  it("opens appearance when a member enters Apps", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.user.username);
    const { router } = renderApp("/apps");
    await waitFor(() => expect(router.state.location.pathname).toBe("/apps/themes"));
    expect(screen.queryByRole("searchbox")).not.toBeInTheDocument();
  });

  it("sends members back to the overview", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.user.username);
    const { router } = renderApp("/apps/plugins");
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
    expect(screen.queryByText("还没有安装插件")).not.toBeInTheDocument();
  });
});
