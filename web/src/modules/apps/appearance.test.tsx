import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { SkinLoadError } from "@/skins/loader";
import { getSkinState, resetSkinStore, setSkin, setSkinLoader } from "@/skins/store";
import { ALL_FEATURES, installMockDb } from "@/test/msw-server";
import { renderApp, selectUploadedSkin, signIn } from "@/test/render-app";
import { UninstallSkinDialog } from "./skin-dialogs";

afterEach(() => {
  resetSkinStore();
});

/** First paint of the whole app can be slow on a busy machine. */
const FIRST_PAINT = { timeout: 5000 };

function skinsRegion() {
  return screen.findByRole("region", { name: "皮肤" }, FIRST_PAINT);
}

describe("应用 › 外观", () => {
  it("isolates a mixed-direction package name while preserving the spoken confirmation", async () => {
    const name = "Studio אולפן";
    const skin = { id: "studio", pluginId: "org.example.studio", name: { en: name }, description: {},
      version: "1.0.0", sizeBytes: 1024, previewUrl: null };
    const idle = () => {};
    render(<UninstallSkinDialog skin={skin} inUse={false} onClose={idle} onConfirm={idle} busy={false} error={null} />);
    const dialog = await screen.findByRole("alertdialog", { name: `卸载「${name}」？` });
    const heading = within(dialog).getByRole("heading", { name: `卸载「${name}」？` });
    expect(heading.textContent).toBe(`卸载「\u2068${name}\u2069」？`);
  });

  it("lists the built-in skin as the one in use", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.user.username);
    renderApp("/apps/themes");
    const skins = await skinsRegion();
    const rows = within(skins).getAllByRole("listitem");
    expect(rows).toHaveLength(1);
    expect(within(rows[0]!).getByRole("heading", { name: "作品色" })).toBeInTheDocument();
    expect(within(rows[0]!).getByText("正在使用")).toBeInTheDocument();
    expect(rows[0]).toHaveAttribute("aria-current", "true");
    expect(within(rows[0]!).queryByRole("button", { name: /卸载/ })).not.toBeInTheDocument();
  });

  it("without the skins feature keeps only the built-in skin and light/dark, and says the catalogue is not enabled", async () => {
    const db = installMockDb({ features: M0_FEATURES.filter((feature) => feature !== "skins") });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/apps/themes");

    const skins = await skinsRegion();
    expect(within(skins).getAllByRole("listitem")).toHaveLength(1);
    const catalog = screen.getByRole("region", { name: "可以安装的皮肤" });
    expect(within(catalog).getByText("此功能尚未启用")).toBeInTheDocument();
    expect(within(catalog).queryByRole("button")).not.toBeInTheDocument();

    const modes = screen.getByRole("radiogroup", { name: "明暗" });
    expect(within(modes).getAllByRole("radio").map((radio) => radio.textContent)).toEqual(["跟随系统", "浅色", "深色"]);
    await user.click(within(modes).getByRole("radio", { name: "深色" }));
    expect(getSkinState().mode).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("with the skins feature lists installed and available skins, installs one with consent and applies it", async () => {
    await selectUploadedSkin();
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/apps/themes");

    const catalog = await screen.findByRole("region", { name: "可以安装的皮肤" }, FIRST_PAINT);
    for (const name of ["原生克制", "纸墨", "器材"]) expect(await within(catalog).findByRole("heading", { name })).toBeInTheDocument();
    // Before installing, each package's own preview image stands in for the live preview.
    for (const name of ["原生克制", "纸墨", "器材"]) expect(within(catalog).getByRole("img", { name: `「${name}」的预览图` })).toHaveAttribute("src");

    await user.click(within(catalog).getByRole("button", { name: "安装「纸墨」" }));
    const dialog = await screen.findByRole("alertdialog", { name: "安装「纸墨」？" });
    expect(dialog).toHaveAttribute("data-secure-surface", "host");
    expect(within(dialog).getByText(/不含任何代码，也不需要任何权限/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "安装" }));
    await waitFor(() => expect(screen.queryByRole("alertdialog", { name: "安装「纸墨」？" })).not.toBeInTheDocument());
    expect(await screen.findByText("已安装「纸墨」，可以在「皮肤」里应用")).toBeInTheDocument();

    // Installed: listed under 皮肤 with its live preview and actions, gone from the catalogue.
    const skins = await skinsRegion();
    const apply = await within(skins).findByRole("button", { name: "应用「纸墨」" });
    expect(within(skins).getByRole("heading", { name: "纸墨" })).toBeInTheDocument();
    expect(within(skins).getByRole("button", { name: "卸载「纸墨」" })).toBeInTheDocument();
    await waitFor(() => expect(within(catalog).queryByRole("heading", { name: "纸墨" })).not.toBeInTheDocument());

    await user.click(apply);
    await waitFor(() => expect(getSkinState().active.manifest.id).toBe("ink"));
    expect(document.documentElement.dataset.skin).toBe("ink");
    expect(await within(skins).findByText("正在使用")).toBeInTheDocument();
    expect(within(skins).getByRole("button", { name: "应用「作品色」" })).toBeInTheDocument();
  });

  it("explains a skin refused by validation in plain words and applies nothing of it", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    setSkinLoader((id) => Promise.reject(new SkinLoadError("invalid", `light: fg on bg is 2.10:1, needs 4.5:1 (body text on page) [${id}]`)));
    const user = userEvent.setup();
    renderApp("/apps/themes");

    const catalog = await screen.findByRole("region", { name: "可以安装的皮肤" }, FIRST_PAINT);
    await user.click(await within(catalog).findByRole("button", { name: "安装「器材」" }));
    await user.click(within(await screen.findByRole("alertdialog", { name: "安装「器材」？" })).getByRole("button", { name: "安装" }));

    const skins = await skinsRegion();
    const problem = await within(skins).findByRole("alert");
    expect(problem).toHaveTextContent("这个皮肤现在不能用。");
    expect(problem).toHaveTextContent("没有通过检查");
    expect(within(problem).getByText(/needs 4\.5:1/)).toBeInTheDocument();
    expect(within(skins).queryByRole("button", { name: "应用「器材」" })).not.toBeInTheDocument();
    expect(getSkinState().active.manifest.id).toBe("workcolor");
  });

  it("says when the chosen skin could not be loaded and the built-in one is in use", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    setSkinLoader(() => Promise.reject(new SkinLoadError("unreachable", "offline")));
    await setSkin("gear").catch(() => undefined);
    expect(getSkinState().status).toBe("fallback");
    renderApp("/apps/themes");

    const skins = await skinsRegion();
    expect(await within(skins).findByText("正在使用内置的「作品色」")).toBeInTheDocument();
    expect(within(skins).getByText(/暂时连不上服务器/)).toBeInTheDocument();
    expect(within(skins).getByRole("button", { name: "重试" })).toBeInTheDocument();
  });
});
