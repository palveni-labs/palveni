import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { afterEach, expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

afterEach(() => vi.restoreAllMocks());

function openAlbum(width: number) {
  vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(width);
  const db = installMockDb({ features: [...M0_FEATURES, "music", "musicArtwork", "personal"] });
  const library = db.createLibrary("Listening room", "music", ["/media/room"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const browseQueries: URLSearchParams[] = [];
  const requestedCovers: string[] = [];
  server.use(
    http.get(`/api/v1/libraries/${library.id}/music`, ({ request }) => {
      const params = new URL(request.url).searchParams;
      browseQueries.push(params);
      const album = params.get("album");
      const second = params.has("cursor");
      if (!album) {
        const id = second ? "west" : "east";
        return HttpResponse.json({ entries: [{ kind: "album", id, name: `Sessions ${id}`, artworkItemId: `art-${id}` }], nextCursor: second ? null : "more-albums" });
      }
      const number = second ? 2 : 1;
      return HttpResponse.json({ entries: [{
        kind: "track", id: `${album}-${number}`, title: `Recording ${album} ${number}`,
        discNumber: 1, trackNumber: number, artist: "Shore quartet", durationMs: 60_000,
      }], nextCursor: second ? null : "more-tracks" });
    }),
    http.get<{ id: string }>("/api/v1/items/:id/artwork", ({ params }) => {
      requestedCovers.push(params.id);
      return new HttpResponse(null, { status: 404 });
    }),
    http.post("/api/v1/items/west-1/playback", () => HttpResponse.json({ mode: "direct", url: "/audio/west-1", expiresAt: "2099-01-01T00:00:00Z" })),
    http.get("/api/v1/me/items/west-1", () => HttpResponse.json({ positionMs: 0, completed: false, playbackVersion: 0 })),
  );
  renderApp(`/library/${library.id}?view=music&artist=quartet&artistName=Shore+quartet&album=east&albumName=Sessions+east`);
  return { browseQueries, requestedCovers };
}

it("pages the album list and tracks independently and plays from the newly selected album", async () => {
  const user = userEvent.setup();
  const { requestedCovers } = openAlbum(900);
  const albums = within(await screen.findByRole("region", { name: "专辑" }));
  const tracks = within(await screen.findByRole("region", { name: "曲目" }));
  expect(await albums.findByRole("link", { name: "Sessions east" })).toHaveAttribute("aria-current", "page");
  await tracks.findByText("Recording east 1");
  expect(tracks.queryByText(/1 首/)).not.toBeInTheDocument();
  await user.click(tracks.getByRole("button", { name: "加载更多" }));
  await tracks.findByText("Recording east 2");
  await tracks.findByText(/2 首/);
  await user.click(albums.getByRole("button", { name: "加载更多" }));
  await user.click(await albums.findByRole("link", { name: "Sessions west" }));
  await user.click(await screen.findByRole("button", { name: "播放《Recording west 1》" }));
  await waitFor(() => expect(document.querySelector("audio")).toHaveAttribute("src", "/audio/west-1"));
  expect(screen.queryByText("Recording east 1")).not.toBeInTheDocument();
  expect(albums.getByRole("link", { name: "Sessions west" })).toHaveAttribute("aria-current", "page");
  expect(requestedCovers).toEqual(expect.arrayContaining(["art-east", "art-west"]));
});

it("opens a deep-linked album on a narrow screen with its track artwork and no album-list fetch", async () => {
  const { browseQueries, requestedCovers } = openAlbum(390);
  await screen.findByText("Recording east 1");
  expect(screen.queryByRole("region", { name: "专辑" })).not.toBeInTheDocument();
  expect(browseQueries.every((params) => params.get("album") === "east")).toBe(true);
  await waitFor(() => expect(requestedCovers).toContain("east-1"));
});
