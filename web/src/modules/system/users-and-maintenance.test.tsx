import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ALL_FEATURES, installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("users, account security and maintenance", () => {
  it("creates a restricted family member, edits library ACLs, resets password and deletes", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, "admin");
    renderApp("/system/users");
    const user = userEvent.setup();

    expect(await screen.findByText("family", {}, { timeout: 5000 })).toBeInTheDocument();

    // Create new restricted family member
    await user.click(screen.getByRole("button", { name: "添加成员" }));
    const createDialog = await screen.findByRole("dialog");
    await user.type(within(createDialog).getByLabelText("用户名"), "kid");
    await user.type(within(createDialog).getByLabelText("密码"), "kidpass123");
    await user.click(within(createDialog).getByRole("switch", { name: /允许访问全部媒体库/ }));
    await user.click(within(createDialog).getByRole("checkbox", { name: /无损音乐/ }));
    await user.click(within(createDialog).getByRole("button", { name: "创建" }));

    await waitFor(() => expect(db.users.some((u) => u.username === "kid")).toBe(true));
    const kidStored = db.users.find((u) => u.username === "kid")!;
    expect(kidStored.policy.allLibraries).toBe(false);
    expect(kidStored.allowedLibraryIds).toHaveLength(1);
    expect(await screen.findByText("仅限 1 个媒体库")).toBeInTheDocument();

    // Reset password for kid
    await user.click(screen.getByRole("button", { name: "重置密码 kid" }));
    const resetDialog = await screen.findByRole("dialog");
    await user.type(within(resetDialog).getByLabelText("新密码"), "newkidpass99");
    await user.click(within(resetDialog).getByRole("button", { name: "保存" }));
    await waitFor(() => expect(db.users.find((u) => u.username === "kid")?.password).toBe("newkidpass99"));

    // Delete kid
    await user.click(screen.getByRole("button", { name: "删除 kid" }));
    const confirmDialog = await screen.findByRole("alertdialog", { name: /删除账号/ });
    await user.click(within(confirmDialog).getByRole("button", { name: "删除" }));
    await waitFor(() => expect(db.users.some((u) => u.username === "kid")).toBe(false));
  });

  it("lets a signed-in member view active sessions, revoke another device, and change password", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    const familyUser = db.users.find((u) => u.username === "family")!;
    db.issueToken(familyUser, "卧室 iPad");
    signIn(db, "family");
    renderApp("/system/settings");
    const user = userEvent.setup();

    expect(await screen.findByText("当前会话", {}, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByText("卧室 iPad")).toBeInTheDocument();

    // Revoke the iPad session
    await user.click(screen.getByRole("button", { name: "下线" }));
    await waitFor(() => expect(screen.queryByText("卧室 iPad")).not.toBeInTheDocument());

    // Change password
    await user.click(screen.getByRole("button", { name: "修改密码" }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText("当前密码"), "family1234");
    await user.type(within(dialog).getByLabelText("新密码"), "family9999");
    await user.type(within(dialog).getByLabelText("确认新密码"), "family9999");
    await user.click(within(dialog).getByRole("button", { name: "保存" }));

    await waitFor(() => expect(db.users.find((u) => u.username === "family")?.password).toBe("family9999"));
  });

  it("creates and restores portable snapshots, previews/imports Jellyfin history, and filters audit logs", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, "admin");
    renderApp("/system/maintenance");
    const user = userEvent.setup();

    // 1. Create snapshot backup, mutate libraries, and restore
    const labelInput = await screen.findByPlaceholderText(/快照备注/, {}, { timeout: 5000 });
    await user.type(labelInput, "升级前快照");
    await user.click(screen.getByRole("button", { name: "创建快照" }));
    expect(await screen.findByText("升级前快照")).toBeInTheDocument();

    const originalLibCount = db.libraries.length;
    db.libraries = db.libraries.slice(0, 1);
    expect(db.libraries).toHaveLength(1);

    await user.click(screen.getByRole("button", { name: "恢复" }));
    await user.click(await screen.findByRole("button", { name: "确认恢复" }));
    await waitFor(() => expect(db.libraries).toHaveLength(originalLibCount));

    // 2. Dry-run preview and apply external Jellyfin history import
    const sampleItem = db.allLeafItems()[0]!;
    const payload = JSON.stringify({
      Items: [
        {
          Name: sampleItem.title,
          Path: sampleItem.path,
          UserData: { IsFavorite: true, Played: true, PlayCount: 2 },
        },
        {
          Name: "Unknown Work",
          Path: "/nowhere/unknown.mkv",
          UserData: { Played: true },
        },
      ],
    });
    const textarea = screen.getByLabelText("导入内容");
    await user.clear(textarea);
    await user.paste(payload);
    await user.click(screen.getByRole("button", { name: "预览匹配" }));
    expect(await screen.findByText(/共 2 条 · 已匹配 1 · 未匹配 1/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "确认导入" }));
    await waitFor(() => expect(db.auditLogs.some((entry) => entry.action === "system.imported")).toBe(true));

    // 3. Filter security audit log by system. prefix
    const filterSelect = screen.getByRole("combobox", { name: "筛选操作前缀" });
    await user.selectOptions(filterSelect, "system.");
    expect(await screen.findByText("system.backup_created")).toBeInTheDocument();
    expect(screen.getByText("system.imported")).toBeInTheDocument();
    expect(screen.queryByText("setup.completed")).not.toBeInTheDocument();
  });
});
