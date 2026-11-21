import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

it("searches the selected library beyond its browse rows, retries a changed page and opens an album", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "music", "musicSearch", "personal"] });
  const lib = db.createLibrary("Field recordings", "music", ["/media/recordings"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const artist = "a".repeat(64), album = "b".repeat(64);
  let refreshed = false;
  server.use(
    http.get(`/api/v1/libraries/${lib.id}/music`, ({ request }) => HttpResponse.json({
      entries: new URL(request.url).searchParams.has("album")
        ? [{ kind: "track", id: "real-track", title: "Distant shore", artist: "Owl", trackNumber: 1, discNumber: 1, durationMs: 50 }]
        : [], nextCursor: null,
    })),
    http.get(`/api/v1/libraries/${lib.id}/music/search`, ({ request }) => {
      const url = new URL(request.url);
      if (url.searchParams.get("q") !== "shore") return HttpResponse.json({ entries: [], nextCursor: null });
      if (url.searchParams.get("kind") === "album") return HttpResponse.json({
        entries: [{ id: album, title: "West shore", artist: "Owl", album: null, artistId: artist }], nextCursor: null,
      });
      if (url.searchParams.has("cursor")) {
        refreshed = true;
        return HttpResponse.json({ error: { code: "conflict", message: "Changed" } }, { status: 409 });
      }
      return HttpResponse.json({ entries: [{ id: "song", title: refreshed ? "New shore" : "First shore", artist: null, album: null, artistId: null }], nextCursor: refreshed ? null : "cursor" });
    }),
  );
  const user = userEvent.setup();
  const { router } = renderApp(`/library/${lib.id}?view=music`);
  await user.type(await screen.findByRole("searchbox", { name: "搜索这个音乐库" }), "shore");
  await screen.findByRole("button", { name: "播放《First shore》" });
  await user.click(screen.getByRole("button", { name: "加载更多" }));
  await screen.findByRole("button", { name: "播放《New shore》" });
  expect(screen.queryByText("First shore")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "专辑" }));
  await user.click(await screen.findByRole("link", { name: /West shore/ }));
  await screen.findByText("Distant shore");
  await waitFor(() => expect(router.state.location.search).toMatchObject({ view: "music", artist, album }));
});
