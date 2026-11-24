import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { ALL_FEATURES, installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("MCP access, external downloaders, RSS subscriptions, and reversible media organizer", () => {
  it("renders downloads queue, configures a downloader client, submits a Magnet task, and checks RSS feeds", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/tasks/downloads");

    expect(await screen.findByText("Dune.Part.Two.2024.2160p.WEB-DL.HEVC.mkv")).toBeInTheDocument();
    expect(screen.getByText("NAS qBittorrent")).toBeInTheDocument();

    // Configure an additional Transmission client
    await user.click(screen.getByRole("button", { name: "配置下载器" }));
    const drawer = await screen.findByRole("dialog", { name: "配置下载器" });
    await user.selectOptions(within(drawer).getByLabelText("类型"), "transmission");
    await user.type(within(drawer).getByPlaceholderText("NAS qBittorrent"), "书房 Transmission");
    await user.click(within(drawer).getByRole("button", { name: "保存下载器" }));
    expect((await screen.findAllByText("书房 Transmission")).length).toBeGreaterThanOrEqual(1);

    // Submit a Magnet URI task
    const magnetInput = screen.getByPlaceholderText(/粘贴 Magnet 磁力链接/);
    await user.type(magnetInput, "magnet:?xt=urn:btih:99887766&dn=Severance.S02E01.2160p.mkv");
    await user.click(screen.getByRole("button", { name: "投递下载" }));
    expect(await screen.findByText("Severance.S02E01.2160p.mkv")).toBeInTheDocument();

    // Trigger 1-click RSS check on seeded subscription, then 1-click push matched item to downloader
    await user.click(screen.getByRole("button", { name: "立即检查" }));
    const matchedTitle = await screen.findByText("[Feed] 三体 S01E06 2160p HEVC");
    expect(matchedTitle).toBeInTheDocument();
    const matchRow = matchedTitle.closest("li")!;
    await user.click(within(matchRow).getByRole("button", { name: "投递下载" }));
    expect(await within(matchRow).findByRole("button", { name: "已投递" })).toBeDisabled();
  }, 15_000);

  it("previews naming templates, applies selected renames, and performs 1-click undo", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/tasks/organizer");

    const previewBtn = await screen.findByRole("button", { name: "生成变更预览" });
    await waitFor(() => expect(previewBtn).toBeEnabled());
    await user.click(previewBtn);

    const applyBtn = await screen.findByRole("button", { name: /执行选中项/ });
    expect(applyBtn).toBeEnabled();
    await user.click(applyBtn);

    const undoBtn = await screen.findByRole("button", { name: "一键撤销" });
    await user.click(undoBtn);

    expect(await screen.findByText("已撤销")).toBeInTheDocument();
  }, 15_000);

  it("exposes MCP client JSON configurations and cloud path redaction policy in assistant settings", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/assistant");

    await user.click(await screen.findByRole("button", { name: "模型与记忆" }));
    const drawer = await screen.findByRole("dialog", { name: "模型与记忆" });

    expect(await within(drawer).findByText("MCP 外部接入 (Claude / Cursor / OpenClaw)")).toBeInTheDocument();
    expect(await within(drawer).findByText("/api/v1/mcp")).toBeInTheDocument();
    expect(within(drawer).getByText(/MCP 2025-03-26/)).toBeInTheDocument();

    await user.click(within(drawer).getByRole("tab", { name: "OpenClaw" }));
    expect(within(drawer).getByText(/"streamable-http"/)).toBeInTheDocument();
    await user.click(within(drawer).getByRole("button", { name: "复制 JSON 配置" }));

    const redactToggle = within(drawer).getByRole("checkbox", { name: /云端路径脱敏保护/ });
    expect(redactToggle).toBeChecked();
    await user.click(redactToggle);
    expect(redactToggle).not.toBeChecked();
    await user.click(within(drawer).getByRole("button", { name: "保存模型配置" }));
  }, 15_000);
});
