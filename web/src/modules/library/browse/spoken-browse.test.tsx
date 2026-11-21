import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import type { components } from "@/api/generated";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";

it.each([
  ["audiobook", "book", "chapter"],
  ["podcast", "show", "episode"],
] as const)("opens a %s as a regular listener and restores the recording position", async (kind, groupKind, itemKind) => {
  const db = installMockDb({ features: [...M0_FEATURES, "spoken", "spokenChapters"] });
  const library = db.createLibrary("Field recordings", kind, ["/media/recordings"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const collection = "a".repeat(64);
  const id = "6277b2e7-9da5-4bd4-9226-c5f75ca47e1f";
  const state: components["schemas"]["MediaState"] = {
    favorite: false, stars: 0, positionMs: 8000, completed: false, playbackVersion: 3, lastPlayedAt: null, playCount: 1,
  };
  const checkpoints: unknown[] = [];
  const attempted: string[] = [];
  let chapterReads = 0;
  server.use(
    http.get(`/api/v1/items/${id}/chapters`, () => {
      chapterReads++;
      return HttpResponse.json({ chapters: [{ title: null, startMs: 5000 }, { title: "Night watch", startMs: 20000 }] });
    }),
    http.get(`/api/v1/libraries/${library.id}/spoken`, ({ request }) => HttpResponse.json({
      entries: new URL(request.url).searchParams.get("collection") === collection
        ? [{ kind: itemKind, id, title: "Dawn chorus", artist: null, discNumber: null, number: 2, durationMs: 120_000 }]
        : [{ kind: groupKind, id: collection, title: "Birdsong" }], nextCursor: null,
    })),
    http.post(`/api/v1/items/${id}/playback`, ({ request }) => {
      const query = new URL(request.url).searchParams;
      const format = query.get("format");
      if (format) attempted.push(format);
      if (kind === "podcast" && format === "mp3")
        return HttpResponse.json({ error: { code: "transcode_format_unavailable", message: "Encoder unavailable" } }, { status: 503 });
      const url = query.has("format") ? `/media/${query.get("startMs")}.${query.get("format")}` : "/media/example.opus";
      return HttpResponse.json({ mode: query.has("format") ? "transcode" : "direct", url, expiresAt: new Date(Date.now() + 900_000).toISOString() });
    }),
    http.get(`/api/v1/me/items/${id}`, () => HttpResponse.json(state)),
    http.put(`/api/v1/me/items/${id}/progress`, async ({ request }) => {
      checkpoints.push(await request.json());
      return HttpResponse.json({ ...state, playbackVersion: 4 });
    }),
  );
  const user = userEvent.setup();
  const { container } = await renderLoadedApp(`/library/${library.id}`);
  await user.click(await screen.findByRole("link", { name: "Birdsong" }));
  await user.click(await screen.findByRole("button", { name: "播放《Dawn chorus》" }));
  const audio = await waitFor(() => {
    const node = container.querySelector("audio");
    expect(node).toHaveAttribute("src", "/media/example.opus");
    return node!;
  });
  Object.defineProperty(audio, "duration", { value: 120 });
  fireEvent.loadedMetadata(audio);
  expect(audio.currentTime).toBe(8);
  fireEvent.playing(audio);
  audio.currentTime = 9.75;
  fireEvent.seeked(audio);
  await waitFor(() => expect(checkpoints).toEqual([{ completed: false, positionMs: 9750, expectedVersion: 3 }]));
  if (kind === "audiobook") {
    await user.selectOptions(await screen.findByRole("combobox", { name: "跳至章节" }), "20000");
    expect(audio.currentTime).toBe(20);
    await waitFor(() => expect(checkpoints.at(-1)).toMatchObject({ positionMs: 20000 }));
  }

  Object.defineProperty(audio, "error", { value: { code: 4 } });
  Object.defineProperty(audio, "canPlayType", { value: (mime: string) => mime.startsWith("audio/") ? "probably" : "" });
  fireEvent.error(audio);
  if (kind === "audiobook")
    expect(screen.getByRole("combobox", { name: "跳至章节" })).toBeDisabled();
  const expectedFormat = kind === "podcast" ? "aac" : "mp3";
  await waitFor(() => expect(audio).toHaveAttribute("src", `/media/${kind === "audiobook" ? 20000 : 9750}.${expectedFormat}`));
  expect(attempted).toEqual(kind === "podcast" ? ["mp3", "aac"] : ["mp3"]);
  if (kind === "audiobook") {
    await user.selectOptions(screen.getByRole("combobox", { name: "跳至章节" }), screen.getByRole("option", { name: "第 1 章" }));
    await waitFor(() => expect(audio).toHaveAttribute("src", "/media/5000.mp3"));
  }
  audio.currentTime = 0;
  fireEvent.loadedMetadata(audio);
  expect(audio.currentTime).toBe(0);
  fireEvent.playing(audio);
  audio.currentTime = 2;
  fireEvent.pause(audio);
  await waitFor(() => expect(checkpoints.at(-1)).toEqual({ completed: false, positionMs: kind === "audiobook" ? 7000 : 11750, expectedVersion: 4 }));
  expect(chapterReads).toBe(kind === "audiobook" ? 1 : 0);

  const breadcrumb = screen.getByRole("navigation", { name: "当前位置" });
  await user.click(within(breadcrumb).getByRole("link"));
  await user.click(await screen.findByRole("link", { name: "Birdsong" }));
  await screen.findByRole("button", { name: "播放《Dawn chorus》" });
  expect(container.querySelector("audio")).toBeNull();
}, 15_000);
