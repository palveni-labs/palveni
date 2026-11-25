import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { getToken, setSession } from "@/api/session";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { resetSkinStore } from "@/skins/store";
import { installMockDb } from "@/test/msw-server";
import { renderApp, selectUploadedSkin, signIn } from "@/test/render-app";

afterEach(() => {
  // The wizard's appearance step changes the light/dark setting.
  resetSkinStore();
});

describe("bootstrap and auth guard", () => {
  it("sends a server without users to the setup wizard", async () => {
    installMockDb({ scenario: "fresh" });
    const { router } = renderApp("/library");
    expect(await screen.findByText("欢迎使用 Palveni Server")).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/setup");
  });

  it("walks through setup and lands signed in", async () => {
    const db = installMockDb({ scenario: "fresh" });
    const user = userEvent.setup();
    renderApp("/");
    await user.click(await screen.findByRole("button", { name: "开始设置" }));
    await user.type(screen.getByLabelText("密码"), "long-enough-pw");
    await user.type(screen.getByLabelText("确认密码"), "long-enough-pw");
    await user.click(screen.getByRole("button", { name: "创建并登录" }));
    // The account exists as soon as the account step succeeds.
    expect(await screen.findByRole("heading", { name: "选择外观" })).toBeInTheDocument();
    expect(db.users).toHaveLength(1);
    expect(db.users[0]?.role).toBe("admin");
    expect(getToken()).not.toBeNull();
    // Appearance: the built-in skin and the light/dark choice, applied at once.
    expect(screen.getByText("精密工坊")).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "深色" }));
    expect(document.documentElement.dataset.theme).toBe("dark");
    await user.click(screen.getByRole("button", { name: /继续/ }));
    expect(await screen.findByText("设置完成")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "添加第一个媒体库" })).toHaveAttribute("href", "/system/libraries?create=true");
  });

  it("checks the account form before creating anything", async () => {
    const db = installMockDb({ scenario: "fresh" });
    const user = userEvent.setup();
    renderApp("/setup");
    await user.click(await screen.findByRole("button", { name: "开始设置" }));
    await user.type(screen.getByLabelText("密码"), "short");
    await user.type(screen.getByLabelText("确认密码"), "different");
    await user.click(screen.getByRole("button", { name: "创建并登录" }));
    expect(await screen.findByText("密码至少 8 个字符")).toBeInTheDocument();
    expect(screen.getByText("两次输入的密码不一致")).toBeInTheDocument();
    expect(db.users).toHaveLength(0);
  });

  it("closes the wizard once setup is done", async () => {
    installMockDb();
    const { router } = renderApp("/setup");
    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
  });

  it("requires a token and remembers where the user was going", async () => {
    installMockDb();
    const { router } = renderApp("/tasks");
    expect(await screen.findByRole("button", { name: "登录" })).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/login");
    expect(router.state.location.search).toEqual({ redirect: "/tasks" });
  });

  it("drops an invalid token and shows the login page", async () => {
    installMockDb();
    setSession({ accessToken: "mock.not-a-user.x", expiresAt: null });
    const { router } = renderApp("/");
    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
    expect(getToken()).toBeNull();
  });

  it("signs in and continues to the requested page", async () => {
    installMockDb();
    const user = userEvent.setup();
    const { router } = renderApp("/login?redirect=%2Flibrary");
    await user.type(await screen.findByLabelText("用户名"), MOCK_ACCOUNTS.admin.username);
    await user.type(screen.getByLabelText("密码"), MOCK_ACCOUNTS.admin.password);
    await user.click(screen.getByRole("button", { name: "登录" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/library"));
    expect(await screen.findByRole("heading", { name: "媒体库" })).toBeInTheDocument();
  });

  it("keeps login errors, credentials and the preferences popover in host surfaces under an uploaded skin", async () => {
    await selectUploadedSkin();
    installMockDb();
    const user = userEvent.setup();
    renderApp("/login");
    await user.type(await screen.findByLabelText("用户名"), "admin");
    await user.type(screen.getByLabelText("密码"), "wrong-password");
    await user.click(screen.getByRole("button", { name: "登录" }));
    const error = await screen.findByText("用户名或密码错误");
    const scope = error.closest("[data-secure-surface=host]");
    expect(scope).toContainElement(screen.getByLabelText("密码"));
    expect(scope).toContainElement(screen.getByRole("button", { name: "登录" }));
    expect(screen.getByLabelText("密码")).toHaveValue("");
    expect(screen.getByLabelText("密码")).toHaveFocus();

    await user.click(screen.getByRole("button", { name: "外观" }));
    const menu = await screen.findByRole("menu", { name: "外观" });
    expect(scope).not.toContainElement(menu);
    expect(menu.closest("[data-secure-surface=host]")).toBeInTheDocument();
    await user.click(screen.getByRole("menuitem", { name: /深色/ }));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("asks for missing fields without calling the server", async () => {
    installMockDb();
    const user = userEvent.setup();
    const { router } = renderApp("/login");
    await user.click(await screen.findByRole("button", { name: "登录" }));
    expect(screen.getByText("请输入用户名")).toBeInTheDocument();
    expect(screen.getByText("请输入密码")).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/login");
  });

  it("keeps members out of admin pages", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.user.username);
    const { router } = renderApp("/system/hardware");
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
    expect(await screen.findByText("电影")).toBeInTheDocument();
    await router.navigate({ to: "/system/settings" });
    expect(await screen.findByRole("heading", { name: "设置" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "硬件与转码" })).not.toBeInTheDocument();
  });

  it("returns to the login page when the session is revoked mid-use", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const token = getToken();
    const { router } = renderApp("/library");
    await screen.findByRole("heading", { name: "媒体库" });

    db.revoke(token ?? "");
    // The hardware report is not cached yet, so this navigation hits the API with the dead token.
    void router.navigate({ to: "/system/hardware" });
    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
    expect(getToken()).toBeNull();
  });

  it("signs out from settings straight to the login page", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const errors = vi.spyOn(console, "error");
    const { router } = renderApp("/system/settings");
    await user.click(await screen.findByRole("button", { name: "退出登录" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));
    expect(await screen.findByRole("button", { name: "登录" })).toBeInTheDocument();
    expect(getToken()).toBeNull();
    // The workspace left before the signed-in user was forgotten: nothing under it lost its user.
    expect(errors.mock.calls.flat().map(String).join("\n")).not.toMatch(/outside the authenticated layout/);
    errors.mockRestore();
  });
});
