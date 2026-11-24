import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import type { components } from "@/api/generated";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { MEDIA_EXCLUSIVE_EVENT, playerStore } from "@/player/store";
import { ALL_FEATURES, installMockDb, server } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

function collectKeys(obj: unknown, prefix = ""): string[] {
  if (typeof obj !== "object" || obj === null) return [prefix];
  return Object.entries(obj).flatMap(([k, v]) => collectKeys(v, prefix ? `${prefix}.${k}` : k));
}

describe("music app module", () => {
  it("defines matching en and zh-CN module locale keys", () => {
    expect(collectKeys(en).sort()).toEqual(collectKeys(zhCN).sort());
  });

  it("renders Music Home, starts shuffled playback in the persistent bar, and pauses on video exclusive event", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    await renderLoadedApp("/music");

    expect(await screen.findByRole("region", { name: "最近添加" })).toBeInTheDocument();
    expect(await screen.findByRole("region", { name: "随机推荐" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "随机播放全部" }));

    // Global player bar mounts with the playing track
    await waitFor(() => expect(playerStore.getSnapshot().queue.length).toBeGreaterThan(0));
    const bars = await screen.findAllByRole("region", { name: "音乐播放器" });
    expect(bars.length).toBeGreaterThan(0);

    // Open full-screen Now Playing via keyboard shortcut L
    fireEvent.keyDown(window, { key: "l" });
    const nowPlaying = await screen.findByRole("dialog", { name: "正在播放" });
    expect(nowPlaying).toBeInTheDocument();

    // Switch to Queue tab inside Now Playing, then close with Escape
    await user.click(within(nowPlaying).getByRole("tab", { name: "播放队列" }));
    expect(within(nowPlaying).getByText(/队列 · \d+ 首/)).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "正在播放" })).not.toBeInTheDocument());

    // Exclusive media event pauses music
    expect(playerStore.getSnapshot().playing).toBe(true);
    act(() => {
      window.dispatchEvent(new CustomEvent(MEDIA_EXCLUSIVE_EVENT, { detail: { source: "video" } }));
    });
    expect(playerStore.getSnapshot().playing).toBe(false);
  });

  it("browses albums, toggles grid/list view, opens album detail, and favorites album", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = await renderLoadedApp("/music/albums");

    const albumLink = await screen.findByRole("link", { name: "松针与电波" });
    expect(albumLink).toBeInTheDocument();

    // Toggle to list view and back
    await user.click(screen.getByRole("radio", { name: "列表视图" }));
    expect(await screen.findByRole("link", { name: "松针与电波" })).toBeInTheDocument();

    // Open album detail
    await user.click(screen.getByRole("link", { name: "松针与电波" }));
    await waitFor(() => expect(router.state.location.pathname).toMatch(/^\/music\/albums\/.+\/.+$/));
    expect(await screen.findByRole("heading", { level: 1, name: "松针与电波" })).toBeInTheDocument();

    // Play album populates queue
    await user.click(screen.getByRole("button", { name: "播放全部" }));
    await waitFor(() => expect(playerStore.getSnapshot().currentTrack?.album).toBe("松针与电波"));
  });

  it("supports multi-select on Songs table and ignores Space shortcut while typing in search", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    await renderLoadedApp("/music/songs");

    const checkboxes = await screen.findAllByRole("checkbox");
    // checkboxes[0] is Select All, checkboxes[1] and [2] are first two tracks
    expect(checkboxes.length).toBeGreaterThan(2);
    await user.click(checkboxes[1]!);
    await user.click(checkboxes[2]!);

    expect(await screen.findByRole("toolbar", { name: "已选 2 首" })).toBeInTheDocument();
    await user.click(within(screen.getByRole("toolbar", { name: "已选 2 首" })).getByRole("button", { name: "立即播放" }));
    await waitFor(() => expect(playerStore.getSnapshot().queue).toHaveLength(2));
    expect(playerStore.getSnapshot().playing).toBe(true);

    // Typing space inside search input must NOT toggle pause
    const searchBox = screen.getByRole("searchbox", { name: "筛选歌曲、艺人或专辑…" });
    await user.click(searchBox);
    await user.type(searchBox, "a b");
    expect(playerStore.getSnapshot().playing).toBe(true);
  });

  it("creates, opens, reorders and deletes playlists", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    const { router } = await renderLoadedApp("/music/playlists");

    expect(await screen.findByRole("link", { name: "通勤路上" })).toBeInTheDocument();

    // Create a new playlist
    await user.click(screen.getByRole("button", { name: "新建歌单" }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText("歌单名称"), "周末爵士");
    await user.click(within(dialog).getByRole("button", { name: "创建" }));
    expect(await screen.findByRole("link", { name: "周末爵士" })).toBeInTheDocument();

    // Open seeded playlist detail and verify tracks load
    await user.click(screen.getByRole("link", { name: "通勤路上" }));
    await waitFor(() => expect(router.state.location.pathname).toMatch(/^\/music\/playlists\/.+$/));
    expect(await screen.findByRole("heading", { level: 1, name: "通勤路上" })).toBeInTheDocument();
    const moveDownButtons = await screen.findAllByRole("button", { name: "下移" });
    expect(moveDownButtons.length).toBeGreaterThan(1);
    await user.click(moveDownButtons[0]!);
  });

  it("navigates Artists, Genres, Favorites tabs, Search, and switches between multiple music libraries", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    db.createLibrary("古典黑胶", "music", ["/srv/media/vinyl"]);
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();

    const artistsApp = await renderLoadedApp("/music/artists");
    expect(await within(artistsApp.container).findByRole("link", { name: "林间信号" })).toBeInTheDocument();

    // Library switcher appears when multiple music libraries exist
    const switcher = within(artistsApp.container).getByRole("button", { name: "切换音乐库" });
    await user.click(switcher);
    const menu = await screen.findByRole("menu", { name: "切换音乐库" });
    await user.click(within(menu).getByRole("menuitem", { name: "古典黑胶" }));
    expect(window.localStorage.getItem("palveni.music.libraryId")).toBeTruthy();
    artistsApp.unmount();

    // Genres
    const genres = await renderLoadedApp("/music/genres");
    expect(await within(genres.container).findByRole("link", { name: "爵士" })).toBeInTheDocument();
    genres.unmount();

    // Favorites tabs
    const favs = await renderLoadedApp("/music/favorites");
    await user.click(await within(favs.container).findByRole("radio", { name: "专辑" }));
    await user.click(within(favs.container).getByRole("radio", { name: "艺人" }));
    favs.unmount();

    // Search
    const searchApp = await renderLoadedApp("/music/search?q=%E6%9E%97%E9%97%B4");
    const artistsRegion = await within(searchApp.container).findByRole("region", { name: "艺人" });
    expect(within(artistsRegion).getByRole("link", { name: "林间信号" })).toBeInTheDocument();
  }, 15_000);

  it("browses /music/audiobooks, resumes spoken chapters, jumps M4B timestamps, and controls speed & sleep timer", async () => {
    const db = installMockDb({ features: [...ALL_FEATURES, "spoken", "spokenChapters"] });
    const audiobookLib = db.createLibrary("有声书房", "audiobook", ["/srv/media/audiobooks"]);
    const podcastLib = db.createLibrary("深夜电台", "podcast", ["/srv/media/podcasts"]);
    signIn(db, MOCK_ACCOUNTS.admin.username);

    const bookCollectionId = "b".repeat(64);
    const chapter1Id = "7177b2e7-9da5-4bd4-9226-c5f75ca47e11";
    const chapter2Id = "7177b2e7-9da5-4bd4-9226-c5f75ca47e12";
    const chapter1State: components["schemas"]["MediaState"] = {
      favorite: false,
      stars: 0,
      positionMs: 85_000,
      completed: false,
      playbackVersion: 2,
      lastPlayedAt: "2026-11-22T20:00:00Z",
      playCount: 0,
    };
    const chapter2State: components["schemas"]["MediaState"] = {
      favorite: false,
      stars: 0,
      positionMs: 0,
      completed: true,
      playbackVersion: 1,
      lastPlayedAt: "2026-11-21T20:00:00Z",
      playCount: 1,
    };

    server.use(
      http.get(`/api/v1/libraries/${audiobookLib.id}/spoken`, ({ request }) => {
        const col = new URL(request.url).searchParams.get("collection");
        if (col === bookCollectionId) {
          return HttpResponse.json({
            entries: [
              { kind: "chapter", id: chapter1Id, title: "第一部 · 沉默的春天", artist: "刘慈欣", discNumber: 1, number: 1, durationMs: 1_800_000 },
              { kind: "chapter", id: chapter2Id, title: "第二部 · 黑暗森林", artist: "刘慈欣", discNumber: 1, number: 2, durationMs: 2_100_000 },
            ],
            nextCursor: null,
          });
        }
        return HttpResponse.json({
          entries: [{ kind: "book", id: bookCollectionId, title: "三体全集（有声珍藏版）" }],
          nextCursor: null,
        });
      }),
      http.get(`/api/v1/libraries/${podcastLib.id}/spoken`, () =>
        HttpResponse.json({
          entries: [{ kind: "show", id: "c".repeat(64), title: "内核漫游指南" }],
          nextCursor: null,
        }),
      ),
      http.get(`/api/v1/items/${chapter1Id}/chapters`, () =>
        HttpResponse.json({
          chapters: [
            { title: "序幕", startMs: 0 },
            { title: "红岸基地", startMs: 300_000 },
          ],
        }),
      ),
      http.get(`/api/v1/items/${chapter2Id}/chapters`, () => HttpResponse.json({ chapters: [] })),
      http.get(`/api/v1/me/items/${chapter1Id}`, () => HttpResponse.json(chapter1State)),
      http.get(`/api/v1/me/items/${chapter2Id}`, () => HttpResponse.json(chapter2State)),
      http.put(`/api/v1/me/items/${chapter1Id}/progress`, async ({ request }) => {
        const body = (await request.json()) as { positionMs: number; completed: boolean };
        chapter1State.positionMs = body.positionMs;
        chapter1State.completed = body.completed;
        chapter1State.playbackVersion += 1;
        return HttpResponse.json(chapter1State);
      }),
      http.post(`/api/v1/items/${chapter1Id}/playback`, () =>
        HttpResponse.json({
          mode: "direct",
          url: "/api/v1/items/mock-stream",
          expiresAt: new Date(Date.now() + 900_000).toISOString(),
        }),
      ),
      http.get(`/api/v1/items/${chapter1Id}/artwork`, () => new HttpResponse(null, { status: 404 })),
      http.get(`/api/v1/items/${chapter1Id}/loudness`, () => new HttpResponse(null, { status: 404 })),
    );

    const user = userEvent.setup();
    await renderLoadedApp("/music/audiobooks");

    // Both spoken libraries appear in switcher; open the audiobook collection
    expect(await screen.findByRole("button", { name: "有声书房" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "深夜电台" })).toBeInTheDocument();

    await user.click(await screen.findByRole("button", { name: /三体全集（有声珍藏版）/ }));
    expect(await screen.findByRole("heading", { level: 2, name: "三体全集（有声珍藏版）" })).toBeInTheDocument();

    // Resume badge + button for chapter 1 (85s -> 1:25) and Completed badge for chapter 2
    const resumeBtn = await screen.findByRole("button", { name: /续听 1:25/ });
    expect(resumeBtn).toBeInTheDocument();
    expect(await screen.findByText("已听完")).toBeInTheDocument();

    // M4B embedded chapters render and can seek/start playback
    expect(await screen.findByRole("button", { name: "跳至章节：红岸基地" })).toBeInTheDocument();

    // Click Resume -> starts global player at 85s
    await user.click(resumeBtn);
    await waitFor(() => expect(playerStore.getSnapshot().currentTrack?.id).toBe(chapter1Id));
    expect(playerStore.getSnapshot().currentTimeSec).toBe(85);

    // Click M4B embedded chapter "红岸基地" -> seeks global player to 300s
    await user.click(screen.getByRole("button", { name: "跳至章节：红岸基地" }));
    expect(playerStore.getSnapshot().currentTimeSec).toBe(300);

    // Change playback speed to 1.5x via inline quick control
    const speedGroup = screen.getByRole("group", { name: "播放倍速" });
    await user.click(within(speedGroup).getByRole("button", { name: "1.5×" }));
    expect(playerStore.getSnapshot().playbackRate).toBe(1.5);

    // Activate Sleep Timer (30 min) via PlayerBar popover and verify live badge
    const sleepTriggers = screen.getAllByRole("button", { name: "睡眠定时" });
    await user.click(sleepTriggers[0]!);
    const sleepDialog = await screen.findByRole("dialog", { name: "睡眠定时" });
    await user.click(within(sleepDialog).getByRole("button", { name: "30 分钟" }));
    expect(playerStore.getSnapshot().sleepRemainingMinutes).toBe(30);
    expect(screen.getAllByText("30分").length).toBeGreaterThan(0);
  }, 15_000);

  it("shows empty state with Library Management link and spoken-genre quick link when no spoken library exists", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const musicLib = db.libraries.find((lib) => lib.kind === "music")!;
    server.use(
      http.get(`/api/v1/libraries/${musicLib.id}/music/genres`, () =>
        HttpResponse.json({
          genres: [
            { name: "有声书", albumCount: 4, trackCount: 36 },
            { name: "爵士", albumCount: 8, trackCount: 64 },
          ],
        }),
      ),
    );

    await renderLoadedApp("/music/audiobooks");

    expect(await screen.findByText("还没有有声书或播客媒体库")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "管理媒体库" })).toHaveAttribute("href", "/system/libraries?create=true");
    expect(await screen.findByRole("link", { name: "查看“有声书”专辑" })).toBeInTheDocument();
  });
});
