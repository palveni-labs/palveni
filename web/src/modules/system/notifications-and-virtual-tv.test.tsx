import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import broadcastEn from "@/modules/broadcast/locales/en.json";
import broadcastZh from "@/modules/broadcast/locales/zh-CN.json";
import { installMockDb } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";

function flattenKeys(obj: unknown, prefix = ""): string[] {
  if (typeof obj !== "object" || obj === null) return [prefix];
  return Object.entries(obj).flatMap(([k, v]) => flattenKeys(v, prefix ? `${prefix}.${k}` : k));
}

describe("notifications, routines, inbound webhooks, and virtual live TV", () => {
  it("keeps broadcast en and zh-CN locales in sync", () => {
    expect(flattenKeys(broadcastEn).sort()).toEqual(flattenKeys(broadcastZh).sort());
  });

  it("manages inbox notifications, external push channels with 1-click test, routines, and one-time inbound webhook URLs", async () => {
    const db = installMockDb({
      features: [...M0_FEATURES, "notifications", "routines"],
    });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();

    await renderLoadedApp("/system/notifications");

    // 1. Inbox displays seeded unread notification and supports mark-all-read
    expect(await screen.findByText("媒体库扫描完成：电影", {}, { timeout: 5000 })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "全部标为已读" }));
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "全部标为已读" })).toBeNull(),
    );

    // 2. Add a Feishu push channel with quiet hours and trigger 1-click test
    await user.click(screen.getByRole("button", { name: "添加推送渠道" }));
    await user.type(screen.getByLabelText("渠道名称"), "客厅飞书通知");
    await user.selectOptions(screen.getByLabelText("推送类型"), "feishu");
    await user.type(
      screen.getByLabelText("接口或 Webhook 地址"),
      "https://open.feishu.cn/open-apis/bot/v2/hook/demo",
    );
    await user.type(screen.getByLabelText("免打扰时段"), "23:00-07:00");
    await user.click(screen.getByRole("button", { name: "创建" }));

    expect(await screen.findByText("客厅飞书通知")).toBeVisible();
    expect(screen.getByText("免打扰 23:00-07:00")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "测试推送" }));
    expect(await screen.findByText(/通知渠道「客厅飞书通知」测试消息发送成功/)).toBeVisible();

    // 3. Run scheduled routine immediately ("立即执行")
    expect(screen.getByText("凌晨媒体巡检")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "立即执行" }));
    expect(await screen.findByText("succeeded")).toBeVisible();

    // 4. Create Sonarr inbound webhook and verify one-time copyable trigger URL
    await user.click(screen.getByRole("button", { name: "新建入站令牌" }));
    await user.type(screen.getByLabelText("来源名称"), "Sonarr 自动入库");
    await user.selectOptions(screen.getByLabelText("自动化工具"), "sonarr");
    await user.click(screen.getByRole("button", { name: "创建" }));

    const urlInput = await screen.findByRole<HTMLInputElement>("textbox", {
      name: "一次性回调地址（请立即复制保存）",
    });
    expect(urlInput.value).toMatch(/\/api\/v1\/webhooks\/inbound\/pwh_[a-f0-9]+\/trigger$/);
    await user.click(screen.getByRole("button", { name: "完成" }));
    expect(
      screen.queryByRole("textbox", { name: "一次性回调地址（请立即复制保存）" }),
    ).toBeNull();
    expect(screen.getByText("Sonarr 自动入库")).toBeVisible();
  });

  it("renders 24x7 Virtual Live TV guide with live progress, EPG schedule, instant tune-in link, M3U/XMLTV feeds, and channel creation modal", async () => {
    const db = installMockDb({
      features: [...M0_FEATURES, "broadcast", "virtualTv", "video"],
    });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();

    await renderLoadedApp("/broadcast/virtual");

    expect(
      await screen.findByRole("region", { name: /CH 101 · 家庭科幻影院/ }, { timeout: 5000 }),
    ).toBeInTheDocument();

    // Live airing title, upcoming EPG, progressbar, and copyable M3U / XMLTV buttons
    expect(screen.getByText("天鹅绒地平线")).toBeVisible();
    expect(screen.getByText("深海回响")).toBeVisible();
    expect(screen.getByRole("progressbar", { name: /家庭科幻影院/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "复制 M3U 列表地址" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "复制 XMLTV 节目单地址" })).toBeEnabled();

    // Instant tune-in link points to video player with exact itemId, libraryId, and live startMs offset
    const tuneLink = screen.getByRole("link", { name: "立即调台观看" });
    const href = tuneLink.getAttribute("href") ?? "";
    expect(href).toContain("/video/play/23587a91-9e35-4821-8a48-3aeddba77d11");
    expect(href).toContain("libraryId=");
    expect(href).toContain("startMs=");

    // Admin modal creates a new virtual channel
    await user.click(screen.getByRole("button", { name: "新建频道" }));
    const dialog = await screen.findByRole("dialog", { name: "新建频道" });
    await user.type(within(dialog).getByLabelText("频道名称"), "经典剧情轮播台");
    await user.click(within(dialog).getByRole("button", { name: "创建频道" }));

    expect(await screen.findByRole("region", { name: /CH 102 · 经典剧情轮播台/ })).toBeInTheDocument();
  });
});
