import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { shell } from "@/shell/shell-store";
import { installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

afterEach(() => vi.restoreAllMocks());

describe("browsing a library", () => {
  it("opens folders through the breadcrumbs and hands a selection to the assistant", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "assistant"] });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const photos = db.libraries.find((library) => library.name === "家庭相册");
    if (!photos) throw new Error("no seeded photo library");
    const folders = db.listChildren(photos.id, null).filter((item) => item.kind === "folder");
    const [first, second] = folders;
    if (!first || !second) throw new Error("the seeded photo library has no folders");
    const openAssistant = vi.spyOn(shell, "openAssistant");
    const user = userEvent.setup();
    const { router } = renderApp(`/library/${photos.id}`);

    // At the top level the trail is just "全部文件"; opening a folder adds it as the current step.
    const trail = () => screen.getByRole("navigation", { name: "当前位置" });
    expect(within(await screen.findByRole("navigation", { name: "当前位置" }, { timeout: 5000 })).getByText("全部文件")).toHaveAttribute("aria-current", "page");
    await user.click(await screen.findByRole("button", { name: first.title }));
    await waitFor(() => expect(router.state.location.search).toEqual({ parent: first.id }));
    await waitFor(() => expect(within(trail()).getByText(first.title)).toHaveAttribute("aria-current", "page"));
    await user.click(within(trail()).getByRole("link", { name: "全部文件" }));
    await waitFor(() => expect(router.state.location.search).toEqual({}));

    // Selection mode: activating items toggles them; the bar counts and hands them off.
    await user.click(await screen.findByRole("button", { name: "选择" }));
    const bar = await screen.findByRole("toolbar", { name: "已选文件的操作" });
    expect(within(bar).getByRole("button", { name: "交给助手" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: first.title }));
    await user.click(screen.getByRole("button", { name: second.title }));
    expect(screen.getByRole("button", { name: first.title })).toHaveAttribute("aria-pressed", "true");
    expect(within(bar).getByText("已选 2 项")).toBeInTheDocument();
    await user.click(within(bar).getByRole("button", { name: "交给助手" }));
    expect(openAssistant).toHaveBeenCalledWith(expect.objectContaining({ itemIds: [first.id, second.id] }));
    expect(screen.queryByRole("toolbar", { name: "已选文件的操作" })).not.toBeInTheDocument();
    // Selecting did not navigate into the folders.
    expect(router.state.location.search).toEqual({});
  }, 15_000);

  it("leaves selection mode with Escape", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const photos = db.libraries.find((library) => library.name === "家庭相册");
    if (!photos) throw new Error("no seeded photo library");
    const user = userEvent.setup();
    renderApp(`/library/${photos.id}`);

    await user.click(await screen.findByRole("button", { name: "选择" }, { timeout: 5000 }));
    const bar = await screen.findByRole("toolbar", { name: "已选文件的操作" });
    // Without the assistant feature the hand-off says why it is unavailable.
    expect(within(bar).getByText("助手尚未启用")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("toolbar", { name: "已选文件的操作" })).not.toBeInTheDocument());
  });
});
