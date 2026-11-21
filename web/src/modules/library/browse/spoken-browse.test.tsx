import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import type { components } from "@/api/generated";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

it.each([
  ["audiobook", "book", "chapter"],
  ["podcast", "show", "episode"],
] as const)("opens a %s as a regular listener and restores the recording position", async (kind, groupKind, itemKind) => {
  const db = installMockDb({ features: [...M0_FEATURES, "spoken"] });
  const library = db.createLibrary("Field recordings", kind, ["/media/recordings"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const collection = "a".repeat(64);
  const id = "6277b2e7-9da5-4bd4-9226-c5f75ca47e1f";
  const state: components["schemas"]["MediaState"] = {
    favorite: false, stars: 0, positionMs: 8000, completed: false, playbackVersion: 3, lastPlayedAt: null, playCount: 1,
  };
  const checkpoints: unknown[] = [];
  server.use(
    http.get(`/api/v1/libraries/${library.id}/spoken`, ({ request }) => HttpResponse.json({
      entries: new URL(request.url).searchParams.get("collection") === collection
        ? [{ kind: itemKind, id, title: "Dawn chorus", artist: null, discNumber: null, number: 2, durationMs: 120_000 }]
        : [{ kind: groupKind, id: collection, title: "Birdsong" }], nextCursor: null,
    })),
    http.post(`/api/v1/items/${id}/playback`, () => HttpResponse.json({
      mode: "direct", url: "/media/example.opus", expiresAt: new Date(Date.now() + 900_000).toISOString(),
    })),
    http.get(`/api/v1/me/items/${id}`, () => HttpResponse.json(state)),
    http.put(`/api/v1/me/items/${id}/progress`, async ({ request }) => {
      checkpoints.push(await request.json());
      return HttpResponse.json({ ...state, playbackVersion: 4 });
    }),
  );
  const user = userEvent.setup();
  const { container } = renderApp(`/library/${library.id}`);
  await user.click(await screen.findByRole("link", { name: "Birdsong" }, { timeout: 5000 }));
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

  const breadcrumb = screen.getByRole("navigation", { name: "当前位置" });
  await user.click(within(breadcrumb).getByRole("link"));
  await user.click(await screen.findByRole("link", { name: "Birdsong" }));
  await screen.findByRole("button", { name: "播放《Dawn chorus》" });
  expect(container.querySelector("audio")).toBeNull();
});
