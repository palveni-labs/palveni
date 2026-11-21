import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

it("opens homonymous recordings, artists and albums in their own libraries from the command palette", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "music", "globalMusicSearch", "personal"] });
  const libraries = ["North archive", "South archive"].map((name, i) => db.createLibrary(name, "music", [`/media/set-${i}`]));
  signIn(db, MOCK_ACCOUNTS.user.username);
  const artist = "a".repeat(64), album = "b".repeat(64);
  const tracks = ["north-track", "south-track"];
  const original = { positionMs: 0, completed: false, revision: 0 };
  server.use(
    http.get("/api/v1/music/search", ({ request }) => {
      const params = new URL(request.url).searchParams;
      const kind = params.get("kind");
      if (params.get("q") !== "harbour") return HttpResponse.json({ entries: [], nextCursor: null });
      return HttpResponse.json({
        entries: libraries.map((library, i) => ({
          libraryId: library.id, libraryName: library.name, title: "Harbour",
          id: kind === "album" ? album : kind === "artist" ? artist : tracks[i],
          artistId: kind === "album" ? artist : null, artist: kind === "artist" ? null : "Harbour", album: null,
        })),
        nextCursor: null,
      });
    }),
    http.post("/api/v1/items/south-track/playback", () => HttpResponse.json({ mode: "direct", url: "/audio/south", expiresAt: "2099-01-01T00:00:00Z" })),
    http.get("/api/v1/me/items/south-track", () => HttpResponse.json(original)),
    http.get<{ library: string }>("/api/v1/libraries/:library/music", ({ request, params }) => {
      const q = new URL(request.url).searchParams;
      return HttpResponse.json({ entries: q.has("album")
        ? [{ kind: "track", id: "found", title: `Recording from ${params.library}`, artist: "Harbour" }]
        : [{ kind: "album", id: album, name: `Album from ${params.library}`, artworkItemId: null }], nextCursor: null });
    }),
  );
  const user = userEvent.setup();
  const { router } = renderApp("/");
  await screen.findByRole("region", { name: "媒体库" });
  await user.keyboard("{Control>}k{/Control}");
  const palette = within(await screen.findByRole("dialog"));
  await user.type(palette.getByRole("searchbox"), "harbour");
  await user.click(await palette.findByText("在音乐库中搜索“harbour”"));
  await user.click(await screen.findByRole("button", { name: "播放《Harbour》 · South archive" }));
  await waitFor(() => expect(document.querySelector("audio")).toHaveAttribute("src", "/audio/south"));
  await user.click(screen.getByRole("button", { name: "专辑艺人" }));
  await user.click(await screen.findByRole("link", { name: /Harbour North archive/ }));
  await screen.findByText(`Album from ${libraries[0]!.id}`);
  expect(router.state.location.search).toMatchObject({ view: "music", artist });
  router.history.back();
  await user.click(await screen.findByRole("button", { name: "专辑" }));
  await user.click(await screen.findByRole("link", { name: /Harbour South archive/ }));
  await screen.findByText(`Recording from ${libraries[1]!.id}`);
  expect(router.state.location.search).toMatchObject({ view: "music", artist, album });
}, 15_000);

it("pages the global search and restarts at the first page when a source changes", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "globalMusicSearch"] });
  signIn(db, MOCK_ACCOUNTS.user.username);
  let changed = false;
  server.use(http.get("/api/v1/music/search", ({ request }) => {
    const params = new URL(request.url).searchParams;
    if (params.get("q") !== "2024") return HttpResponse.json({ entries: [], nextCursor: null });
    const cursor = params.get("cursor");
    if (cursor === "last") {
      changed = true;
      return HttpResponse.json({ error: { code: "conflict", message: "Changed" } }, { status: 409 });
    }
    const title = changed ? "Revised" : cursor ? "Second" : "First";
    return HttpResponse.json({
      entries: [{ title, id: title, artist: null, album: null, artistId: null, libraryId: "lib", libraryName: "Archive" }],
      nextCursor: changed ? null : cursor ? "last" : "next",
    });
  }));
  const user = userEvent.setup();
  renderApp("/library/music-search");
  await user.type(await screen.findByRole("searchbox", { name: "搜索所有音乐库" }), "2024{Enter}");
  await screen.findByText("First");
  await user.click(screen.getByRole("button", { name: "下一页" }));
  await screen.findByText("Second");
  expect(screen.queryByText("First")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "下一页" }));
  await screen.findByText("Revised");
  expect(screen.queryByRole("button", { name: "下一页" })).not.toBeInTheDocument();
});
