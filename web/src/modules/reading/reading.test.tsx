import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import {
  BOOKS_LIBRARY_ID,
  SAMPLE_CBR_ID,
  SAMPLE_COMIC_1_ID,
  SAMPLE_EPUB_ID,
  SAMPLE_PDF_ID,
} from "@/mocks/reading/state";
import { installMockDb } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";
import enLocale from "./locales/en.json";
import zhLocale from "./locales/zh-CN.json";

function flattenKeys(obj: unknown, prefix = ""): string[] {
  if (typeof obj !== "object" || obj === null) return [prefix];
  return Object.entries(obj).flatMap(([k, v]) => flattenKeys(v, prefix ? `${prefix}.${k}` : k));
}

afterEach(() => {
  localStorage.removeItem("palveni.reading.typography");
  localStorage.removeItem("palveni.reading.comicPrefs");
  vi.restoreAllMocks();
});

describe("reading module", () => {
  it("keeps en and zh-CN module locales in sync", () => {
    expect(flattenKeys(enLocale).sort()).toEqual(flattenKeys(zhLocale).sort());
  });

  it("renders reading home shelves and series links", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "reading"] });
    signIn(db, MOCK_ACCOUNTS.user.username);

    await renderLoadedApp("/reading");
    expect(await screen.findByRole("region", { name: "继续阅读" }, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "最新书籍" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "最新漫画" })).toBeInTheDocument();

    const seriesRegion = screen.getByRole("region", { name: "系列合集" });
    expect(within(seriesRegion).getByRole("link", { name: /北湾手记/ })).toBeInTheDocument();
    expect(within(seriesRegion).getByRole("link", { name: /云梯集市/ })).toBeInTheDocument();
  });

  it("routes book and comic libraries from /library to /reading while keeping Files view", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "reading"] });
    signIn(db, MOCK_ACCOUNTS.user.username);

    await renderLoadedApp("/library");
    expect(await screen.findByRole("link", { name: "电子书 · 文件" }, { timeout: 5000 })).toHaveAttribute(
      "href",
      `/library/${BOOKS_LIBRARY_ID}?view=files`,
    );
    const bookLinks = screen
      .getAllByRole("link")
      .filter((el) => el.getAttribute("href") === `/reading?library=${BOOKS_LIBRARY_ID}`);
    expect(bookLinks.length).toBeGreaterThan(0);
  });

  it("filters books shelf by format, favorites and search", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "reading"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    const shelf = await renderLoadedApp("/reading/books");
    expect(await screen.findByText("共 4 本", {}, { timeout: 5000 })).toBeInTheDocument();

    // Filter by PDF format -> only 设计系统手册
    await user.selectOptions(screen.getByRole("combobox", { name: "全部格式" }), "pdf");
    expect(await screen.findByText("共 1 本")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "设计系统手册" })).toBeInTheDocument();
    shelf.unmount();

    // Favorites shelf includes seeded favorite book + comic
    const favs = await renderLoadedApp("/reading/favorites");
    expect(await screen.findByText("共 2 本", {}, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "雾灯" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "云梯集市 第1卷" })).toBeInTheDocument();
    favs.unmount();

    // Search shelf
    await renderLoadedApp("/reading/search?q=巡岸");
    expect(await screen.findByRole("link", { name: "白礁航线" }, { timeout: 5000 })).toBeInTheDocument();
  });

  it("drives ebook reader TOC, typography themes, keyboard chapter turn, bookmarks and inline PDF viewer", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "reading"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    const ebook = await renderLoadedApp(`/reading/book/${SAMPLE_EPUB_ID}`);
    // Resumes on saved chapter index 1 ("第二章 潮汐图")
    expect(await screen.findByRole("heading", { level: 2, name: "第二章 潮汐图" }, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByText(/两段潮汐恰好重合/)).toBeInTheDocument();

    // Open TOC drawer and jump to chapter 0 ("第一章 夜灯初亮")
    await user.click(screen.getByRole("button", { name: "目录" }));
    const tocDialog = await screen.findByRole("dialog", { name: "目录" });
    await user.click(within(tocDialog).getByRole("button", { name: /第一章 夜灯初亮/ }));
    expect(await screen.findByRole("heading", { level: 2, name: "第一章 夜灯初亮" })).toBeInTheDocument();

    // Keyboard ArrowRight advances to chapter 1
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(await screen.findByRole("heading", { level: 2, name: "第二章 潮汐图" })).toBeInTheDocument();

    // Typography drawer: switch to Warm Sepia theme
    await user.click(screen.getByRole("button", { name: "排版与主题" }));
    const typoDialog = await screen.findByRole("dialog", { name: "排版与主题" });
    await user.click(within(typoDialog).getByRole("button", { name: "护眼暖色" }));
    expect(within(typoDialog).getByRole("button", { name: "护眼暖色" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    // Bookmarks drawer: add a new bookmark and delete it
    await user.click(screen.getByRole("button", { name: /书签/ }));
    const bmDialog = await screen.findByRole("dialog", { name: "书签" });
    await user.type(
      within(bmDialog).getByPlaceholderText("书签标题（默认当前章节）"),
      "阿岑的潮汐记录",
    );
    await user.click(within(bmDialog).getByRole("button", { name: "保存书签" }));
    expect(await within(bmDialog).findByText("阿岑的潮汐记录")).toBeInTheDocument();

    await user.click(
      within(bmDialog).getByRole("button", { name: "删除书签 阿岑的潮汐记录" }),
    );
    await waitFor(() =>
      expect(within(bmDialog).queryByText("阿岑的潮汐记录")).not.toBeInTheDocument(),
    );
    ebook.unmount();

    // Inline PDF reader uses signed `/reading-inline?ticket=...` URL inside iframe
    await renderLoadedApp(`/reading/book/${SAMPLE_PDF_ID}`);
    const iframe = await screen.findByTitle(/设计系统手册 · PDF 文档阅读器/, {}, { timeout: 5000 });
    expect(iframe).toHaveAttribute(
      "src",
      `/api/v1/items/${SAMPLE_PDF_ID}/reading-inline?ticket=mock-pdf-ticket`,
    );
  });

  it("drives dark comic reader across single, spread & webtoon modes, RTL/LTR keyboard nav, scrubber and 415 fallback", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "reading"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    const comic = await renderLoadedApp(`/reading/comic/${SAMPLE_COMIC_1_ID}`);
    // Resumes on saved page 1 (displayed 1-based as "2 / 6")
    expect(await screen.findByTestId("comic-page-counter", {}, { timeout: 5000 })).toHaveTextContent(
      "2 / 6",
    );
    const stage = screen.getByTestId("comic-reader-stage");
    expect(stage).toHaveAttribute("data-direction", "ltr");
    expect(stage).toHaveAttribute("data-view-mode", "single");

    // In LTR mode, ArrowRight advances to page 3 / 6
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(await screen.findByTestId("comic-page-counter")).toHaveTextContent("3 / 6");

    // Switch to Manga RTL mode -> ArrowLeft advances to page 4 / 6!
    await user.click(screen.getByRole("button", { name: "右翻 LTR" }));
    expect(stage).toHaveAttribute("data-direction", "rtl");
    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(await screen.findByTestId("comic-page-counter")).toHaveTextContent("4 / 6");

    // Switch to Double-page spread mode
    await user.click(screen.getByRole("button", { name: "双页拼合" }));
    expect(await screen.findByTestId("comic-spread-pair")).toBeInTheDocument();

    // Switch to Vertical Webtoon mode
    await user.click(screen.getByRole("button", { name: "条漫卷轴" }));
    expect(await screen.findByTestId("comic-webtoon-scroll")).toBeInTheDocument();

    // Jump with bottom page scrubber
    fireEvent.change(screen.getByRole("slider", { name: "跳转页码" }), {
      target: { value: "6" },
    });
    expect(await screen.findByTestId("comic-page-counter")).toHaveTextContent("6 / 6");
    comic.unmount();

    // Unsupported CBR archive displays clear offline/download notice
    await renderLoadedApp(`/reading/comic/${SAMPLE_CBR_ID}`);
    expect(
      await screen.findByRole(
        "heading",
        { name: "该漫画压缩格式暂不支持在线逐页流式解码" },
        { timeout: 5000 },
      ),
    ).toBeInTheDocument();
  });
});
