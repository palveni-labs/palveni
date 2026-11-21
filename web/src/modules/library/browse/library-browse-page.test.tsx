import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("browsing a library", () => {
  it("opens folders through breadcrumbs and selects multiple entries", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const photos = db.libraries.find((library) => library.name === "家庭相册");
    if (!photos) throw new Error("no seeded photo library");
    const folders = db.listChildren(photos.id, null).filter((item) => item.kind === "folder");
    const [first, second] = folders;
    if (!first || !second) throw new Error("the seeded photo library has no folders");
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

    // Selection mode: activating items toggles them; the bar reports the selection.
    await user.click(await screen.findByRole("button", { name: "选择" }));
    const bar = await screen.findByRole("toolbar", { name: "已选文件的操作" });
    await user.click(screen.getByRole("button", { name: first.title }));
    await user.click(screen.getByRole("button", { name: second.title }));
    expect(screen.getByRole("button", { name: first.title })).toHaveAttribute("aria-pressed", "true");
    expect(within(bar).getByText("已选 2 项")).toBeInTheDocument();
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
    await screen.findByRole("toolbar", { name: "已选文件的操作" });
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("toolbar", { name: "已选文件的操作" })).not.toBeInTheDocument());
  });
});
