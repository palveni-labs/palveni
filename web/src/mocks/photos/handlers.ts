import { delay, http, HttpResponse } from "msw";
import type {
  CreatePhotoAlbumRequest,
  CreatePhotoPersonRequest,
  PhotoUploadRequest,
  UpdatePhotoAlbumRequest,
  UpdatePhotoPersonRequest,
} from "@/modules/photos/api";
import type { MockDb } from "../db";
import type { HandlerOptions } from "../handlers";
import { PhotosMockState } from "./state";

const BASE = "/api/v1";

function fail(status: number, code: string, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

function bearer(request: Request): string | null {
  const header = request.headers.get("Authorization");
  const match = header ? /^Bearer\s+(.+)$/i.exec(header) : null;
  return match?.[1] ?? null;
}

function renderPhotoSvg(id: string, size: number): string {
  const hash = [...id].reduce((acc, ch) => (acc * 31 + ch.charCodeAt(0)) >>> 0, 7);
  // Warm amber / slate / teal hues — never purple/violet.
  const palettes = [
    ["#1f3a4d", "#c27a2c"],
    ["#233d32", "#b88746"],
    ["#2d333b", "#8a5a36"],
    ["#1b3644", "#4f7d73"],
  ] as const;
  const [c1, c2] = palettes[hash % palettes.length]!;
  const w = Math.max(96, Math.min(1920, size));
  const h = Math.round(w * 0.75);
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
    <defs>
      <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="${c1}" />
        <stop offset="100%" stop-color="${c2}" />
      </linearGradient>
    </defs>
    <rect width="${w}" height="${h}" fill="url(#g)" />
    <circle cx="${Math.round(w * 0.72)}" cy="${Math.round(h * 0.3)}" r="${Math.round(h * 0.12)}" fill="rgba(255,255,255,0.28)" />
    <path d="M0 ${h} L${Math.round(w * 0.35)} ${Math.round(h * 0.52)} L${Math.round(w * 0.65)} ${Math.round(h * 0.78)} L${w} ${Math.round(h * 0.44)} L${w} ${h} Z" fill="rgba(0,0,0,0.32)" />
  </svg>`;
}

export function createPhotosHandlers(db: MockDb, options: HandlerOptions = {}) {
  const state = new PhotosMockState(db.now);
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(Math.round(latency * 0.5)) : Promise.resolve());

  const authUser = (request: Request) => {
    const user = db.userForToken(bearer(request));
    if (!user) return { error: fail(401, "unauthorized", "Missing or invalid access token") };
    if (!db.features.includes("photos")) return { error: fail(404, "not_found", "Feature not enabled") };
    return { user };
  };

  return [
    /* Intercept /me/items/:id/favorite ONLY for photo items; pass through for music/video items */
    http.put(`${BASE}/me/items/:id/favorite`, async ({ request, params }) => {
      const id = String(params.id);
      if (!db.features.includes("photos") || !state.hasPhoto(id)) {
        return;
      }
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const body = (await request.json().catch(() => ({}))) as { favorite?: boolean };
      const fav = Boolean(body.favorite);
      state.setFavorite(auth.user.id, id, fav);
      return HttpResponse.json({
        favorite: fav,
        stars: 0,
        positionMs: 0,
        completed: false,
        playCount: 0,
        lastPlayedAt: null,
        playbackVersion: 1,
      });
    }),

    http.get(`${BASE}/photos/timeline`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const q = new URL(request.url).searchParams;
      const libraryId = q.get("libraryId") ?? undefined;
      if (libraryId && !db.getLibrary(libraryId)) {
        return fail(404, "not_found", "Library not found");
      }
      const page = state.timeline(auth.user.id, {
        libraryId,
        albumId: q.get("albumId") ?? undefined,
        favorite: q.has("favorite") ? q.get("favorite") === "true" : undefined,
        hasLocation: q.has("hasLocation") ? q.get("hasLocation") === "true" : undefined,
        camera: q.get("camera") ?? undefined,
        month: q.get("month") ?? undefined,
        place: q.get("place") ?? undefined,
        q: q.get("q") ?? undefined,
        cursor: q.get("cursor") ?? undefined,
        limit: q.has("limit") ? Number(q.get("limit")) : undefined,
      });
      return page ? HttpResponse.json(page) : fail(404, "not_found", "Photo album not found");
    }),

    http.get(`${BASE}/photos/memories`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      return HttpResponse.json(state.memories(auth.user.id));
    }),

    http.get(`${BASE}/items/:id/photo-info`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const info = state.photoInfo(auth.user.id, String(params.id), auth.user.role === "admin");
      return info ? HttpResponse.json(info) : fail(404, "not_found", "Photo not found");
    }),

    http.get(`${BASE}/items/:id/photo`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const id = String(params.id);
      if (!state.hasPhoto(id)) return fail(404, "not_found", "Photo not found");
      const q = new URL(request.url).searchParams;
      const original = q.get("original") === "true";
      const size = Number(q.get("size") ?? 1920) || 1920;
      const svg = renderPhotoSvg(id, size);
      return new HttpResponse(svg, {
        headers: {
          "Content-Type": "image/svg+xml; charset=utf-8",
          "Cache-Control": "private, no-cache",
          ...(original ? { "Content-Disposition": `attachment; filename="photo.svg"` } : {}),
        },
      });
    }),

    http.get(`${BASE}/me/photo-albums`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      return HttpResponse.json({ albums: state.listAlbums(auth.user.id) });
    }),

    http.post(`${BASE}/me/photo-albums`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const body = (await request.json().catch(() => null)) as CreatePhotoAlbumRequest | null;
      if (!body?.title?.trim()) return fail(400, "validation_failed", "title is required");
      const created = state.createAlbum(auth.user.id, body);
      return HttpResponse.json(created, { status: 201 });
    }),

    http.get(`${BASE}/me/photo-albums/:id`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const album = state.getAlbum(auth.user.id, String(params.id));
      return album ? HttpResponse.json(album) : fail(404, "not_found", "Photo album not found");
    }),

    http.patch(`${BASE}/me/photo-albums/:id`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const body = (await request.json().catch(() => ({}))) as UpdatePhotoAlbumRequest;
      const updated = state.updateAlbum(auth.user.id, String(params.id), body);
      return updated ? HttpResponse.json(updated) : fail(404, "not_found", "Photo album not found");
    }),

    http.delete(`${BASE}/me/photo-albums/:id`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      return state.deleteAlbum(auth.user.id, String(params.id))
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Photo album not found");
    }),

    http.post(`${BASE}/me/photo-albums/:id/items`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const body = (await request.json().catch(() => ({}))) as { itemIds?: string[] };
      if (!Array.isArray(body.itemIds) || body.itemIds.length === 0) {
        return fail(400, "validation_failed", "itemIds is required");
      }
      const updated = state.addAlbumItems(auth.user.id, String(params.id), body.itemIds);
      return updated ? HttpResponse.json(updated) : fail(404, "not_found", "Photo album not found");
    }),

    http.delete(`${BASE}/me/photo-albums/:id/items`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const body = (await request.json().catch(() => ({}))) as { itemIds?: string[] };
      if (!Array.isArray(body.itemIds) || body.itemIds.length === 0) {
        return fail(400, "validation_failed", "itemIds is required");
      }
      const updated = state.removeAlbumItems(auth.user.id, String(params.id), body.itemIds);
      return updated ? HttpResponse.json(updated) : fail(404, "not_found", "Photo album not found");
    }),

    http.post(`${BASE}/libraries/:id/photos/upload`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const libraryId = String(params.id);
      const library = db.getLibrary(libraryId);
      if (!library) return fail(404, "not_found", "Library not found");
      if (library.kind !== "photo") return fail(400, "validation_failed", "Not a photo library");
      if (library.paths.some((p) => /readonly|ro\b/i.test(p))) {
        return fail(409, "conflict", "Photo library storage is read-only");
      }
      const body = (await request.json().catch(() => null)) as PhotoUploadRequest | null;
      if (!body?.fileName || !body.dataBase64) {
        return fail(400, "validation_failed", "fileName and dataBase64 are required");
      }
      const info = state.uploadPhoto(
        auth.user.id,
        auth.user.role === "admin",
        libraryId,
        body.fileName,
      );
      library.itemCount += 1;
      return HttpResponse.json(info, { status: 201 });
    }),

    http.post(`${BASE}/photos/analyze`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      return HttpResponse.json(state.analyze());
    }),

    http.get(`${BASE}/photos/search`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const q = new URL(request.url).searchParams;
      const query = q.get("q") ?? "";
      const personId = q.get("personId") ?? undefined;
      const limit = q.has("limit") ? Number(q.get("limit")) : 60;
      return HttpResponse.json(state.search(auth.user.id, query, personId, limit));
    }),

    http.get(`${BASE}/photos/duplicates`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const q = new URL(request.url).searchParams;
      const threshold = q.has("threshold") ? Number(q.get("threshold")) : 6;
      return HttpResponse.json(state.duplicates(auth.user.id, threshold));
    }),

    http.get(`${BASE}/me/photo-people`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      return HttpResponse.json({ people: state.listPeople(auth.user.id) });
    }),

    http.post(`${BASE}/me/photo-people`, async ({ request }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const body = (await request.json().catch(() => null)) as CreatePhotoPersonRequest | null;
      if (!body?.name?.trim()) return fail(400, "validation_failed", "name is required");
      return HttpResponse.json(state.createPerson(auth.user.id, body), { status: 201 });
    }),

    http.patch(`${BASE}/me/photo-people/:id`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const body = (await request.json().catch(() => ({}))) as UpdatePhotoPersonRequest;
      const updated = state.updatePerson(auth.user.id, String(params.id), body);
      return updated ? HttpResponse.json(updated) : fail(404, "not_found", "Photo person not found");
    }),

    http.delete(`${BASE}/me/photo-people/:id`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      return state.deletePerson(auth.user.id, String(params.id))
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Photo person not found");
    }),

    http.get(`${BASE}/items/:id/photo-people`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const itemId = String(params.id);
      if (!state.hasPhoto(itemId)) return fail(404, "not_found", "Photo not found");
      return HttpResponse.json({ people: state.listItemPeople(auth.user.id, itemId) });
    }),

    http.post(`${BASE}/items/:id/photo-people/:personId`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      const updated = state.tagPerson(auth.user.id, String(params.id), String(params.personId));
      return updated ? HttpResponse.json(updated) : fail(404, "not_found", "Photo or person not found");
    }),

    http.delete(`${BASE}/items/:id/photo-people/:personId`, async ({ request, params }) => {
      await wait();
      const auth = authUser(request);
      if ("error" in auth) return auth.error;
      return state.untagPerson(auth.user.id, String(params.id), String(params.personId))
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Photo person tag not found");
    }),
  ];
}
