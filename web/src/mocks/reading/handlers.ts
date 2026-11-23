/**
 * MSW handlers for the book and comic reader.
 * Active only when `db.features` includes `"reading"`.
 */
import { delay, http, HttpResponse } from "msw";
import type { CreateBookmarkRequest, ReadingProgressUpdate } from "@/modules/reading/api";
import type { MockDb } from "../db";
import type { HandlerOptions } from "../handlers";
import { ReadingMockState } from "./state";

const BASE = "/api/v1";

function fail(status: number, code: string, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

function bearer(request: Request): string | null {
  const header = request.headers.get("Authorization");
  const match = header ? /^Bearer\s+(.+)$/i.exec(header) : null;
  return match?.[1] ?? null;
}

function svgCover(title: string, subtitle: string) {
  const safeTitle = title.replace(/[<>&"]/g, "");
  const safeSub = subtitle.replace(/[<>&"]/g, "");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 360 540"><rect width="360" height="540" fill="#1f2428"/><rect x="24" y="24" width="312" height="492" rx="8" fill="none" stroke="#d97706" stroke-opacity="0.45" stroke-width="2"/><text x="42" y="140" fill="#f5f5f4" font-size="28" font-weight="700" font-family="sans-serif">${safeTitle}</text><text x="42" y="185" fill="#a8a29e" font-size="18" font-family="sans-serif">${safeSub}</text></svg>`;
  return new HttpResponse(svg, {
    headers: {
      "Content-Type": "image/svg+xml; charset=utf-8",
      "Cache-Control": "private, no-cache",
    },
  });
}

export function createReadingHandlers(db: MockDb, options: HandlerOptions = {}) {
  const state = new ReadingMockState(db.now);
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(Math.round(latency * 0.5)) : Promise.resolve());

  const guard = (request: Request) => {
    if (!db.userForToken(bearer(request))) {
      return fail(401, "unauthorized", "Missing or invalid access token");
    }
    if (!db.features.includes("reading")) {
      return fail(404, "not_found", "Feature not enabled");
    }
    return null;
  };

  return [
    /* Favorite fallthrough handler for reading items (passes through to music/video if not a reading item) */
    http.put(`${BASE}/me/items/:id/favorite`, async ({ request, params }) => {
      if (!db.features.includes("reading")) return undefined;
      const itemId = String(params.id);
      if (!state.hasItem(itemId)) return undefined;
      await wait();
      if (!db.userForToken(bearer(request))) {
        return fail(401, "unauthorized", "Missing or invalid access token");
      }
      const body = (await request.json().catch(() => null)) as { favorite?: boolean } | null;
      const fav = Boolean(body?.favorite);
      state.setFavorite(itemId, fav);
      const rec = state.getRecord(itemId)!;
      return HttpResponse.json({
        favorite: fav,
        stars: 0,
        positionMs: rec.entry.state.page,
        completed: rec.entry.state.completed,
        playCount: rec.entry.state.completed ? 1 : 0,
        lastPlayedAt: rec.entry.state.lastReadAt,
        playbackVersion: 1,
      });
    }),

    http.get(`${BASE}/reading/home`, async ({ request }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const libraryId = new URL(request.url).searchParams.get("libraryId");
      if (libraryId && !db.getLibrary(libraryId)) {
        return fail(404, "not_found", "Reading library not found");
      }
      return HttpResponse.json(state.home(libraryId));
    }),

    http.get(`${BASE}/libraries/:id/reading`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const libraryId = String(params.id);
      const lib = db.getLibrary(libraryId);
      if (!lib || (lib.kind !== "book" && lib.kind !== "comic")) {
        return fail(404, "not_found", "Reading library not found");
      }
      const q = new URL(request.url).searchParams;
      return HttpResponse.json(
        state.list(libraryId, {
          kind: q.get("kind"),
          series: q.get("series"),
          author: q.get("author"),
          format: q.get("format"),
          favorite: q.get("favorite"),
          completed: q.get("completed"),
          q: q.get("q"),
          sort: q.get("sort"),
          dir: q.get("dir"),
          cursor: q.get("cursor"),
          limit: q.get("limit"),
        }),
      );
    }),

    http.get(`${BASE}/items/:id/reading-manifest`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const manifest = state.manifest(String(params.id));
      return manifest ? HttpResponse.json(manifest) : fail(404, "not_found", "Reading item not found");
    }),

    http.get(`${BASE}/items/:id/reading-cover`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const rec = state.getRecord(String(params.id));
      if (!rec || !rec.entry.hasCover) return fail(404, "not_found", "Cover unavailable");
      return svgCover(rec.entry.title, rec.entry.authors[0] ?? rec.entry.format.toUpperCase());
    }),

    http.get(`${BASE}/items/:id/reading-page/:page`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const rec = state.getRecord(String(params.id));
      if (!rec) return fail(404, "not_found", "Comic not found");
      if (["cbr", "rar", "7z", "cb7"].includes(rec.entry.format)) {
        return fail(
          415,
          "unsupported_media_type",
          `Format ${rec.entry.format} is not supported for online page streaming`,
        );
      }
      const pageNum = Number(params.page);
      const total = rec.entry.pageCount ?? 0;
      if (!Number.isInteger(pageNum) || pageNum < 0 || pageNum >= total) {
        return fail(404, "not_found", "Page out of bounds");
      }
      return svgCover(rec.entry.title, `Page ${pageNum + 1} / ${total}`);
    }),

    http.get(`${BASE}/items/:id/reading-chapter/:index`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const ch = state.chapter(String(params.id), Number(params.index));
      return ch ? HttpResponse.json(ch) : fail(404, "not_found", "Chapter not found");
    }),

    http.get(`${BASE}/items/:id/reading-asset`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const rec = state.getRecord(String(params.id));
      const assetPath = new URL(request.url).searchParams.get("path");
      if (!rec || !assetPath || assetPath.includes("..")) {
        return fail(404, "not_found", "Asset not found");
      }
      return svgCover(rec.entry.title, assetPath);
    }),

    http.post(`${BASE}/items/:id/reading-inline`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const rec = state.getRecord(String(params.id));
      if (!rec) return fail(404, "not_found", "Reading item not found");
      if (rec.entry.format !== "pdf") {
        return fail(415, "unsupported_media_type", "Inline streaming is only supported for PDF");
      }
      return HttpResponse.json({
        url: `/api/v1/items/${rec.entry.itemId}/reading-inline?ticket=mock-pdf-ticket`,
        expiresAt: "2099-01-01T00:00:00Z",
      });
    }),

    http.get(`${BASE}/items/:id/reading-inline`, ({ request, params }) => {
      const ticket = new URL(request.url).searchParams.get("ticket");
      if (!ticket && !db.userForToken(bearer(request))) {
        return fail(401, "unauthorized", "Authentication required");
      }
      const rec = state.getRecord(String(params.id));
      if (!rec || rec.entry.format !== "pdf") return fail(404, "not_found", "PDF not found");
      return new HttpResponse("%PDF-1.7 mock inline pdf", {
        headers: {
          "Content-Type": "application/pdf",
          "Content-Disposition": "inline",
        },
      });
    }),

    http.post(`${BASE}/items/:id/book-download`, ({ request, params }) => {
      if (!db.userForToken(bearer(request))) {
        return fail(401, "unauthorized", "Authentication required");
      }
      const rec = state.getRecord(String(params.id));
      if (!rec) return undefined;
      return HttpResponse.json({
        url: `/api/v1/items/${rec.entry.itemId}/book-download?ticket=mock-dl-ticket`,
        expiresAt: "2099-01-01T00:00:00Z",
      });
    }),

    http.put(`${BASE}/me/items/:id/reading-progress`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const body = (await request.json()) as ReadingProgressUpdate;
      const updated = state.saveProgress(String(params.id), body);
      return updated
        ? HttpResponse.json(updated)
        : fail(404, "not_found", "Reading item not found");
    }),

    http.post(`${BASE}/me/items/:id/bookmarks`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const body = (await request.json()) as CreateBookmarkRequest;
      const created = state.createBookmark(String(params.id), body);
      return created
        ? HttpResponse.json(created, { status: 201 })
        : fail(404, "not_found", "Reading item not found");
    }),

    http.delete(`${BASE}/me/bookmarks/:id`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return state.deleteBookmark(String(params.id))
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Bookmark not found");
    }),
  ];
}
