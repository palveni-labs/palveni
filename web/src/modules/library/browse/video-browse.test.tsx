import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { afterEach, expect, it, vi } from "vitest";
import type { components } from "@/api/generated";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

afterEach(() => vi.restoreAllMocks());

it("opens an authenticated poster wall, navigates episodes and resumes when the stream ticket expires", async () => {
  vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(640);
  vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(960);
  const db = installMockDb({ features: [...M0_FEATURES, "video"] });
  const lib = db.createLibrary("Cinema", "video", ["/media/cinema"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const item = "23587a91-9e35-4821-8a48-3aeddba77d11";
  const series = "a".repeat(64);
  const work: components["schemas"]["VideoEntry"] = {
    id: series,
    kind: "series",
    libraryId: lib.id,
    title: "Harbour",
    fileCount: 1,
    year: 2024,
    posterItemId: item,
    itemId: null,
    overview: null,
    seasonNumber: null,
    episodeNumber: null,
    seriesId: null,
    seriesTitle: null,
    originalTitle: null,
    tagline: null,
    genres: [],
    runtimeMs: null,
    communityRating: null,
    officialRating: null,
    premiered: null,
    backdropItemId: null,
    thumbItemId: null,
    addedAt: "2026-11-22T00:00:00Z",
    state: null,
  };
  const state: components["schemas"]["MediaState"] = {
    favorite: false,
    stars: 0,
    positionMs: 8000,
    completed: false,
    playbackVersion: 3,
    lastPlayedAt: null,
    playCount: 0,
  };
  let tickets = 0;
  const pictureRequests: string[] = [];
  server.use(
    http.get(`/api/v1/libraries/${lib.id}/videos`, ({ request }) => {
      const params = new URL(request.url).searchParams;
      if (!params.has("series")) {
        const other = { ...work, id: "b".repeat(64), title: "Archive", posterItemId: null };
        const entries = params.get("order") === "poster" ? [work, other] : [other, work];
        return HttpResponse.json({ entries, nextCursor: null });
      }
      if (params.has("order")) return new HttpResponse(null, { status: 400 });
      let entry = work;
      if (params.get("series") === series) {
        entry =
          params.get("season") === "2"
            ? {
                ...work,
                id: item,
                title: "Breakwater",
                kind: "episode",
                itemId: item,
                seasonNumber: 2,
                episodeNumber: 3,
              }
            : { ...work, id: "2", kind: "season", seasonNumber: 2 };
      }
      return HttpResponse.json({ entries: [entry], nextCursor: null });
    }),
    http.get(`/api/v1/items/${item}/poster`, ({ request }) => {
      if (request.headers.get("Authorization")?.startsWith("Bearer ")) {
        pictureRequests.push(
          new URL(request.url).searchParams.get("level") ?? "",
        );
      }
      return new HttpResponse(null, { status: 404 });
    }),
    http.post(`/api/v1/items/${item}/playback`, () => {
      tickets += 1;
      return HttpResponse.json({
        mode: "direct",
        url: `/media/video-${tickets}`,
        expiresAt: new Date(
          Date.now() + (tickets === 1 ? -1000 : 900_000),
        ).toISOString(),
      });
    }),
    http.get(`/api/v1/me/items/${item}`, () => HttpResponse.json(state)),
    http.put(`/api/v1/me/items/${item}/progress`, () =>
      HttpResponse.json({ ...state, playbackVersion: 4 }),
    ),
  );
  const user = userEvent.setup();
  const { container } = renderApp(`/library/${lib.id}`);
  await screen.findByRole("button", { name: /Harbour/ }, { timeout: 5000 });
  const firstWork = () => screen.getAllByRole("button", { name: /Harbour|Archive/ })[0];
  expect(firstWork()).toHaveAccessibleName(/Harbour/);
  await user.selectOptions(screen.getByRole("combobox", { name: "作品排序" }), "title");
  await waitFor(() => expect(firstWork()).toHaveAccessibleName(/Archive/));
  await user.click(
    await screen.findByRole("button", { name: /Harbour/ }, { timeout: 5000 }),
  );
  await user.click(await screen.findByRole("button", { name: /第 2 季/ }));
  await user.click(await screen.findByRole("button", { name: /Breakwater/ }));
  await user.click(await screen.findByRole("button", { name: "预览原片" }));
  const video = await waitFor(() => {
    const element = container.querySelector("video");
    expect(element).toHaveAttribute("src", "/media/video-1");
    return element!;
  });
  Object.defineProperty(video, "duration", { value: 300 });
  fireEvent.loadedMetadata(video);
  expect(video.currentTime).toBe(8);
  fireEvent.playing(video);
  video.currentTime = 27;
  fireEvent.timeUpdate(video);
  fireEvent.error(video);
  await waitFor(() => expect(video).toHaveAttribute("src", "/media/video-2"));
  video.currentTime = 0;
  fireEvent.loadedMetadata(video);
  expect(video.currentTime).toBe(27);
  expect(pictureRequests).toEqual(
    expect.arrayContaining(["work", "season", "episode"]),
  );
});

it("searches all titles by name and type, keeps filters across seasons and can clear an empty result", async () => {
  vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(640);
  vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(960);
  const db = installMockDb({ features: [...M0_FEATURES, "video"] });
  const lib = db.createLibrary("TV", "video", ["/media/tv"]);
  signIn(db, MOCK_ACCOUNTS.user.username);
  const series = "f".repeat(64);
  const entry: components["schemas"]["VideoEntry"] = {
    id: series,
    kind: "series",
    libraryId: lib.id,
    title: "Beyond the Sea",
    fileCount: 4,
    year: null,
    seasonNumber: null,
    episodeNumber: null,
    overview: null,
    posterItemId: null,
    itemId: null,
    seriesId: null,
    seriesTitle: null,
    originalTitle: null,
    tagline: null,
    genres: [],
    runtimeMs: null,
    communityRating: null,
    officialRating: null,
    premiered: null,
    backdropItemId: null,
    thumbItemId: null,
    addedAt: "2026-11-22T00:00:00Z",
    state: null,
  };
  const requests: URLSearchParams[] = [];
  server.use(http.get(`/api/v1/libraries/${lib.id}/videos`, ({ request }) => {
    const params = new URL(request.url).searchParams;
    requests.push(params);
    if (params.has("series")) {
      if (params.has("q") || params.has("kind")) return new HttpResponse(null, { status: 400 });
      return HttpResponse.json({ entries: [{ ...entry, kind: "season", id: "1", seasonNumber: 1 }], nextCursor: null });
    }
    const entries = params.get("q") === "Sea" && params.get("kind") !== "movie"
      ? [entry]
      : params.has("q") ? [] : [{ ...entry, kind: "movie", title: "First Page" }];
    return HttpResponse.json({ entries, nextCursor: null });
  }));
  const user = userEvent.setup();
  renderApp(`/library/${lib.id}`);
  await screen.findByRole("button", { name: /First Page/ });
  const kind = () => screen.getByRole("combobox", { name: "作品类型" });
  await user.selectOptions(kind(), "series");
  await user.type(screen.getByRole("searchbox", { name: "搜索片名" }), "Sea{Enter}");
  await user.click(await screen.findByRole("button", { name: /Beyond the Sea/ }));
  await screen.findByRole("button", { name: /第 1 季/ });
  expect(requests.at(-1)?.has("q")).toBe(false);
  await user.click(within(screen.getByRole("navigation", { name: "当前位置" })).getByRole("link", { name: "作品" }));
  await screen.findByRole("button", { name: /Beyond the Sea/ });
  expect(screen.getByRole("searchbox")).toHaveValue("Sea");
  expect(kind()).toHaveValue("series");
  await user.selectOptions(kind(), "movie");
  await screen.findByText("没有符合条件的作品");
  await user.click(screen.getByRole("button", { name: "清除筛选" }));
  await screen.findByRole("button", { name: /First Page/ });
  expect(screen.getByRole("searchbox")).toHaveValue("");
});
