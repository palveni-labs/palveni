import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import type { components } from "@/api/generated";
import { MOCK_ACCOUNTS, M0_FEATURES } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

const initial: components["schemas"]["MediaState"] = {
  completed: false, favorite: false, lastPlayedAt: null, playCount: 0, playbackVersion: 2, positionMs: 11000, stars: 0,
};

it("resumes from the latest position, updates favorites and removes completed video from Continue", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "personal"] });
  const library = db.createLibrary("Cinema", "video", ["/media/cinema"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const itemId = "fa505335-b76f-494d-aa4b-a961f3b8316d";
  const state = { ...initial };
  const checkpoints: unknown[] = [];
  server.use(
    http.get("/api/v1/me/items", ({ request }) => {
      const view = new URL(request.url).searchParams.get("view");
      const included = view === "recent" || (view === "continue" ? !state.completed : state.favorite);
      return HttpResponse.json({ items: included ? [{ itemId, libraryId: library.id, kind: "video", title: "North shore", state }] : [], nextBefore: null });
    }),
    http.put(`/api/v1/me/items/${itemId}/favorite`, async ({ request }) => {
      state.favorite = (await request.json() as { favorite: boolean }).favorite;
      return HttpResponse.json(state);
    }),
    http.post(`/api/v1/items/${itemId}/playback`, () => HttpResponse.json({ mode: "direct", url: "/example.mp4", expiresAt: "2099-01-01T00:00:00Z" })),
    http.get(`/api/v1/me/items/${itemId}`, () => HttpResponse.json({ ...state, positionMs: 32000 })),
    http.put(`/api/v1/me/items/${itemId}/progress`, async ({ request }) => {
      const update = await request.json() as components["schemas"]["PlaybackUpdate"];
      checkpoints.push(update);
      Object.assign(state, { completed: update.completed, positionMs: update.positionMs, playbackVersion: state.playbackVersion + 1 });
      return HttpResponse.json(state);
    }),
  );
  const user = userEvent.setup();
  renderApp("/library");
  const shelf = within(await screen.findByRole("region", { name: "我的播放" }, { timeout: 5000 }));
  await user.click(await shelf.findByRole("button", { name: "收藏《North shore》" }));
  await user.click(shelf.getByRole("button", { name: "我的收藏" }));
  const saved = await shelf.findByRole("button", { name: "收藏《North shore》" });
  expect(saved).toHaveAttribute("aria-pressed", "true");
  await user.click(shelf.getByRole("button", { name: "继续播放" }));
  await user.click(await shelf.findByRole("button", { name: "从 0:11 继续播放《North shore》" }));
  const dialog = await screen.findByRole("dialog", { name: "North shore" });
  const video = await waitFor(() => {
    const element = dialog.querySelector("video");
    expect(element).toHaveAttribute("src", "/example.mp4");
    return element!;
  });
  Object.defineProperty(video, "duration", { value: 60 });
  fireEvent.loadedMetadata(video);
  expect(video.currentTime).toBe(32);
  fireEvent.playing(video);
  Object.defineProperty(video, "ended", { value: true });
  fireEvent.ended(video);
  await waitFor(() => expect(checkpoints).toEqual([{ expectedVersion: 2, positionMs: 0, completed: true }]));
  await user.click(within(dialog).getByRole("button", { name: "关闭播放器" }));
  expect(await shelf.findByText("未播完的影视、音乐和音频节目会显示在这里。")).toBeVisible();
  await user.click(shelf.getByRole("button", { name: "最近播放" }));
  expect(await shelf.findByRole("button", { name: "播放《North shore》" })).toBeVisible();
  expect(screen.queryByRole("link", { name: "媒体库管理" })).not.toBeInTheDocument();
});

it("uses server cursors, handles a withdrawn recording, and keeps the library navigation usable", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "personal"] });
  const library = db.createLibrary("Audio library", "audiobook", ["/media/books"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  let removed = false;
  const itemId = "a620b3ea-324a-46e1-8ad0-8af4c640d1f8";
  server.use(
    http.get("/api/v1/me/items", ({ request }) => {
      const second = new URL(request.url).searchParams.get("before") === "70";
      const visible = { itemId, libraryId: library.id, kind: "chapter", title: second ? "Chapter 8" : "Chapter 7", state: initial };
      return HttpResponse.json({ items: [
        ...(removed && second ? [] : [visible]), { ...visible, itemId: "removed", libraryId: "withdrawn", title: "Private library title" },
      ], nextBefore: second ? null : 70 });
    }),
    http.post(`/api/v1/items/${itemId}/playback`, () => {
      removed = true;
      return HttpResponse.json({ code: "not_found", message: "No such item" }, { status: 404 });
    }),
    http.get(`/api/v1/me/items/${itemId}`, () => HttpResponse.json(initial)),
  );
  const user = userEvent.setup();
  renderApp("/library");
  const shelf = within(await screen.findByRole("region", { name: "我的播放" }, { timeout: 5000 }));
  await user.click(await shelf.findByRole("button", { name: "下一页" }));
  await user.click(await shelf.findByRole("button", { name: /继续播放《Chapter 8》/ }));
  const dialog = await screen.findByRole("dialog", { name: "Chapter 8" });
  expect(await within(dialog).findByRole("alert")).toBeVisible();
  expect(dialog.querySelector("audio")).toBeNull();
  await user.click(within(dialog).getByRole("button", { name: "关闭播放器" }));
  await waitFor(() => expect(shelf.queryByText("Chapter 8")).not.toBeInTheDocument());
  await user.click(shelf.getByRole("button", { name: "上一页" }));
  expect(await shelf.findByText("Chapter 7")).toBeVisible();
  expect(screen.queryByText("Private library title")).not.toBeInTheDocument();
});

it("opens a music library at its artists, plays an album track, and keeps the original files reachable", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "personal", "music"] });
  const library = db.createLibrary("Music collection", "music", ["/media/sound"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const track = "a620b3ea-324a-46e1-8ad0-8af4c640d1f8";
  const artist = "a".repeat(64);
  const album = "b".repeat(64);
  server.use(
    http.get(`/api/v1/libraries/${library.id}/music`, ({ request }) => {
      const q = new URL(request.url).searchParams;
      const entry = q.get("album") ? { kind: "track", title: "Sunrise", id: track, artist: "Ensemble", durationMs: 60000, trackNumber: 1 }
        : q.get("artist") ? { kind: "album", name: "Skies", id: album, artworkItemId: track } : { kind: "artist", name: "Ensemble", id: artist };
      return HttpResponse.json({ entries: [entry], nextCursor: null });
    }),
    http.post(`/api/v1/items/${track}/playback`, () => HttpResponse.json({ mode: "direct", url: "/track.wav", expiresAt: "2099-01-01T00:00:00Z" })),
    http.get(`/api/v1/me/items/${track}`, () => HttpResponse.json(initial)),
  );
  const user = userEvent.setup();
  const { container, router } = renderApp(`/library/${library.id}`);
  await user.click(await screen.findByRole("link", { name: "Ensemble" }, { timeout: 5000 }));
  await user.click(await screen.findByRole("link", { name: "Skies" }));
  await user.click(await screen.findByRole("button", { name: "播放《Sunrise》" }));
  await waitFor(() => expect(container.querySelector("audio")).toHaveAttribute("src", "/track.wav"));
  const modes = within(screen.getByRole("navigation", { name: "显示方式" }));
  await user.click(modes.getByRole("link", { name: "文件" }));
  await waitFor(() => expect(router.state.location.search).toEqual({ view: "files" }));
  expect(container.querySelector("audio")).toBeNull();
});
