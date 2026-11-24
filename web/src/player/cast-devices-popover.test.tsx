import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import { applyServerEvent } from "@/api/events";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import type { MusicTrack } from "@/modules/music/api";
import { installMockDb, server } from "@/test/msw-server";
import { signIn } from "@/test/render-app";
import { CastDevicesPopover } from "./cast-devices-popover";
import {
  CAST_SESSION_STORAGE_KEY,
  deriveAmbientHex,
  getLocalPlaybackSessionId,
  usePlaybackSessionSync,
  type PlaybackCommandRequest,
  type PlaybackHeartbeatRequest,
} from "./cast-sync";
import { playerStore } from "./store";

function makeTrack(id: string, title: string): MusicTrack {
  return {
    id,
    libraryId: "00000000-0000-4000-8000-000000000010",
    title,
    artist: "周杰伦",
    albumArtist: "周杰伦",
    album: "叶惠美",
    albumId: "al-1",
    artistId: "ar-1",
    trackNumber: 1,
    discNumber: 1,
    year: 2003,
    genres: ["Pop"],
    durationMs: 269_000,
    artworkItemId: id,
    codec: "flac",
    bitrateKbps: 980,
    sampleRateHz: 44100,
    bitDepth: 16,
    channels: 2,
    sizeBytes: 25_000_000,
    addedAt: "2026-01-01T00:00:00Z",
    state: {
      favorite: false,
      stars: 0,
      playCount: 0,
      lastPlayedAt: null,
    },
  };
}

function renderWithQuery(ui: React.ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return {
    queryClient,
    ...render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>),
  };
}

function SyncHarness() {
  usePlaybackSessionSync(true);
  return null;
}

describe("CastDevicesPopover & playback session sync", () => {
  it("persists sessionId in sessionStorage and derives deterministic #RRGGBB ambientHex", () => {
    window.sessionStorage.clear();
    const first = getLocalPlaybackSessionId();
    const second = getLocalPlaybackSessionId();
    expect(first).toBe(second);
    expect(window.sessionStorage.getItem(CAST_SESSION_STORAGE_KEY)).toBe(first);

    const hexA = deriveAmbientHex("track-abc");
    const hexB = deriveAmbientHex("track-abc");
    expect(hexA).toBe(hexB);
    expect(hexA).toMatch(/^#[0-9A-F]{6}$/);
  });

  it("sends heartbeat with ambientHex and executes matching SSE playback.command events", async () => {
    window.sessionStorage.clear();
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);

    const heartbeats: PlaybackHeartbeatRequest[] = [];
    server.use(
      http.post("/api/v1/sessions/playback/heartbeat", async ({ request }) => {
        const body = (await request.json()) as PlaybackHeartbeatRequest;
        heartbeats.push(body);
        return HttpResponse.json({
          id: body.sessionId,
          userId: "u1",
          deviceName: body.deviceName,
          clientKind: body.clientKind,
          mediaKind: body.mediaKind,
          title: body.title ?? null,
          subtitle: body.subtitle ?? null,
          state: body.state,
          positionMs: body.positionMs,
          durationMs: body.durationMs,
          volume: body.volume,
          ambientHex: body.ambientHex ?? null,
          updatedAt: new Date().toISOString(),
        });
      }),
    );

    const { queryClient } = renderWithQuery(<SyncHarness />);
    const sessionId = getLocalPlaybackSessionId();

    const t1 = makeTrack("track-1", "晴天");
    const t2 = makeTrack("track-2", "东风破");
    playerStore.playQueue([t1, t2], 0);

    await waitFor(() => {
      expect(heartbeats.length).toBeGreaterThanOrEqual(1);
    });
    expect(heartbeats[0]?.sessionId).toBe(sessionId);
    expect(heartbeats[0]?.title).toBe("晴天");
    expect(heartbeats[0]?.ambientHex).toMatch(/^#[0-9A-F]{6}$/);

    // Command targeting another session must be ignored
    applyServerEvent(queryClient, {
      id: "1",
      event: "playback.command",
      data: JSON.stringify({
        targetSessionId: "some-other-device",
        command: "pause",
        issuedBy: "admin",
      }),
    });
    expect(playerStore.getSnapshot().playing).toBe(true);

    // Pause command targeting this tab pauses local playerStore
    applyServerEvent(queryClient, {
      id: "2",
      event: "playback.command",
      data: JSON.stringify({
        targetSessionId: sessionId,
        command: "pause",
        issuedBy: "admin",
      }),
    });
    expect(playerStore.getSnapshot().playing).toBe(false);

    // Volume & seek & next commands targeting this tab apply to playerStore
    applyServerEvent(queryClient, {
      id: "3",
      event: "playback.command",
      data: JSON.stringify({
        targetSessionId: sessionId,
        command: "volume",
        volume: 40,
        issuedBy: "admin",
      }),
    });
    expect(playerStore.getSnapshot().volume).toBeCloseTo(0.4, 2);

    applyServerEvent(queryClient, {
      id: "4",
      event: "playback.command",
      data: JSON.stringify({
        targetSessionId: sessionId,
        command: "seek",
        positionMs: 30_000,
        issuedBy: "admin",
      }),
    });
    expect(playerStore.getSnapshot().seekRequest?.sec).toBe(30);

    applyServerEvent(queryClient, {
      id: "5",
      event: "playback.command",
      data: JSON.stringify({
        targetSessionId: sessionId,
        command: "next",
        issuedBy: "admin",
      }),
    });
    expect(playerStore.getSnapshot().currentTrack?.id).toBe("track-2");
  });

  it("lists active devices and dispatches remote commands from the popover", async () => {
    window.sessionStorage.clear();
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);

    const commands: Array<{ id: string; body: PlaybackCommandRequest }> = [];
    server.use(
      http.post("/api/v1/sessions/playback/:id/command", async ({ params, request }) => {
        const body = (await request.json()) as PlaybackCommandRequest;
        commands.push({ id: String(params.id), body });
        return HttpResponse.json(
          { accepted: true, targetSessionId: String(params.id), command: body.command },
          { status: 202 },
        );
      }),
    );

    const user = userEvent.setup();
    renderWithQuery(<CastDevicesPopover tone="player" forceEnabled />);

    await user.click(screen.getByRole("button", { name: "打开设备与跨设备播控" }));

    expect(await screen.findByText("客厅 Apple TV")).toBeInTheDocument();
    expect(screen.getByText(/晴天/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "客厅 Apple TV 暂停" }));
    await waitFor(() => {
      expect(commands).toContainEqual({ id: "living-tv-demo", body: { command: "pause" } });
    });

    await user.click(screen.getByRole("button", { name: "客厅 Apple TV 下一首" }));
    await waitFor(() => {
      expect(commands).toContainEqual({ id: "living-tv-demo", body: { command: "next" } });
    });
  });
});
