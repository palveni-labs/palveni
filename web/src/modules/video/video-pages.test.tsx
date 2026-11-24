import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { MOVIES_LIBRARY_ID, SERIES_LIBRARY_ID } from "@/mocks/video/state";
import { installMockDb, server } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";
import type { PlaybackRequest } from "./api";
import enLocale from "./locales/en.json";
import zhLocale from "./locales/zh-CN.json";

function flattenKeys(obj: unknown, prefix = ""): string[] {
  if (typeof obj !== "object" || obj === null) return [prefix];
  return Object.entries(obj).flatMap(([k, v]) => flattenKeys(v, prefix ? `${prefix}.${k}` : k));
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("video module", () => {
  it("keeps en and zh-CN module locales in sync", () => {
    expect(flattenKeys(enLocale).sort()).toEqual(flattenKeys(zhLocale).sort());
  });

  it("renders Emby-style video home with hero, rails and library tiles", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    signIn(db, MOCK_ACCOUNTS.user.username);

    await renderLoadedApp("/video");

    expect(await screen.findByRole("region", { name: "精选推荐" }, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "继续观看" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "接下来" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "最新电影" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "最新剧集" })).toBeInTheDocument();

    const librariesSection = screen.getByRole("region", { name: "影视库" });
    expect(within(librariesSection).getByRole("link", { name: /电影/ })).toHaveAttribute(
      "href",
      `/video/${MOVIES_LIBRARY_ID}`,
    );
    expect(within(librariesSection).getByRole("link", { name: /剧集/ })).toHaveAttribute(
      "href",
      `/video/${SERIES_LIBRARY_ID}`,
    );
  });

  it("filters the poster wall by alphabet startsWith, favorite and genres, showing total count", async () => {
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(800);
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(1100);
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp(`/video/${MOVIES_LIBRARY_ID}`);

    expect(await screen.findByText("共 18 部", {}, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "文件视图" })).toHaveAttribute(
      "href",
      `/library/${MOVIES_LIBRARY_ID}?view=files`,
    );

    // Jump to letter V -> "天鹅绒地平线" (The Velvet Horizon)
    const alphaNav = screen.getAllByRole("navigation", { name: "按字母跳转" })[0]!;
    await user.click(within(alphaNav).getByRole("button", { name: "V" }));
    expect(await screen.findByText("共 1 部")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "天鹅绒地平线" })).toBeInTheDocument();

    // Clear filter restores full wall
    await user.click(screen.getByRole("button", { name: "清除筛选" }));
    expect(await screen.findByText("共 18 部")).toBeInTheDocument();

    // Genres tab -> select 科幻 -> returns to filtered grid
    await user.click(screen.getByRole("button", { name: "流派" }));
    await user.click(await screen.findByRole("button", { name: /科幻/ }));
    expect(await screen.findByRole("combobox", { name: "全部流派" })).toHaveValue("科幻");
  });

  it("renders series detail with seasons, episodes, cast, media info and played/favorite toggles", async () => {
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(800);
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(1100);
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    await renderLoadedApp(`/video/${SERIES_LIBRARY_ID}`);

    const harbourLink = await screen.findByRole("link", { name: "港口" }, { timeout: 5000 });
    await user.click(harbourLink);

    expect(await screen.findByRole("heading", { level: 1, name: "港口" }, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByText("Harbour")).toBeInTheDocument();
    expect(await screen.findByRole("region", { name: "媒体规格" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "演职员" })).toBeInTheDocument();

    // Switch to Season 1 and verify episodes load
    await user.click(screen.getByRole("tab", { name: /第 1 季/ }));
    expect(await screen.findByRole("button", { name: "01 · 四号码头" })).toBeInTheDocument();

    // Toggle favorite button
    const favButton = screen.getByRole("button", { name: "已收藏" });
    expect(favButton).toHaveAttribute("aria-pressed", "true");
    await user.click(favButton);
    await waitFor(() => expect(screen.getByRole("button", { name: "收藏" })).toHaveAttribute("aria-pressed", "false"));
  });

  it("runs fullscreen player with capabilities, media-exclusive event, resume prompt, quality switch and session cleanup", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    const itemId = "23587a91-9e35-4821-8a48-3aeddba77d11";
    const playbackRequests: PlaybackRequest[] = [];
    const endedSessions: string[] = [];
    const exclusiveEvents: Event[] = [];
    const onExclusive = (e: Event) => exclusiveEvents.push(e);
    window.addEventListener("palveni:media-exclusive", onExclusive);

    let seq = 1;
    server.use(
      http.get(`/api/v1/me/items/${itemId}`, () =>
        HttpResponse.json({
          favorite: false,
          stars: 0,
          positionMs: 125_000,
          completed: false,
          playbackVersion: 3,
          lastPlayedAt: null,
          playCount: 0,
        }),
      ),
      http.post(`/api/v1/items/${itemId}/video-playback`, async ({ request }) => {
        const body = (await request.json()) as PlaybackRequest;
        playbackRequests.push(body);
        const sid = `sess-${seq++}`;
        return HttpResponse.json({
          sessionId: sid,
          mode: body.maxHeight ? "transcode" : "direct",
          protocol: "file",
          url: `/mock/video-${sid}.mp4`,
          expiresAt: "2099-01-01T00:00:00Z",
          startMs: 0,
          reason: body.maxHeight ? `capped ${body.maxHeight}p` : null,
        });
      }),
      http.delete("/api/v1/video-sessions/:sessionId", ({ params }) => {
        endedSessions.push(String(params.sessionId));
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const playMock = vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined);
    const { container, unmount } = await renderLoadedApp(`/video/play/${itemId}?title=TestFilm`);

    // Initial playback request carried detected client capabilities
    await waitFor(() => expect(playbackRequests).toHaveLength(1));
    expect(playbackRequests[0]?.client.containers?.length).toBeGreaterThan(0);
    expect(playbackRequests[0]?.client.videoCodecs?.length).toBeGreaterThan(0);
    expect(playbackRequests[0]?.client.audioCodecs?.length).toBeGreaterThan(0);

    // Resume prompt appears because saved positionMs = 125_000 (> 10s)
    const resumeDialog = await screen.findByRole("dialog", { name: /上次观看到 2:05/ });
    const video = container.querySelector("video")!;
    Object.defineProperty(video, "duration", { value: 600, configurable: true });
    fireEvent.loadedMetadata(video);

    await user.click(within(resumeDialog).getByRole("button", { name: "继续播放" }));
    expect(video.currentTime).toBe(125);
    expect(exclusiveEvents.length).toBeGreaterThan(0);
    expect(playMock).toHaveBeenCalled();

    // Switch quality to 720p -> re-requests playback with maxHeight: 720 and ends previous session
    await user.click(screen.getByRole("button", { name: "清晰度" }));
    await user.click(await screen.findByRole("menuitemradio", { name: "720p" }));
    await waitFor(() => expect(playbackRequests).toHaveLength(2));
    expect(playbackRequests[1]?.maxHeight).toBe(720);
    await waitFor(() => expect(endedSessions).toContain("sess-1"));

    // Leaving player cleans up active session
    unmount();
    await waitFor(() => expect(endedSessions).toContain("sess-2"));
    window.removeEventListener("palveni:media-exclusive", onExclusive);
  });

  it("navigates from cast member to person-filtered filmography and renders search and favorites", async () => {
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(800);
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(1100);
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    // Person filter via CastRail link
    const first = await renderLoadedApp(`/video/${SERIES_LIBRARY_ID}`);
    await user.click(await screen.findByRole("link", { name: "港口" }, { timeout: 5000 }));
    const castRegion = await screen.findByRole("region", { name: "演职员" }, { timeout: 5000 });
    const firstActor = within(castRegion).getAllByRole("link")[0]!;
    await user.click(firstActor);
    expect(await screen.findByRole("button", { name: /演职员：/ })).toBeInTheDocument();
    first.unmount();

    // Cross-library search page
    const second = await renderLoadedApp("/video/search?q=%E6%B8%AF%E5%8F%A3");
    expect(await screen.findByRole("link", { name: "港口" }, { timeout: 5000 })).toBeInTheDocument();
    second.unmount();

    // Cross-library favorites page
    const third = await renderLoadedApp("/video/favorites");
    expect(await screen.findByRole("heading", { level: 1, name: "我的收藏" }, { timeout: 5000 })).toBeInTheDocument();
    expect(await screen.findByRole("link", { name: "港口" })).toBeInTheDocument();
    third.unmount();
  });

  it("toggles Emby playback info overlay and chapter list in fullscreen player", async () => {
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    const itemId = "23587a91-9e35-4821-8a48-3aeddba77d11";
    vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined);
    server.use(
      http.get(`/api/v1/items/${itemId}/media-info`, () =>
        HttpResponse.json({
          container: "mkv",
          durationMs: 1_200_000,
          sizeBytes: 2_000_000_000,
          bitrateKbps: 14_500,
          video: { codec: "hevc", profile: "Main 10", width: 3840, height: 2160, frameRate: 23.976, bitDepth: 10, hdr: "hdr10" },
          audio: [{ index: 1, codec: "eac3", language: "zh", title: "国语 5.1", channels: 6, channelLayout: "5.1", default: true }],
          subtitles: [],
          chapters: [
            { startMs: 0, title: "序幕" },
            { startMs: 300_000, title: "深海信号" },
          ],
        }),
      ),
    );

    const { unmount } = await renderLoadedApp(`/video/play/${itemId}?fromStart=1&title=ChapterTest`);
    await user.click(await screen.findByRole("button", { name: "播放信息" }, { timeout: 5000 }));
    const infoPanel = await screen.findByRole("complementary", { name: "播放信息" });
    expect(within(infoPanel).getByText(/4K · HEVC · HDR10/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "章节" }));
    expect(await screen.findByRole("menuitemradio", { name: /深海信号/ })).toBeInTheDocument();
    unmount();
  });
});
