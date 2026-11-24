import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";

describe("photos local AI: smart search, people and duplicate cleanup", () => {
  it("parses bilingual natural-language smart queries, shows filter chips and runs local analysis", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos", "photoAi"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp("/photos");
    expect(await screen.findByText("共 4 张照片", {}, { timeout: 5000 })).toBeInTheDocument();

    // Trigger local smart analysis button
    await user.click(screen.getByRole("button", { name: "运行本地智能分析" }));
    expect(await screen.findByText(/累计已标记 4 张/)).toBeInTheDocument();

    // Click smart suggestion chip "去年秋天的夜景"
    await user.click(screen.getByRole("button", { name: "去年秋天的夜景" }));
    const parsed = await screen.findByRole("region", { name: "已识别筛选条件" });
    expect(within(parsed).getByText("秋季")).toBeInTheDocument();
    expect(within(parsed).getByText("夜景")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "HARBOR_2025" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "DSC_1090" })).not.toBeInTheDocument();

    // Search "蓝色天空" matches blue photos
    const searchInput = screen.getByRole("searchbox");
    await user.clear(searchInput);
    await user.type(searchInput, "蓝色天空");
    const blueChips = await screen.findByRole("region", { name: "已识别筛选条件" });
    expect(within(blueChips).getByText("蓝色调")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "IMG_4821" })).toBeInTheDocument();
  }, 15_000);

  it("manages people profiles on /photos/people and supports quick person tagging inside lightbox", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos", "photoAi"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp("/photos/people");
    expect(await screen.findByText("小明", {}, { timeout: 5000 })).toBeInTheDocument();

    // Create a new person
    await user.click(screen.getByRole("button", { name: "新建人物" }));
    const createDialog = await screen.findByRole("dialog", { name: "新建人物档案" });
    await user.type(within(createDialog).getByLabelText("人物姓名"), "小红");
    await user.click(within(createDialog).getByRole("button", { name: "保存" }));
    expect(await screen.findByText("小红")).toBeInTheDocument();

    // Open 小明's photos
    await user.click(screen.getByRole("button", { name: /小明.*1 张照片/ }));
    expect(await screen.findByRole("button", { name: "IMG_4821" })).toBeInTheDocument();

    // Open Lightbox and tag 小红 directly from Lightbox
    await user.click(screen.getByRole("button", { name: "IMG_4821" }));
    const viewer = await screen.findByRole("dialog", { name: "照片查看器" });
    await user.click(within(viewer).getByRole("button", { name: "标注人物" }));
    const tagPanel = await within(viewer).findByRole("complementary", { name: "标注人物" });
    const hongBtn = await within(tagPanel).findByRole("button", { name: "小红" });
    expect(hongBtn).toHaveAttribute("aria-pressed", "false");
    await user.click(hongBtn);
    await waitFor(() => expect(hongBtn).toHaveAttribute("aria-pressed", "true"));
  }, 15_000);

  it("displays similar dHash duplicate clusters on /photos/cleanup with keep badge and safe staging", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos", "photoAi"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp("/photos/cleanup");
    expect(await screen.findByText("推荐保留", {}, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByText(/相似画面/)).toBeInTheDocument();
    expect(screen.getByText("IMG_4821.JPG")).toBeInTheDocument();
    expect(screen.getByText("IMG_4822.JPG")).toBeInTheDocument();

    // Mark duplicate for safe review and stage into Cleanup album
    await user.click(screen.getByRole("button", { name: "标记待清理" }));
    expect(await screen.findByText("已标记 1 张待清理照片")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "移入待清理相簿" }));
    expect(await screen.findByText(/已将 1 张照片安全归集至相簿/)).toBeInTheDocument();

    // Navigate via shell sidebar back to Timeline
    const [nav] = screen.getAllByRole("navigation", { name: "导航" });
    await user.click(within(nav!).getByRole("link", { name: "时间线" }));
    expect(await screen.findByText("共 4 张照片")).toBeInTheDocument();
  }, 15_000);
});
