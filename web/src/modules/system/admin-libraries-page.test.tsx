import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, selectUploadedSkin, signIn } from "@/test/render-app";

describe("library management", () => {
  it("points at the rejected path, then creates and scans the library with live updates", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/system/libraries");

    await user.click(await screen.findByRole("button", { name: "新建媒体库" }));
    const dialog = await screen.findByRole("dialog");
    const form = within(dialog);
    await user.type(form.getByLabelText("名称"), "纪录片");
    await user.click(form.getByRole("radio", { name: "影视" }));
    await user.type(form.getByLabelText("文件夹路径"), "/srv/media/docs");
    await user.click(form.getByRole("button", { name: "添加路径" }));
    await user.type(form.getByLabelText("文件夹路径 2"), "/mnt/missing/docs");
    await user.click(form.getByRole("button", { name: "创建" }));
    expect(await form.findByText("「/mnt/missing/docs」不存在或不是文件夹")).toBeInTheDocument();
    // The server names the second path (details.index); that input, and only that one, is marked.
    expect(form.getByLabelText("文件夹路径 2")).toHaveAttribute("aria-invalid", "true");
    expect(form.getByLabelText("文件夹路径")).not.toHaveAttribute("aria-invalid");
    expect(form.getByLabelText("名称")).not.toHaveAttribute("aria-invalid");

    await user.click(form.getByRole("button", { name: "移除第 2 个路径" }));
    await user.click(form.getByRole("button", { name: "创建" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    const created = db.libraries.find((library) => library.name === "纪录片");
    expect(created?.paths).toEqual(["/srv/media/docs"]);
    // "Scan after creating" is on by default; the finished scan arrives over SSE.
    const row = (await screen.findByText("/srv/media/docs")).closest("[data-library-row]") as HTMLElement;
    await waitFor(() => expect(created?.lastScanAt).not.toBeNull(), { timeout: 3000 });
    await waitFor(() => expect(within(row).queryByText("从未扫描")).not.toBeInTheDocument(), { timeout: 3000 });
    // The live scan alone can take most of the default 5 s.
  }, 15_000);

  it("renames a library and extends its folders in place", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/system/libraries");
    const row = (await screen.findByText("/srv/books")).closest("[data-library-row]") as HTMLElement;
    const id = db.libraries.find((entry) => entry.paths.includes("/srv/books"))!.id;
    await user.click(within(row).getByRole("button", { name: "更多" }));
    await user.click(await screen.findByRole("menuitem", { name: "编辑媒体库" }));
    const form = within(await screen.findByRole("dialog", { name: "编辑媒体库" }));
    await user.clear(form.getByLabelText("名称"));
    await user.type(form.getByLabelText("名称"), "家里的书架");
    await user.type(form.getByLabelText("追加文件夹"), "/srv/comics");
    await user.click(form.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(db.getLibrary(id)).toMatchObject({ name: "家里的书架", paths: ["/srv/books", "/srv/comics"] });
  });

  it("deletes a library after confirmation", async () => {
    await selectUploadedSkin();
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/system/libraries");

    const row = (await screen.findByText("/srv/books")).closest("[data-library-row]") as HTMLElement;
    await user.click(within(row).getByRole("button", { name: "更多" }));
    await user.click(await screen.findByRole("menuitem", { name: "删除" }));
    // Toasts are alert dialogs too; the confirmation is the one named after the library.
    const confirm = await screen.findByRole("alertdialog", { name: "删除媒体库「电子书」？" });
    expect(confirm).toHaveAttribute("data-secure-surface", "host");
    expect(within(confirm).getByText("只删除索引，磁盘上的文件不会被改动。")).toBeInTheDocument();
    server.use(http.delete("*/api/v1/libraries/:id", () => HttpResponse.json({
      error: { code: "conflict", message: "Review changed" },
    }, { status: 409 }), { once: true }));
    await user.click(within(confirm).getByRole("button", { name: "删除" }));
    expect(await within(confirm).findByRole("alert")).toHaveTextContent("Review changed");
    await user.click(within(confirm).getByRole("button", { name: "取消" }));
    await user.click(within(row).getByRole("button", { name: "更多" }));
    await user.click(await screen.findByRole("menuitem", { name: "删除" }));
    const reopened = await screen.findByRole("alertdialog", { name: "删除媒体库「电子书」？" });
    expect(within(reopened).queryByRole("alert")).toBeNull();
    await user.click(within(reopened).getByRole("button", { name: "删除" }));

    await waitFor(() => expect(screen.queryByText("/srv/books")).not.toBeInTheDocument());
    expect(db.libraries.some((library) => library.name === "电子书")).toBe(false);
  });
});
