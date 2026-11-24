/**
 * MSW handlers for the music app plus the existing personal-state,
 * playlist, lyrics, loudness, search and playback endpoints the app uses, over an in-memory
 * catalogue per music library. Registered by the dev mock (`npm run dev:mock`) and by tests.
 */
import { delay, http, HttpResponse } from "msw";
import type { MusicAlbum, MusicArtist, MusicTrack } from "@/modules/music/api";
import type { MockDb } from "../db";
import { buildCatalogue, rng, type MockAlbum, type MockCatalogue } from "./fixtures";
import { coverSvg, rasterise, synthesise } from "./media";

const BASE = "/api/v1";

/** Features a server with the music app reports (added to the mock's list unless it runs as M0). */
export const MUSIC_MOCK_FEATURES = ["music", "musicArtwork", "personal", "playlists", "musicLoudness", "musicSearch", "globalMusicSearch"] as const;

const BUNDLED = Object.fromEntries(
  Object.entries(import.meta.glob<string>("../provisional/artwork/album*.png", { eager: true, import: "default", query: "?url" })).map(([path, url]) => [
    path.slice("../provisional/artwork/".length, -".png".length),
    url,
  ]),
);

function fail(status: number, code: string, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

interface PersonalState {
  completed: boolean;
  positionMs: number;
  playbackVersion: number;
}

interface StoredPlaylist {
  id: string;
  name: string;
  version: number;
  trackIds: string[];
  updatedAt: number;
}

function encodeCursor(offset: number) {
  return btoa(`m:${offset}`);
}
function decodeCursor(cursor: string | null): number {
  if (!cursor) return 0;
  try {
    const match = /^m:(\d+)$/.exec(atob(cursor));
    return match ? Number(match[1]) : 0;
  } catch {
    return 0;
  }
}

function page<T>(rows: T[], url: URL) {
  const limit = Math.min(200, Math.max(1, Number(url.searchParams.get("limit") ?? 60) || 60));
  const offset = decodeCursor(url.searchParams.get("cursor"));
  const slice = rows.slice(offset, offset + limit);
  return { slice, nextCursor: offset + limit < rows.length ? encodeCursor(offset + limit) : null, total: rows.length };
}

function seeded<T>(rows: T[], seed: number): T[] {
  const random = rng(seed || 1);
  const out = [...rows];
  for (let index = out.length - 1; index > 0; index--) {
    const swap = Math.floor(random() * (index + 1));
    [out[index], out[swap]] = [out[swap]!, out[index]!];
  }
  return out;
}

const collator = new Intl.Collator("zh-CN", { numeric: true, sensitivity: "base" });
const text = (value: string | null | undefined) => value ?? "";
const matches = (q: string, ...values: Array<string | null | undefined>) => {
  const needle = q.trim().toLowerCase();
  return !needle || values.some((value) => value?.toLowerCase().includes(needle));
};

function sortRows<T>(rows: T[], url: URL, keys: Record<string, (row: T) => string | number>, fallback: string): T[] {
  const sort = url.searchParams.get("sort") ?? fallback;
  if (sort === "random") return seeded(rows, Number(url.searchParams.get("seed") ?? 1));
  const key = keys[sort] ?? keys[fallback]!;
  const numeric = !["title", "name", "artist", "album"].includes(sort);
  const defaultDesc = numeric && sort !== "year" && sort !== "duration";
  const order = url.searchParams.get("order") ?? (defaultDesc ? "desc" : "asc");
  const sign = order === "desc" ? -1 : 1;
  return [...rows].sort((a, b) => {
    const left = key(a);
    const right = key(b);
    const result = typeof left === "number" && typeof right === "number" ? left - right : collator.compare(String(left), String(right));
    return sign * result;
  });
}

function publicAlbum(album: MockAlbum): MusicAlbum {
  const { cover: _cover, coverSeed: _seed, ...rest } = album;
  return rest;
}

export function createMusicHandlers(db: MockDb, options: { latency?: number } = {}) {
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(latency + Math.round(Math.random() * latency)) : Promise.resolve());
  if (db.features.includes("assistant") && db.features.includes("home") && db.features.includes("tasks")) {
    Object.assign(db, { features: [...new Set([...db.features, ...MUSIC_MOCK_FEATURES])] });
  }

  const catalogues = new Map<string, MockCatalogue>();
  const catalogue = (libraryId: string): MockCatalogue | null => {
    const library = db.getLibrary(libraryId);
    if (!library || library.kind !== "music") return null;
    let found = catalogues.get(libraryId);
    if (!found) {
      found = buildCatalogue(libraryId, db.now());
      catalogues.set(libraryId, found);
    }
    return found;
  };
  const allCatalogues = () => db.libraries.filter((library) => library.kind === "music").map((library) => catalogue(library.id)!);
  const findTrack = (id: string) => {
    for (const cat of allCatalogues()) {
      const track = cat.tracks.find((entry) => entry.id === id);
      if (track) return { cat, track };
    }
    return null;
  };

  const personal = new Map<string, PersonalState>();
  const personalOf = (id: string) => {
    let state = personal.get(id);
    if (!state) {
      state = { completed: false, positionMs: 0, playbackVersion: 1 };
      personal.set(id, state);
    }
    return state;
  };
  const mediaState = (track: MusicTrack) => ({ ...personalOf(track.id), favorite: track.state.favorite, stars: track.state.stars, playCount: track.state.playCount, lastPlayedAt: track.state.lastPlayedAt });

  const playlists = new Map<string, StoredPlaylist>();
  let playlistsSeeded = false;
  const seedPlaylists = () => {
    if (playlistsSeeded) return;
    playlistsSeeded = true;
    const tracks = allCatalogues().flatMap((cat) => cat.tracks);
    if (tracks.length === 0) return;
    const pickTracks = (seed: number, count: number) => seeded(tracks, seed).slice(0, count).map((track) => track.id);
    const seeds: Array<[string, number, number]> = [["通勤路上", 11, 18], ["深夜写代码", 23, 32], ["周末早晨", 37, 12], ["Road Trip 2026", 41, 24]];
    seeds.forEach(([name, seed, count], index) => {
      const id = `00000000-0000-4000-8000-00000000000${index + 1}`;
      playlists.set(id, { id, name, version: 1, trackIds: pickTracks(seed, count), updatedAt: db.now() - index * 3_600_000 });
    });
  };

  type Guarded = { userId: string } | { response: Response };
  const guard = (request: Request): Guarded => {
    const header = request.headers.get("Authorization");
    const token = header ? /^Bearer\s+(.+)$/i.exec(header)?.[1] ?? null : new URL(request.url).searchParams.get("ticket") ? "ticket" : null;
    if (token === "ticket") return { userId: "ticket" };
    const user = db.userForToken(token);
    if (!user) return { response: fail(401, "unauthorized", "Missing or invalid access token") };
    return { userId: user.id };
  };

  const withCatalogue = async (request: Request, libraryId: string, run: (cat: MockCatalogue, url: URL) => Response | Promise<Response>) => {
    await wait();
    const auth = guard(request);
    if ("response" in auth) return auth.response;
    const cat = catalogue(libraryId);
    if (!cat) return fail(404, "not_found", "Music library not found");
    return run(cat, new URL(request.url));
  };

  const filterAlbums = (cat: MockCatalogue, url: URL) => {
    const genre = url.searchParams.get("genre");
    const yearFrom = Number(url.searchParams.get("yearFrom") ?? 0) || 0;
    const yearTo = Number(url.searchParams.get("yearTo") ?? 0) || 9999;
    const artistId = url.searchParams.get("artistId");
    const favorite = url.searchParams.get("favorite") === "true";
    const q = url.searchParams.get("q") ?? "";
    const rows = cat.albums.filter(
      (album) =>
        (!genre || album.genres.includes(genre)) &&
        (album.year ?? 0) >= yearFrom &&
        (album.year ?? 0) <= yearTo &&
        (!artistId || album.artistId === artistId) &&
        (!favorite || album.favorite) &&
        matches(q, album.name, album.artist),
    );
    return sortRows(rows, url, {
      title: (album) => text(album.name),
      artist: (album) => `${text(album.artist)}\u0000${album.year ?? 0}`,
      year: (album) => album.year ?? 0,
      added: (album) => Date.parse(album.addedAt),
      played: (album) => (album.lastPlayedAt ? Date.parse(album.lastPlayedAt) : 0),
      plays: (album) => album.playCount,
      rating: (album) => album.stars,
    }, "title");
  };

  const filterTracks = (cat: MockCatalogue, url: URL) => {
    const genre = url.searchParams.get("genre");
    const albumId = url.searchParams.get("albumId");
    const artistId = url.searchParams.get("artistId");
    const favorite = url.searchParams.get("favorite") === "true";
    const q = url.searchParams.get("q") ?? "";
    const rows = cat.tracks.filter(
      (track) =>
        (!genre || track.genres.includes(genre)) &&
        (!albumId || track.albumId === albumId) &&
        (!artistId || track.artistId === artistId) &&
        (!favorite || track.state.favorite) &&
        matches(q, track.title, track.artist, track.album),
    );
    return sortRows(rows, url, {
      title: (track) => track.title,
      artist: (track) => text(track.artist),
      album: (track) => `${text(track.album)}\u0000${String(track.discNumber ?? 0).padStart(3, "0")}${String(track.trackNumber ?? 0).padStart(4, "0")}`,
      year: (track) => track.year ?? 0,
      added: (track) => Date.parse(track.addedAt),
      duration: (track) => track.durationMs ?? 0,
      plays: (track) => track.state.playCount,
      played: (track) => (track.state.lastPlayedAt ? Date.parse(track.state.lastPlayedAt) : 0),
      rating: (track) => track.state.stars,
    }, "title");
  };

  const audio = (request: Request, id: string) => {
    const found = findTrack(id);
    if (!found) return fail(404, "not_found", "Track not found");
    const url = new URL(request.url);
    const startMs = Number(url.searchParams.get("startMs") ?? 0) || 0;
    const body = synthesise(id.charCodeAt(0) * 31 + id.charCodeAt(5), found.track.durationMs ?? 180_000, startMs);
    const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.get("Range") ?? "");
    const headers = { "Content-Type": "audio/wav", "Accept-Ranges": "bytes", "Cache-Control": "no-store" };
    if (range) {
      const start = Number(range[1]);
      const end = range[2] ? Math.min(Number(range[2]), body.length - 1) : body.length - 1;
      if (start >= body.length) return new HttpResponse(null, { status: 416, headers: { "Content-Range": `bytes */${body.length}` } });
      const chunk = body.slice(start, end + 1);
      return new HttpResponse(chunk, { status: 206, headers: { ...headers, "Content-Range": `bytes ${start}-${end}/${body.length}`, "Content-Length": String(chunk.length) } });
    }
    return new HttpResponse(body, { headers: { ...headers, "Content-Length": String(body.length) } });
  };

  const lib = `${BASE}/libraries/:id/music`;

  return [
    /* ------------------------------------------------------------ catalogue */
    http.get(`${lib}/albums`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat, url) => {
        const { slice, nextCursor, total } = page(filterAlbums(cat, url), url);
        return HttpResponse.json({ albums: slice.map(publicAlbum), nextCursor, total });
      }),
    ),
    http.get(`${lib}/albums/:albumId`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat) => {
        const album = cat.albums.find((entry) => entry.id === params.albumId);
        if (!album) return fail(404, "not_found", "Album not found");
        const tracks = cat.tracks
          .filter((track) => track.albumId === album.id)
          .sort((a, b) => (a.discNumber ?? 0) - (b.discNumber ?? 0) || (a.trackNumber ?? 0) - (b.trackNumber ?? 0));
        return HttpResponse.json({ album: publicAlbum(album), tracks });
      }),
    ),
    http.put(`${lib}/albums/:albumId/favorite`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat) => {
        const album = cat.albums.find((entry) => entry.id === params.albumId);
        if (!album) return fail(404, "not_found", "Album not found");
        album.favorite = true;
        return new HttpResponse(null, { status: 204 });
      }),
    ),
    http.delete(`${lib}/albums/:albumId/favorite`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat) => {
        const album = cat.albums.find((entry) => entry.id === params.albumId);
        if (!album) return fail(404, "not_found", "Album not found");
        album.favorite = false;
        return new HttpResponse(null, { status: 204 });
      }),
    ),
    http.get(`${lib}/artists`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat, url) => {
        const favorite = url.searchParams.get("favorite") === "true";
        const q = url.searchParams.get("q") ?? "";
        const rows = sortRows(
          cat.artists.filter((artist) => (!favorite || artist.favorite) && matches(q, artist.name)),
          url,
          { name: (artist: MusicArtist) => text(artist.name), albums: (artist) => artist.albumCount, tracks: (artist) => artist.trackCount, plays: (artist) => artist.playCount },
          "name",
        );
        const { slice, nextCursor, total } = page(rows, url);
        return HttpResponse.json({ artists: slice, nextCursor, total });
      }),
    ),
    http.get(`${lib}/artists/:artistId`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat) => {
        const artist = cat.artists.find((entry) => entry.id === params.artistId);
        if (!artist) return fail(404, "not_found", "Artist not found");
        const albums = cat.albums.filter((album) => album.artistId === artist.id).sort((a, b) => (b.year ?? 0) - (a.year ?? 0)).map(publicAlbum);
        const own = cat.tracks.filter((track) => track.artistId === artist.id);
        const top = own.some((track) => track.state.playCount > 0) ? [...own].sort((a, b) => b.state.playCount - a.state.playCount) : own;
        return HttpResponse.json({ artist, albums, topTracks: top.slice(0, 10) });
      }),
    ),
    http.put(`${lib}/artists/:artistId/favorite`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat) => {
        const artist = cat.artists.find((entry) => entry.id === params.artistId);
        if (!artist) return fail(404, "not_found", "Artist not found");
        artist.favorite = true;
        return new HttpResponse(null, { status: 204 });
      }),
    ),
    http.delete(`${lib}/artists/:artistId/favorite`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat) => {
        const artist = cat.artists.find((entry) => entry.id === params.artistId);
        if (!artist) return fail(404, "not_found", "Artist not found");
        artist.favorite = false;
        return new HttpResponse(null, { status: 204 });
      }),
    ),
    http.get(`${lib}/genres`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat) => {
        const genres = new Map<string, { name: string; albumCount: number; trackCount: number }>();
        for (const album of cat.albums) {
          for (const name of album.genres) {
            const entry = genres.get(name) ?? { name, albumCount: 0, trackCount: 0 };
            entry.albumCount += 1;
            entry.trackCount += album.trackCount;
            genres.set(name, entry);
          }
        }
        return HttpResponse.json({ genres: [...genres.values()].sort((a, b) => collator.compare(a.name, b.name)) });
      }),
    ),
    http.get(`${lib}/tracks`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat, url) => {
        const { slice, nextCursor, total } = page(filterTracks(cat, url), url);
        return HttpResponse.json({ tracks: slice, nextCursor, total });
      }),
    ),
    http.get(`${lib}/search`, ({ request, params }) =>
      withCatalogue(request, String(params.id), (cat, url) => {
        const q = url.searchParams.get("q") ?? "";
        const kind = url.searchParams.get("kind") ?? "track";
        const limit = Math.min(100, Number(url.searchParams.get("limit") ?? 24) || 24);
        const entries =
          kind === "artist"
            ? cat.artists.filter((artist) => matches(q, artist.name)).map((artist) => ({ id: artist.id, title: text(artist.name), artist: artist.name, artistId: artist.id, album: null }))
            : kind === "album"
              ? cat.albums.filter((album) => matches(q, album.name, album.artist)).map((album) => ({ id: album.id, title: text(album.name), artist: album.artist, artistId: album.artistId, album: album.name }))
              : cat.tracks.filter((track) => matches(q, track.title, track.artist, track.album)).map((track) => ({ id: track.id, title: track.title, artist: track.artist, artistId: track.artistId, album: track.album }));
        return HttpResponse.json({ entries: entries.slice(0, limit), nextCursor: null });
      }),
    ),
    http.get(`${BASE}/music/search`, async ({ request }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const url = new URL(request.url);
      const q = url.searchParams.get("q") ?? "";
      const kind = url.searchParams.get("kind") ?? "track";
      const limit = Math.min(100, Number(url.searchParams.get("limit") ?? 24) || 24);
      const all = db.libraries
        .filter((library) => library.kind === "music")
        .flatMap((library) => {
          const cat = catalogue(library.id);
          if (!cat) return [];
          const rows =
            kind === "artist"
              ? cat.artists.filter((artist) => matches(q, artist.name)).map((artist) => ({ id: artist.id, title: text(artist.name), artist: artist.name, artistId: artist.id, album: null }))
              : kind === "album"
                ? cat.albums.filter((album) => matches(q, album.name, album.artist)).map((album) => ({ id: album.id, title: text(album.name), artist: album.artist, artistId: album.artistId, album: album.name }))
                : cat.tracks.filter((track) => matches(q, track.title, track.artist, track.album)).map((track) => ({ id: track.id, title: track.title, artist: track.artist, artistId: track.artistId, album: track.album }));
          return rows.map((row) => ({ ...row, libraryId: library.id, libraryName: library.name }));
        });
      const offset = decodeCursor(url.searchParams.get("cursor"));
      const slice = all.slice(offset, offset + limit);
      return HttpResponse.json({ entries: slice, nextCursor: offset + limit < all.length ? encodeCursor(offset + limit) : null });
    }),
    http.post(`${BASE}/music/tracks/lookup`, async ({ request }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const body = (await request.json().catch(() => null)) as { ids?: unknown } | null;
      const ids = Array.isArray(body?.ids) ? body.ids.filter((id): id is string => typeof id === "string") : [];
      if (ids.length > 500) return fail(400, "invalid_request", "At most 500 ids");
      const tracks = ids.map((id) => findTrack(id)?.track).filter((track): track is MusicTrack => Boolean(track));
      return HttpResponse.json({ tracks });
    }),

    /* ------------------------------------------------------------ media */
    http.get(`${BASE}/items/:id/artwork`, async ({ request, params }) => {
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const found = findTrack(String(params.id));
      const album = found?.cat.albumOfTrack.get(found.track.id);
      if (!album) return fail(404, "not_found", "No artwork");
      const size = Math.min(1920, Math.max(48, Number(new URL(request.url).searchParams.get("size") ?? 480) || 480));
      if (album.cover && BUNDLED[album.cover]) {
        try {
          const response = await fetch(BUNDLED[album.cover]!);
          if (response.ok) {
            return new HttpResponse(await response.arrayBuffer(), { headers: { "Content-Type": "image/png", "Cache-Control": "public, max-age=3600" } });
          }
        } catch {
          // Node/jsdom tests cannot fetch dev-server relative URLs; fall through to generated SVG.
        }
      }
      const svg = coverSvg(album.coverSeed, album.name ?? "");
      const png = await rasterise(album.id, svg, size <= 120 ? 128 : size <= 300 ? 320 : 640);
      if (png) return new HttpResponse(png, { headers: { "Content-Type": "image/png", "Cache-Control": "public, max-age=3600" } });
      return new HttpResponse(svg, { headers: { "Content-Type": "image/svg+xml" } });
    }),
    http.get(`${BASE}/items/:id/lyrics`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const found = findTrack(String(params.id));
      if (!found) return fail(404, "not_found", "Track not found");
      const lyrics = found.cat.lyrics.get(found.track.id);
      return HttpResponse.json({ lyrics: lyrics ? { ...lyrics, source: "sidecar" } : null });
    }),
    http.get(`${BASE}/items/:id/loudness`, ({ request, params }) => {
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const found = findTrack(String(params.id));
      if (!found) return fail(404, "not_found", "Track not found");
      const lufs = found.cat.loudness.get(found.track.id);
      return HttpResponse.json({
        measurement: lufs == null ? null : { current: true, integratedLufs: Math.round(lufs * 10) / 10, rangeLu: 6.2, truePeakDbtp: -0.8, measuredAt: db.iso(-86_400_000) },
      });
    }),
    http.post(`${BASE}/items/:id/playback`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const id = String(params.id);
      if (!findTrack(id)) return fail(404, "not_found", "Track not found");
      const url = new URL(request.url);
      const format = url.searchParams.get("format");
      const ticket = `mock-${Math.random().toString(36).slice(2)}`;
      const transcode = format && format !== "original";
      const target = transcode
        ? `${BASE}/items/${encodeURIComponent(id)}/audio?ticket=${ticket}&format=${format}&startMs=${url.searchParams.get("startMs") ?? 0}`
        : `${BASE}/items/${encodeURIComponent(id)}/stream?ticket=${ticket}`;
      return HttpResponse.json({ url: target, mode: transcode ? "transcode" : "direct", expiresAt: db.iso(10 * 60_000) });
    }),
    http.get(`${BASE}/items/:id/stream`, ({ request, params }) => audio(request, String(params.id))),
    http.get(`${BASE}/items/:id/audio`, ({ request, params }) => audio(request, String(params.id))),

    /* ------------------------------------------------------------ personal state */
    http.get(`${BASE}/me/items/:id`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const found = findTrack(String(params.id));
      return found ? HttpResponse.json(mediaState(found.track)) : fail(404, "not_found", "Item not found");
    }),
    http.put(`${BASE}/me/items/:id/favorite`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const found = findTrack(String(params.id));
      if (!found) return fail(404, "not_found", "Item not found");
      const body = (await request.json().catch(() => null)) as { favorite?: boolean } | null;
      found.track.state.favorite = Boolean(body?.favorite);
      return HttpResponse.json(mediaState(found.track));
    }),
    http.put(`${BASE}/me/items/:id/rating`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const found = findTrack(String(params.id));
      if (!found) return fail(404, "not_found", "Item not found");
      const body = (await request.json().catch(() => null)) as { stars?: number } | null;
      const stars = Number(body?.stars ?? 0);
      if (!Number.isInteger(stars) || stars < 0 || stars > 5) return fail(400, "invalid_request", "stars must be 0–5");
      found.track.state.stars = stars;
      const album = found.cat.albumOfTrack.get(found.track.id);
      if (album) {
        const rated = found.cat.tracks.filter((t) => t.albumId === album.id && t.state.stars > 0);
        album.stars = rated.length ? Math.round(rated.reduce((s, t) => s + t.state.stars, 0) / rated.length) : 0;
      }
      return HttpResponse.json(mediaState(found.track));
    }),
    http.put(`${BASE}/me/items/:id/progress`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const found = findTrack(String(params.id));
      if (!found) return fail(404, "not_found", "Item not found");
      const body = (await request.json().catch(() => null)) as { completed?: boolean; positionMs?: number; expectedVersion?: number } | null;
      const state = personalOf(found.track.id);
      if (body?.expectedVersion !== state.playbackVersion) return fail(409, "conflict", "Playback state changed elsewhere");
      state.playbackVersion += 1;
      state.completed = Boolean(body.completed);
      state.positionMs = body.completed ? 0 : Math.max(0, Number(body.positionMs ?? 0));
      if (body.completed) {
        found.track.state.playCount += 1;
        found.track.state.lastPlayedAt = db.iso();
        const album = found.cat.albumOfTrack.get(found.track.id);
        if (album) {
          album.playCount += 1;
          album.lastPlayedAt = found.track.state.lastPlayedAt;
        }
        const artist = found.cat.artists.find((a) => a.id === found.track.artistId);
        if (artist) artist.playCount += 1;
      }
      return HttpResponse.json(mediaState(found.track));
    }),

    /* ------------------------------------------------------------ playlists */
    http.get(`${BASE}/me/playlists`, async ({ request }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      seedPlaylists();
      const rows = [...playlists.values()].sort((a, b) => b.updatedAt - a.updatedAt);
      return HttpResponse.json({ playlists: rows.map(({ id, name, version }) => ({ id, name, version })), nextBefore: null });
    }),
    http.get(`${BASE}/me/playlists/:id`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      seedPlaylists();
      const playlist = playlists.get(String(params.id));
      if (!playlist) return fail(404, "not_found", "Playlist not found");
      return HttpResponse.json({ id: playlist.id, name: playlist.name, version: playlist.version, trackIds: playlist.trackIds });
    }),
    http.get(`${BASE}/me/playlists/:id/tracks`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      seedPlaylists();
      const playlist = playlists.get(String(params.id));
      if (!playlist) return fail(404, "not_found", "Playlist not found");
      const url = new URL(request.url);
      if (Number(url.searchParams.get("version")) !== playlist.version) return fail(409, "conflict", "Playlist changed");
      const after = Number(url.searchParams.get("after") ?? -1);
      const limit = Math.min(200, Number(url.searchParams.get("limit") ?? 100) || 100);
      const rows = playlist.trackIds
        .map((itemId, position) => ({ itemId, position, track: findTrack(itemId)?.track }))
        .filter((row) => row.position > after)
        .slice(0, limit);
      return HttpResponse.json({
        tracks: rows.map(({ itemId, position, track }) => ({ itemId, position, title: track?.title ?? itemId, artist: track?.artist ?? null, album: track?.album ?? null, durationMs: track?.durationMs ?? null, libraryId: track?.libraryId ?? "" })),
        nextAfter: rows.length === limit ? rows.at(-1)!.position : null,
      });
    }),
    http.put(`${BASE}/me/playlists/:id`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      seedPlaylists();
      const id = String(params.id);
      const body = (await request.json().catch(() => null)) as { name?: string; trackIds?: string[]; expectedVersion?: number } | null;
      const existing = playlists.get(id);
      const expected = body?.expectedVersion ?? -1;
      if ((existing ? existing.version : 0) !== expected) return fail(409, "conflict", "Playlist changed");
      const name = body?.name?.trim();
      if (!name) return fail(400, "invalid_request", "Name required");
      const next: StoredPlaylist = { id, name, version: expected + 1, trackIds: body?.trackIds ?? [], updatedAt: db.now() };
      playlists.set(id, next);
      return HttpResponse.json({ id, name, version: next.version, trackIds: next.trackIds });
    }),
    http.delete(`${BASE}/me/playlists/:id`, async ({ request, params }) => {
      await wait();
      const auth = guard(request);
      if ("response" in auth) return auth.response;
      const playlist = playlists.get(String(params.id));
      if (!playlist) return fail(404, "not_found", "Playlist not found");
      if (Number(new URL(request.url).searchParams.get("version")) !== playlist.version) return fail(409, "conflict", "Playlist changed");
      playlists.delete(playlist.id);
      return new HttpResponse(null, { status: 204 });
    }),
  ];
}
