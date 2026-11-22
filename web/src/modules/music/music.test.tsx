import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { MUSIC_MOCK_FEATURES } from "@/mocks/music/handlers";
import { MEDIA_EXCLUSIVE_EVENT, playerStore } from "@/player/store";
import { installMockDb } from "@/test/msw-server";
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
    const db = installMockDb({ features: [...M0_FEATURES, ...MUSIC_MOCK_FEATURES] });
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
    const db = installMockDb({ features: [...M0_FEATURES, ...MUSIC_MOCK_FEATURES] });
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
    const db = installMockDb({ features: [...M0_FEATURES, ...MUSIC_MOCK_FEATURES] });
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
    const db = installMockDb({ features: [...M0_FEATURES, ...MUSIC_MOCK_FEATURES] });
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
    const db = installMockDb({ features: [...M0_FEATURES, ...MUSIC_MOCK_FEATURES] });
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
});
