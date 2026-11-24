import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { MOVIES_LIBRARY_ID } from "@/mocks/video/state";
import { installMockDb, server } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";
import type { PlaybackRequest, PutVideoSegmentsRequest } from "./api";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("video pro player upgrades", () => {
  it("shows skip intro pill, seeks to endMs, saves custom segments, and switches between multi-version files preserving position", async () => {
    vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined);
    vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => {});
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    signIn(db, MOCK_ACCOUNTS.user.username);
    const user = userEvent.setup();

    const workId = "a".repeat(64);
    const item4k = "11111111-1111-4111-8111-111111111111";
    const item1080 = "22222222-2222-4222-8222-222222222222";
    const playbackCalls: Array<{ itemId: string; body: PlaybackRequest }> = [];
    const savedSegmentPayloads: PutVideoSegmentsRequest[] = [];

    server.use(
      http.get(`/api/v1/libraries/${MOVIES_LIBRARY_ID}/videos/works/${workId}`, () =>
        HttpResponse.json({
          work: {
            id: workId,
            kind: "movie",
            libraryId: MOVIES_LIBRARY_ID,
            title: "Dune",
            fileCount: 2,
            year: 2021,
            overview: "Arrakis",
            seasonNumber: null,
            episodeNumber: null,
            posterItemId: item4k,
            itemId: item4k,
            seriesId: null,
            seriesTitle: null,
            originalTitle: "Dune",
            tagline: null,
            genres: ["Sci-Fi"],
            runtimeMs: 600_000,
            communityRating: 8.2,
            officialRating: null,
            premiered: "2021-10-22",
            backdropItemId: null,
            thumbItemId: null,
            addedAt: "2026-11-23T00:00:00Z",
            state: null,
          },
          studios: [],
          people: [],
          seasons: [],
          files: [
            { itemId: item4k, title: "Dune.2021.2160p.HEVC.mkv", sizeBytes: 12_000_000_000 },
            { itemId: item1080, title: "Dune.2021.1080p.H264.mp4", sizeBytes: 4_500_000_000 },
          ],
          nextUp: null,
          similar: [],
        }),
      ),
      http.get("/api/v1/items/:id/media-info", ({ params }) =>
        HttpResponse.json({
          container: params.id === item4k ? "mkv" : "mp4",
          durationMs: 600_000,
          sizeBytes: params.id === item4k ? 12_000_000_000 : 4_500_000_000,
          bitrateKbps: params.id === item4k ? 24_000 : 8_000,
          video: {
            codec: params.id === item4k ? "hevc" : "h264",
            profile: "Main",
            width: params.id === item4k ? 3840 : 1920,
            height: params.id === item4k ? 2160 : 1080,
            frameRate: 24,
            bitDepth: params.id === item4k ? 10 : 8,
            hdr: params.id === item4k ? "hdr10" : "sdr",
          },
          audio: [],
          subtitles: [],
          chapters: [],
        }),
      ),
      http.get("/api/v1/me/items/:id", () =>
        HttpResponse.json({
          favorite: false,
          stars: 0,
          positionMs: 0,
          completed: false,
          playbackVersion: 0,
          lastPlayedAt: null,
          playCount: 0,
        }),
      ),
      http.get("/api/v1/items/:id/segments", ({ params }) =>
        HttpResponse.json({
          segments: [
            {
              id: "seg-intro-1",
              itemId: String(params.id),
              kind: "intro",
              startMs: 10_000,
              endMs: 90_000,
              source: "chapter",
              updatedAt: "2026-11-23T00:00:00Z",
            },
          ],
        }),
      ),
      http.put("/api/v1/items/:id/segments", async ({ params, request }) => {
        const body = (await request.json()) as PutVideoSegmentsRequest;
        savedSegmentPayloads.push(body);
        return HttpResponse.json({
          segments: body.segments.map((s, idx) => ({
            id: `saved-${idx}`,
            itemId: String(params.id),
            kind: s.kind,
            startMs: s.startMs,
            endMs: s.endMs,
            source: s.source ?? "manual",
            updatedAt: "2026-11-23T00:00:00Z",
          })),
        });
      }),
      http.post("/api/v1/items/:id/video-playback", async ({ params, request }) => {
        const body = (await request.json()) as PlaybackRequest;
        playbackCalls.push({ itemId: String(params.id), body });
        return HttpResponse.json({
          sessionId: `sess-${playbackCalls.length}`,
          mode: "direct",
          protocol: "file",
          url: `/mock/${String(params.id)}.mp4`,
          expiresAt: "2099-01-01T00:00:00Z",
          startMs: 0,
          reason: null,
        });
      }),
      http.get("/api/v1/items/:id/trickplay", () => new HttpResponse(null, { status: 404 })),
    );

    const { container } = await renderLoadedApp(
      `/video/play/${item4k}?libraryId=${MOVIES_LIBRARY_ID}&workId=${workId}&fromStart=true`,
    );

    const videoEl = await waitFor(() => {
      const el = container.querySelector("video");
      expect(el).not.toBeNull();
      return el!;
    });

    Object.defineProperty(videoEl, "duration", { value: 600, configurable: true });
    let curTime = 25;
    Object.defineProperty(videoEl, "currentTime", {
      get: () => curTime,
      set: (v: number) => {
        curTime = v;
      },
      configurable: true,
    });
    fireEvent.loadedMetadata(videoEl);
    fireEvent.timeUpdate(videoEl);

    // Floating Skip Intro pill appears inside 10s..90s
    const skipBtn = await screen.findByRole("button", { name: "跳过片头" }, { timeout: 5000 });
    await user.click(skipBtn);
    expect(curTime).toBe(90);
    expect(screen.queryByRole("button", { name: "跳过片头" })).toBeNull();

    // Open segment markers popover and save a credits marker
    await user.click(screen.getByRole("button", { name: "片头片尾标记" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "片头片尾标记" }), "credits");
    const startInput = screen.getByRole("spinbutton", { name: "开始（秒）" });
    const endInput = screen.getByRole("spinbutton", { name: "结束（秒）" });
    await user.clear(startInput);
    await user.type(startInput, "540");
    await user.clear(endInput);
    await user.type(endInput, "595");
    await user.click(screen.getByRole("button", { name: "保存标记" }));
    await waitFor(() => expect(savedSegmentPayloads).toHaveLength(1));
    expect(savedSegmentPayloads[0]!.segments.some((s) => s.kind === "credits" && s.startMs === 540_000)).toBe(true);

    // Multi-Version Switcher appears with both versions and switches preserving current position (90s)
    const versionBtn = await screen.findByRole("button", { name: "版本切换" });
    await user.click(versionBtn);
    const option1080 = await screen.findByRole("menuitemradio", { name: /1080p.*H\.264.*Dune\.2021\.1080p\.H264\.mp4/ });
    await user.click(option1080);

    await waitFor(() =>
      expect(playbackCalls.some((c) => c.itemId === item1080 && c.body.startMs === 90_000)).toBe(true),
    );
  });
});
