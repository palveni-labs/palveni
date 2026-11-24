import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { PHOTOS_LIBRARY_ID } from "@/mocks/photos/state";
import { installMockDb } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";
import enLocale from "./locales/en.json";
import zhLocale from "./locales/zh-CN.json";

function flattenKeys(obj: unknown, prefix = ""): string[] {
  if (typeof obj !== "object" || obj === null) return [prefix];
  return Object.entries(obj).flatMap(([k, v]) => flattenKeys(v, prefix ? `${prefix}.${k}` : k));
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("photos module", () => {
  it("keeps en and zh-CN module locales in sync", () => {
    expect(flattenKeys(enLocale).sort()).toEqual(flattenKeys(zhLocale).sort());
  });

  it("links photo libraries from /library to /photos?libraryId=... when photos feature is active", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos"] });
    signIn(db, MOCK_ACCOUNTS.user.username);

    await renderLoadedApp("/library");

    const photoLinks = await screen.findAllByRole("link", { name: /家庭相册/ }, { timeout: 10_000 });
    expect(
      photoLinks.some((el) => el.getAttribute("href") === `/photos?libraryId=${PHOTOS_LIBRARY_ID}`),
    ).toBe(true);
  }, 15_000);

  it("renders day-grouped timeline, month jump scrubber, aspect toggle, multi-select and upload", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp("/photos");

    expect(await screen.findByText("共 4 张照片", {}, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "2026-11-18" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "2026-09-05" })).toBeInTheDocument();

    // Month jump scrubber filters by YYYY-MM
    const scrubber = screen.getByRole("navigation", { name: "按年月快速跳转" });
    await user.click(within(scrubber).getByRole("button", { name: /2026-09/ }));
    expect(await screen.findByText("共 1 张照片")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "DSC_1090" })).toBeInTheDocument();

    // Clear filter restores all 4 photos
    await user.click(screen.getByRole("button", { name: "清除筛选" }));
    expect(await screen.findByText("共 4 张照片")).toBeInTheDocument();

    // Square vs Aspect grid toggle
    const aspectBtn = screen.getAllByRole("button", { name: "原比例视图" })[0]!;
    await user.click(aspectBtn);
    await waitFor(() => expect(aspectBtn).toHaveAttribute("aria-pressed", "true"));

    // Multi-select mode -> select IMG_4822 -> batch favorite
    const selectBtn = screen.getAllByRole("button", { name: "多选" })[0]!;
    await user.click(selectBtn);
    await user.click(screen.getByRole("button", { name: "IMG_4822" }));
    expect(await screen.findByText("已选 1 张")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "批量收藏" }));
    await waitFor(() => expect(screen.queryByText("已选 1 张")).not.toBeInTheDocument());

    // Upload Photo modal
    const uploadBtn = screen.getAllByRole("button", { name: "上传照片" })[0]!;
    await user.click(uploadBtn);
    const dialog = await screen.findByRole("dialog", { name: "上传照片" });
    const fileInput = within(dialog).getByLabelText("选择图片文件");
    const samplePng = new File([new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13])], "trip_new.png", {
      type: "image/png",
    });
    await user.upload(fileInput, samplePng);
    await user.click(within(dialog).getByRole("button", { name: "开始上传" }));
    expect(await screen.findByText("共 5 张照片", {}, { timeout: 5000 })).toBeInTheDocument();
  });

  it("supports fullscreen lightbox keyboard navigation, zoom, slideshow and I-key EXIF drawer", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp("/photos");
    await screen.findByText("共 4 张照片", {}, { timeout: 5000 });

    await user.click(screen.getByRole("button", { name: "IMG_4821" }));
    const viewer = await screen.findByRole("dialog", { name: "照片查看器" });
    expect(within(viewer).getByText("1 / 4")).toBeInTheDocument();

    // Toggle zoom 1x -> 2x
    const zoomBtn = within(viewer).getByRole("button", { name: "2× 放大细节" });
    await user.click(zoomBtn);
    expect(within(viewer).getByRole("button", { name: "1× 适应窗口" })).toHaveAttribute("aria-pressed", "true");

    // Press 'i' key to open EXIF Info Drawer
    fireEvent.keyDown(window, { key: "i" });
    const drawer = await within(viewer).findByLabelText("照片信息");
    expect(await within(drawer).findByText("FE 35mm F1.4 GM")).toBeInTheDocument();
    expect(within(drawer).getByText("SONY ILCE-7M4")).toBeInTheDocument();
    expect(within(drawer).getByText("ISO 160")).toBeInTheDocument();
    expect(within(drawer).getByText("f/2.8")).toBeInTheDocument();
    expect(within(drawer).getByText("1/250s")).toBeInTheDocument();
    expect(within(drawer).getByText("35 mm")).toBeInTheDocument();
    expect(within(drawer).getByText("7008 × 4672")).toBeInTheDocument();
    expect(within(drawer).getByText("35.0094, 135.6668")).toBeInTheDocument();

    // ArrowRight navigates to next photo (IMG_4822)
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(await within(viewer).findByText("2 / 4")).toBeInTheDocument();
    expect(within(viewer).getAllByText("IMG_4822").length).toBeGreaterThan(0);

    // Space starts slideshow autoplay
    fireEvent.keyDown(window, { key: " " });
    expect(await within(viewer).findByText("幻灯片播放中")).toBeInTheDocument();

    // Escape closes viewer
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "照片查看器" })).not.toBeInTheDocument());
  });

  it("manages personal albums, removes photos and launches album slideshow", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp("/photos/albums");

    expect(await screen.findByText("京都秋行", {}, { timeout: 5000 })).toBeInTheDocument();

    // Create new album
    await user.click(screen.getAllByRole("button", { name: "新建相簿" })[0]!);
    const createDialog = await screen.findByRole("dialog", { name: "新建相簿" });
    await user.type(within(createDialog).getByLabelText("相簿名称"), "周末郊游");
    await user.click(within(createDialog).getByRole("button", { name: "保存" }));
    expect(await screen.findByText("周末郊游")).toBeInTheDocument();

    // Open 京都秋行 detail
    await user.click(screen.getByRole("link", { name: /京都秋行/ }));
    expect(await screen.findByRole("button", { name: "IMG_4821" }, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "IMG_4822" })).toBeInTheDocument();

    // Select & remove IMG_4822 from album
    await user.click(screen.getAllByRole("button", { name: "多选" })[0]!);
    await user.click(screen.getByRole("button", { name: "IMG_4822" }));
    await user.click(screen.getByRole("button", { name: "移出相簿" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "IMG_4822" })).not.toBeInTheDocument());

    // Launch slideshow directly from album header
    await user.click(screen.getAllByRole("button", { name: "播放幻灯片" })[0]!);
    const viewer = await screen.findByRole("dialog", { name: "照片查看器" });
    expect(within(viewer).getByText("幻灯片播放中")).toBeInTheDocument();
  }, 15_000);

  it("renders offline SVG world places map and filters photos by cluster", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp("/photos/places");
    const map = await screen.findByRole("img", { name: "世界经纬照片聚合地图" }, { timeout: 5000 });
    expect(map.tagName.toLowerCase()).toBe("svg");

    const kyotoPin = within(map).getByRole("button", { name: /35\.0°N, 135\.7°E/ });
    await user.click(kyotoPin);
    expect(await screen.findByRole("button", { name: "IMG_4821" })).toBeInTheDocument();
  });

  it("renders memories (on-this-day, highlights, cameras) and favorites wall", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "photos"] });
    signIn(db, MOCK_ACCOUNTS.user.username);

    const memoriesView = await renderLoadedApp("/photos/memories");
    const otdSection = await screen.findByRole("region", { name: "那年今日" }, { timeout: 5000 });
    expect(within(otdSection).getByRole("button", { name: "HARBOR_2025" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "近期精选" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "器材统计" })).toBeInTheDocument();
    memoriesView.unmount();

    await renderLoadedApp("/photos/favorites");
    expect(await screen.findByText("共 2 张收藏照片", {}, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "IMG_4821" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "DSC_1090" })).toBeInTheDocument();
  }, 15_000);
});
