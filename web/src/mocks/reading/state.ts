/**
 * Deterministic mock reading catalog (books + comics, chapters, pages, progress, bookmarks).
 */
import { stableUuid } from "../fixtures";
import type {
  CreateBookmarkRequest,
  ReadingBookmark,
  ReadingChapter,
  ReadingEntry,
  ReadingHome,
  ReadingKind,
  ReadingManifest,
  ReadingPage,
  ReadingProgressUpdate,
  ReadingReaderMode,
  ReadingSeries,
  ReadingUserState,
} from "@/modules/reading/api";

export const BOOKS_LIBRARY_ID = stableUuid("seed-library:books");
export const COMICS_LIBRARY_ID = stableUuid("seed-library:comics");

export const SAMPLE_EPUB_ID = stableUuid("seed-reading:three-body-1");
export const SAMPLE_EPUB_2_ID = stableUuid("seed-reading:three-body-2");
export const SAMPLE_PDF_ID = stableUuid("seed-reading:design-pdf");
export const SAMPLE_MD_ID = stableUuid("seed-reading:arch-notes");
export const SAMPLE_COMIC_1_ID = stableUuid("seed-reading:dungeon-meshi-1");
export const SAMPLE_COMIC_2_ID = stableUuid("seed-reading:dungeon-meshi-2");
export const SAMPLE_CBR_ID = stableUuid("seed-reading:legacy-cbr");

interface StoredBookRecord {
  entry: ReadingEntry;
  chapters: ReadingChapter[];
}

export class ReadingMockState {
  private readonly records = new Map<string, StoredBookRecord>();
  private readonly bookmarksByItem = new Map<string, ReadingBookmark[]>();
  private bookmarkSeq = 1;
  private readonly now: () => number;

  constructor(now: () => number) {
    this.now = now;
    this.seed();
  }

  private iso(offsetMs = 0): string {
    return new Date(this.now() + offsetMs).toISOString().replace(/\.\d{3}Z$/, "Z");
  }

  private seed() {
    const DAY = 86_400_000;
    const items: StoredBookRecord[] = [
      {
        entry: {
          itemId: SAMPLE_EPUB_ID,
          libraryId: BOOKS_LIBRARY_ID,
          kind: "book",
          format: "epub",
          title: "三体",
          authors: ["刘慈欣"],
          seriesName: "地球往事",
          seriesKey: "地球往事",
          seriesIndex: 1,
          summary: "红岸基地与三体文明的首次接触。",
          publisher: "重庆出版社",
          language: "zh-CN",
          pageCount: 3,
          hasCover: true,
          tags: ["科幻", "硬科幻"],
          sizeBytes: 980_000,
          addedAt: this.iso(-5 * DAY),
          state: {
            page: 1,
            totalPages: 3,
            locator: "chapter:1",
            completed: false,
            favorite: true,
            lastReadAt: this.iso(-3600_000),
          },
        },
        chapters: [
          {
            index: 0,
            title: "第一章 科学边界",
            html: "<p>汪淼觉得眼前的一切都失去了意义。</p><img src=\"/api/v1/items/" + SAMPLE_EPUB_ID + "/reading-asset?path=OEBPS/Images/radar.png\" data-reading-asset=\"OEBPS/Images/radar.png\" alt=\"红岸雷达峰\" />",
            plainText: "汪淼觉得眼前的一切都失去了意义。",
          },
          {
            index: 1,
            title: "第二章 台球",
            html: "<p>丁仪指着台球桌说：物理规律在时间和空间上是均匀的。</p>",
            plainText: "丁仪指着台球桌说：物理规律在时间和空间上是均匀的。",
          },
          {
            index: 2,
            title: "第三章 射手与农场主",
            html: "<p>宇宙闪烁倒计时在视网膜上浮现。</p>",
            plainText: "宇宙闪烁倒计时在视网膜上浮现。",
          },
        ],
      },
      {
        entry: {
          itemId: SAMPLE_EPUB_2_ID,
          libraryId: BOOKS_LIBRARY_ID,
          kind: "book",
          format: "epub",
          title: "黑暗森林",
          authors: ["刘慈欣"],
          seriesName: "地球往事",
          seriesKey: "地球往事",
          seriesIndex: 2,
          summary: "面壁计划与宇宙社会学两条公理。",
          publisher: "重庆出版社",
          language: "zh-CN",
          pageCount: 2,
          hasCover: true,
          tags: ["科幻"],
          sizeBytes: 1_240_000,
          addedAt: this.iso(-4 * DAY),
          state: {
            page: 0,
            totalPages: 2,
            locator: null,
            completed: false,
            favorite: false,
            lastReadAt: null,
          },
        },
        chapters: [
          {
            index: 0,
            title: "序曲 褐蚁与墓碑",
            html: "<p>罗辑与叶文洁在雪地墓碑前的对话。</p>",
            plainText: "罗辑与叶文洁在雪地墓碑前的对话。",
          },
          {
            index: 1,
            title: "第一部 面壁者",
            html: "<p>联合国行星防御理事会宣布面壁计划正式启动。</p>",
            plainText: "联合国行星防御理事会宣布面壁计划正式启动。",
          },
        ],
      },
      {
        entry: {
          itemId: SAMPLE_PDF_ID,
          libraryId: BOOKS_LIBRARY_ID,
          kind: "book",
          format: "pdf",
          title: "设计系统手册",
          authors: ["陈默"],
          seriesName: null,
          seriesKey: null,
          seriesIndex: null,
          summary: "网格、字体排印与高对比无障碍规范。",
          publisher: "工坊出版",
          language: "zh-CN",
          pageCount: 128,
          hasCover: false,
          tags: ["设计", "排版"],
          sizeBytes: 6_400_000,
          addedAt: this.iso(-2 * DAY),
          state: {
            page: 14,
            totalPages: 128,
            locator: "pdf:14",
            completed: false,
            favorite: false,
            lastReadAt: this.iso(-7200_000),
          },
        },
        chapters: [],
      },
      {
        entry: {
          itemId: SAMPLE_MD_ID,
          libraryId: BOOKS_LIBRARY_ID,
          kind: "book",
          format: "md",
          title: "家庭服务器架构笔记",
          authors: ["林舟"],
          seriesName: null,
          seriesKey: null,
          seriesIndex: null,
          summary: "离线 NAS 存储、增量扫描与多端同步备忘。",
          publisher: null,
          language: "zh-CN",
          pageCount: 2,
          hasCover: false,
          tags: ["技术"],
          sizeBytes: 32_000,
          addedAt: this.iso(-1 * DAY),
          state: {
            page: 1,
            totalPages: 2,
            locator: "chapter:1",
            completed: true,
            favorite: false,
            lastReadAt: this.iso(-12 * 3600_000),
          },
        },
        chapters: [
          {
            index: 0,
            title: "存储拓扑",
            html: "<h1>存储拓扑</h1><p>主阵列与离线冷备盘独立挂载。</p>",
            plainText: "存储拓扑\n\n主阵列与离线冷备盘独立挂载。",
          },
          {
            index: 1,
            title: "增量校验",
            html: "<h2>增量校验</h2><p>按文件体积与修改时间戳增量索引。</p>",
            plainText: "增量校验\n\n按文件体积与修改时间戳增量索引。",
          },
        ],
      },
      {
        entry: {
          itemId: SAMPLE_COMIC_1_ID,
          libraryId: COMICS_LIBRARY_ID,
          kind: "comic",
          format: "cbz",
          title: "迷宫饭 第1卷",
          authors: ["九井谅子"],
          seriesName: "迷宫饭",
          seriesKey: "迷宫饭",
          seriesIndex: 1,
          summary: "在地下城烹饪走路菇与蝎子锅的奇幻冒险。",
          publisher: null,
          language: "zh-CN",
          pageCount: 6,
          hasCover: true,
          tags: ["奇幻", "美食"],
          sizeBytes: 42_000_000,
          addedAt: this.iso(-3 * DAY),
          state: {
            page: 1,
            totalPages: 6,
            locator: "page:1",
            completed: false,
            favorite: true,
            lastReadAt: this.iso(-1800_000),
          },
        },
        chapters: [],
      },
      {
        entry: {
          itemId: SAMPLE_COMIC_2_ID,
          libraryId: COMICS_LIBRARY_ID,
          kind: "comic",
          format: "cbz",
          title: "迷宫饭 第2卷",
          authors: ["九井谅子"],
          seriesName: "迷宫饭",
          seriesKey: "迷宫饭",
          seriesIndex: 2,
          summary: "活画与宝虫料理。",
          publisher: null,
          language: "zh-CN",
          pageCount: 6,
          hasCover: true,
          tags: ["奇幻", "美食"],
          sizeBytes: 44_500_000,
          addedAt: this.iso(-1 * DAY),
          state: {
            page: 0,
            totalPages: 6,
            locator: null,
            completed: false,
            favorite: false,
            lastReadAt: null,
          },
        },
        chapters: [],
      },
      {
        entry: {
          itemId: SAMPLE_CBR_ID,
          libraryId: COMICS_LIBRARY_ID,
          kind: "comic",
          format: "cbr",
          title: "旧归档合刊",
          authors: [],
          seriesName: null,
          seriesKey: null,
          seriesIndex: null,
          summary: "RAR 历史归档。",
          publisher: null,
          language: null,
          pageCount: null,
          hasCover: false,
          tags: [],
          sizeBytes: 28_000_000,
          addedAt: this.iso(-6 * DAY),
          state: {
            page: 0,
            totalPages: null,
            locator: null,
            completed: false,
            favorite: false,
            lastReadAt: null,
          },
        },
        chapters: [],
      },
    ];

    for (const rec of items) {
      this.records.set(rec.entry.itemId, rec);
    }

    this.bookmarksByItem.set(SAMPLE_EPUB_ID, [
      {
        id: stableUuid("seed-bookmark:1"),
        itemId: SAMPLE_EPUB_ID,
        page: 0,
        locator: "chapter:0",
        label: "红岸雷达峰初见",
        note: "关键伏笔",
        createdAt: this.iso(-3600_000),
      },
    ]);
  }

  hasItem(itemId: string): boolean {
    return this.records.has(itemId);
  }

  getRecord(itemId: string): StoredBookRecord | undefined {
    return this.records.get(itemId);
  }

  home(libraryId?: string | null): ReadingHome {
    const all = [...this.records.values()]
      .map((r) => r.entry)
      .filter((e) => !libraryId || e.libraryId === libraryId);

    const continueReading = all
      .filter((e) => !e.state.completed && e.state.lastReadAt && (e.state.page > 0 || e.state.locator))
      .sort((a, b) => (b.state.lastReadAt ?? "").localeCompare(a.state.lastReadAt ?? ""));

    const byAdded = [...all].sort((a, b) => b.addedAt.localeCompare(a.addedAt));
    const recentBooks = byAdded.filter((e) => e.kind === "book");
    const recentComics = byAdded.filter((e) => e.kind === "comic");

    const seriesMap = new Map<string, ReadingEntry[]>();
    for (const entry of all) {
      if (entry.seriesKey && entry.seriesName) {
        const key = `${entry.libraryId}:${entry.kind}:${entry.seriesKey}`;
        const list = seriesMap.get(key) ?? [];
        list.push(entry);
        seriesMap.set(key, list);
      }
    }
    const series: ReadingSeries[] = [];
    for (const members of seriesMap.values()) {
      members.sort((a, b) => (a.seriesIndex ?? 999) - (b.seriesIndex ?? 999));
      const first = members[0]!;
      series.push({
        seriesKey: first.seriesKey!,
        seriesName: first.seriesName!,
        libraryId: first.libraryId,
        kind: first.kind,
        itemCount: members.length,
        completedCount: members.filter((m) => m.state.completed).length,
        coverItemId: first.itemId,
        hasCover: members.some((m) => m.hasCover),
      });
    }

    return { continueReading, recentBooks, recentComics, series };
  }

  list(
    libraryId: string,
    params: {
      kind?: string | null;
      series?: string | null;
      author?: string | null;
      format?: string | null;
      favorite?: string | null;
      completed?: string | null;
      q?: string | null;
      sort?: string | null;
      dir?: string | null;
      cursor?: string | null;
      limit?: string | null;
    },
  ): ReadingPage {
    let entries = [...this.records.values()]
      .map((r) => r.entry)
      .filter((e) => e.libraryId === libraryId);

    if (params.kind) entries = entries.filter((e) => e.kind === params.kind);
    if (params.series) {
      const s = params.series.toLowerCase();
      entries = entries.filter(
        (e) => e.seriesKey?.toLowerCase() === s || e.seriesName?.toLowerCase() === s,
      );
    }
    if (params.author) {
      const a = params.author.toLowerCase();
      entries = entries.filter((e) => e.authors.some((au) => au.toLowerCase().includes(a)));
    }
    if (params.format) {
      const f = params.format.toLowerCase();
      entries = entries.filter((e) => e.format === f);
    }
    if (params.favorite === "true" || params.favorite === "false") {
      const fav = params.favorite === "true";
      entries = entries.filter((e) => e.state.favorite === fav);
    }
    if (params.completed === "true" || params.completed === "false") {
      const comp = params.completed === "true";
      entries = entries.filter((e) => e.state.completed === comp);
    }
    if (params.q) {
      const q = params.q.toLowerCase();
      entries = entries.filter(
        (e) =>
          e.title.toLowerCase().includes(q) ||
          e.authors.some((a) => a.toLowerCase().includes(q)) ||
          (e.seriesName?.toLowerCase().includes(q) ?? false) ||
          e.tags.some((t) => t.toLowerCase().includes(q)) ||
          (e.summary?.toLowerCase().includes(q) ?? false),
      );
    }

    const sort = params.sort ?? "added";
    const dir = params.dir ?? (sort === "added" ? "desc" : "asc");
    const mult = dir === "desc" ? -1 : 1;
    entries.sort((a, b) => {
      if (sort === "title") {
        return mult * a.title.localeCompare(b.title, "zh-CN");
      }
      if (sort === "series") {
        if (a.seriesKey && !b.seriesKey) return -1;
        if (!a.seriesKey && b.seriesKey) return 1;
        if (a.seriesKey && b.seriesKey) {
          const k = a.seriesKey.localeCompare(b.seriesKey, "zh-CN");
          if (k !== 0) return mult * k;
          return mult * ((a.seriesIndex ?? 0) - (b.seriesIndex ?? 0));
        }
        return mult * a.title.localeCompare(b.title, "zh-CN");
      }
      return mult * a.addedAt.localeCompare(b.addedAt);
    });

    const total = entries.length;
    const limit = Math.min(200, Math.max(1, Number(params.limit) || 50));
    const offset = params.cursor ? Number(params.cursor) || 0 : 0;
    const slice = entries.slice(offset, offset + limit);
    const nextCursor = offset + limit < total ? String(offset + limit) : null;

    return { entries: slice, nextCursor, total };
  }

  manifest(itemId: string): ReadingManifest | null {
    const rec = this.records.get(itemId);
    if (!rec) return null;
    const readerMode = resolveReaderMode(rec.entry.kind, rec.entry.format);
    return {
      entry: rec.entry,
      readerMode,
      chapters: rec.chapters.map((c) => ({ index: c.index, title: c.title })),
      bookmarks: this.bookmarksByItem.get(itemId) ?? [],
    };
  }

  chapter(itemId: string, index: number): ReadingChapter | null {
    const rec = this.records.get(itemId);
    if (!rec) return null;
    return rec.chapters[index] ?? null;
  }

  saveProgress(itemId: string, update: ReadingProgressUpdate): ReadingUserState | null {
    const rec = this.records.get(itemId);
    if (!rec) return null;
    rec.entry.state = {
      ...rec.entry.state,
      page: update.page,
      totalPages: update.totalPages ?? rec.entry.state.totalPages ?? rec.entry.pageCount,
      locator: update.locator ?? null,
      completed: update.completed,
      lastReadAt: this.iso(),
    };
    return rec.entry.state;
  }

  setFavorite(itemId: string, favorite: boolean): boolean {
    const rec = this.records.get(itemId);
    if (!rec) return false;
    rec.entry.state = { ...rec.entry.state, favorite };
    return true;
  }

  createBookmark(itemId: string, req: CreateBookmarkRequest): ReadingBookmark | null {
    if (!this.records.has(itemId)) return null;
    const bm: ReadingBookmark = {
      id: stableUuid(`reading-bm:${this.bookmarkSeq++}`),
      itemId,
      page: req.page,
      locator: req.locator ?? null,
      label: req.label,
      note: req.note ?? null,
      createdAt: this.iso(),
    };
    const list = this.bookmarksByItem.get(itemId) ?? [];
    list.push(bm);
    list.sort((a, b) => a.page - b.page);
    this.bookmarksByItem.set(itemId, list);
    return bm;
  }

  deleteBookmark(bookmarkId: string): boolean {
    for (const [itemId, list] of this.bookmarksByItem.entries()) {
      const next = list.filter((b) => b.id !== bookmarkId);
      if (next.length !== list.length) {
        this.bookmarksByItem.set(itemId, next);
        return true;
      }
    }
    return false;
  }
}

function resolveReaderMode(kind: ReadingKind, format: string): ReadingReaderMode {
  if (format === "pdf") return "pdf";
  if (kind === "book" && ["epub", "txt", "md"].includes(format)) return "chapters";
  if (kind === "comic" && ["cbz", "zip", "cbt", "tar"].includes(format)) return "pages";
  return "download_only";
}
