import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { ALL_FEATURES, installMockDb } from "@/test/msw-server";
import { renderApp, selectUploadedSkin, signIn } from "@/test/render-app";

describe("feature-gated modules", () => {
  it("shows a 'not enabled' state instead of data when the server lacks the feature", async () => {
    const db = installMockDb(); // M0 feature set
    signIn(db, MOCK_ACCOUNTS.admin.username);
    renderApp("/assistant");
    expect(await screen.findByText("此功能尚未启用")).toBeInTheDocument();
    expect(screen.getByText("这台服务器（0.1.0-dev）还没有提供「助手」。")).toBeInTheDocument();
  });

  it("keeps provisional widgets off the M0 overview", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    renderApp("/");
    expect(await screen.findByRole("region", { name: "媒体库" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "服务器" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "助手建议" })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "家居" })).not.toBeInTheDocument();
  });

  it("runs an assistant request through preview, approval and completion", async () => {
    await selectUploadedSkin();
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/assistant");

    await user.click(await screen.findByText("发现 23 组重复照片，要整理吗？"));
    // Progress arrives as plain-language steps, results as blocks in the result pane.
    expect(await screen.findByText("找到 23 组重复，其中 2 组只是相似")).toBeInTheDocument();
    const canvas = await screen.findByRole("region", { name: "结果" });
    const confirm = await within(canvas).findByRole("button", { name: /移到回收站/ });
    expect(confirm).toHaveTextContent("21");
    expect(confirm.closest("[data-secure-surface=host]")).toBeInTheDocument();
    const [previewRow] = within(canvas).getAllByRole("checkbox");
    expect(previewRow?.closest("[data-secure-surface=host]")).toBeInTheDocument();

    await user.click(confirm);
    expect(await within(canvas).findByText(/已把 21 个重复文件移到回收站/)).toBeInTheDocument();
    expect(within(canvas).getByText("已确认")).toBeInTheDocument();
  });

  it("lists every kind of task and approves one from the drawer", async () => {
    await selectUploadedSkin();
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = renderApp("/tasks");

    // Waiting work leads, then running work, then what finished — every kind in one list.
    const waiting = await screen.findByRole("region", { name: "等你确认" }, { timeout: 5000 });
    const request = within(waiting).getByRole("listitem");
    expect(request).toHaveAttribute("data-secure-surface", "host");
    expect(within(request).getByText("等你确认")).toBeInTheDocument();
    expect(within(request).getByText("助手想为「无损音乐」补全 8 张专辑封面")).toBeInTheDocument();
    expect(within(request).getByRole("button", { name: "查看改动" })).toBeInTheDocument();
    expect(within(request).getByRole("button", { name: "确认" })).toBeInTheDocument();
    const running = screen.getByRole("region", { name: "正在进行" });
    expect(within(running).getByRole("button", { name: /Big Buck Bunny \(2008\)/ })).toBeInTheDocument();
    expect(within(running).getByRole("button", { name: /The Velvet Horizon \(2021\)/ })).toBeInTheDocument();
    const finished = screen.getByRole("region", { name: "已结束" });
    expect(within(finished).getByRole("button", { name: /扫描\s*电影/ })).toBeInTheDocument();
    expect(within(finished).getByRole("button", { name: /每晚 03:00 扫描全部媒体库/ })).toBeInTheDocument();
    expect(within(finished).getByText(/NVENC 不可用/)).toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /^等待确认\s*1$/ }));
    expect(screen.queryByRole("region", { name: "正在进行" })).not.toBeInTheDocument();
    await user.click(await screen.findByText("助手想为「无损音乐」补全 8 张专辑封面"));
    await waitFor(() => expect(router.state.location.search).toMatchObject({ filter: "awaiting", task: "t-covers" }));

    const drawer = await screen.findByRole("dialog");
    expect(within(drawer).getByText("在 MusicBrainz 查找封面")).toBeInTheDocument();
    await user.click(within(drawer).getByRole("button", { name: /写入封面/ }));
    expect(await within(drawer).findByText("封面已写入，客户端下次同步时会显示。")).toBeInTheDocument();
    expect(within(drawer).getByText("已确认")).toBeInTheDocument();
  }, 20_000);

  it("configures assistant model preset and manages visible memories", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/assistant");

    await user.click(await screen.findByRole("button", { name: "模型与记忆" }));
    const drawer = await screen.findByRole("dialog", { name: "模型与记忆" });

    const presetSelect = await within(drawer).findByLabelText("服务商预设");
    await user.selectOptions(presetSelect, "ollama");
    await user.click(within(drawer).getByRole("button", { name: "保存模型配置" }));

    const memInput = within(drawer).getByPlaceholderText(/新增偏好/);
    await user.type(memInput, "偏好 4K 原盘与简体字幕");
    await user.click(within(drawer).getByRole("button", { name: "添加" }));
    expect(await within(drawer).findByText("偏好 4K 原盘与简体字幕")).toBeInTheDocument();

    await user.click(within(drawer).getByRole("button", { name: "删除记忆" }));
    await waitFor(() => expect(within(drawer).queryByText("偏好 4K 原盘与简体字幕")).not.toBeInTheDocument());
  });

  it("manages Home Assistant bridges and custom devices from the home drawer", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/home");

    await user.click(await screen.findByRole("button", { name: "桥接与管理" }));
    const drawer = await screen.findByRole("dialog", { name: "桥接与管理" });

    await user.type(within(drawer).getByPlaceholderText("http://192.168.1.10:8123"), "http://192.168.1.50:8123");
    await user.type(within(drawer).getByPlaceholderText("eyJ..."), "ha-token-123");
    await user.click(within(drawer).getByRole("button", { name: "连接并保存" }));

    expect(await within(drawer).findByText("http://192.168.1.50:8123")).toBeInTheDocument();
    await user.click(within(drawer).getByRole("button", { name: "同步设备" }));

    await user.type(within(drawer).getByPlaceholderText("落地灯"), "阅读落地灯");
    await user.click(within(drawer).getByRole("button", { name: "添加设备" }));
  });
});
