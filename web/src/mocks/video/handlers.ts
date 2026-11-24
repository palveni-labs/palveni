/**
 * MSW handlers for the media API (Video metadata, home, library wall, detail,
 * media-info, playback sessions, subtitles, artwork).
 *
 * Only active when `db.features` includes `"video"` (so M0 tests stay isolated).
 */
import { delay, http, HttpResponse } from "msw";
import type {
  PlaybackRequest,
  PlaybackSession,
  PutVideoSegmentsRequest,
  VideoSegment,
} from "@/modules/video/api";
import type { MockDb } from "../db";
import type { HandlerOptions } from "../handlers";
import { renderBackdropSvg, renderPersonSvg, renderPosterSvg, renderThumbSvg } from "./artwork";
import sampleVideoUrl from "./sample.mp4?url";
import { VideoMockState } from "./state";

const BASE = "/api/v1";

function fail(status: number, code: string, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

function bearer(request: Request): string | null {
  const header = request.headers.get("Authorization");
  const match = header ? /^Bearer\s+(.+)$/i.exec(header) : null;
  return match?.[1] ?? null;
}

function svgResponse(svg: string) {
  return new HttpResponse(svg, {
    headers: {
      "Content-Type": "image/svg+xml; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}

async function readJson<T>(request: Request): Promise<Partial<T>> {
  try {
    const parsed: unknown = await request.json();
    return typeof parsed === "object" && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

export function createVideoHandlers(db: MockDb, options: HandlerOptions = {}) {
  const state = new VideoMockState(db.now);
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(Math.round(latency * 0.6)) : Promise.resolve());

  const guard = (request: Request, feature = "video") => {
    if (!db.userForToken(bearer(request))) return fail(401, "unauthorized", "Missing or invalid access token");
    if (!db.features.includes(feature)) return fail(404, "not_found", "Feature not enabled");
    return null;
  };

  let sessionSeq = 1;
  let segmentSeq = 1;
  const segmentsByItem = new Map<string, VideoSegment[]>();

  return [
    /* ------------------------------------------------------------ home */
    http.get(`${BASE}/videos/home`, async ({ request }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const libraryId = new URL(request.url).searchParams.get("libraryId");
      return HttpResponse.json(state.home(libraryId));
    }),

    /* ------------------------------------------------------------ library list & genres */
    http.get(`${BASE}/libraries/:id/videos`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const libraryId = String(params.id);
      if (!db.getLibrary(libraryId)) return fail(404, "not_found", "Library not found");
      const q = new URL(request.url).searchParams;
      return HttpResponse.json(
        state.list(libraryId, {
          kind: q.get("kind"),
          series: q.get("series"),
          season: q.get("season"),
          q: q.get("q"),
          genre: q.get("genre"),
          person: q.get("person"),
          yearFrom: q.get("yearFrom"),
          yearTo: q.get("yearTo"),
          played: q.get("played"),
          favorite: q.get("favorite"),
          order: q.get("order"),
          dir: q.get("dir"),
          seed: q.get("seed"),
          startsWith: q.get("startsWith"),
          cursor: q.get("cursor"),
          limit: q.get("limit"),
        }),
      );
    }),

    http.get(`${BASE}/libraries/:id/videos/genres`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const libraryId = String(params.id);
      if (!db.getLibrary(libraryId)) return fail(404, "not_found", "Library not found");
      return HttpResponse.json({ genres: state.genres(libraryId) });
    }),

    /* ------------------------------------------------------------ work detail & state */
    http.get(`${BASE}/libraries/:id/videos/works/:workId`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const detail = state.workDetail(String(params.id), String(params.workId));
      return detail ? HttpResponse.json(detail) : fail(404, "not_found", "Work not found");
    }),

    http.put(`${BASE}/libraries/:id/videos/works/:workId/played`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return state.setWorkPlayed(String(params.id), String(params.workId), true)
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Work not found");
    }),

    http.delete(`${BASE}/libraries/:id/videos/works/:workId/played`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return state.setWorkPlayed(String(params.id), String(params.workId), false)
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Work not found");
    }),

    http.put(`${BASE}/libraries/:id/videos/works/:workId/favorite`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return state.setWorkFavorite(String(params.id), String(params.workId), true)
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Work not found");
    }),

    http.delete(`${BASE}/libraries/:id/videos/works/:workId/favorite`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return state.setWorkFavorite(String(params.id), String(params.workId), false)
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Work not found");
    }),

    /* ------------------------------------------------------------ item personal state */
    http.put(`${BASE}/me/items/:id/played`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      state.setItemPlayed(String(params.id), true);
      return new HttpResponse(null, { status: 204 });
    }),

    http.delete(`${BASE}/me/items/:id/played`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      state.setItemPlayed(String(params.id), false);
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/me/items/:id`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return HttpResponse.json(state.getPersonalMediaState(String(params.id)));
    }),

    http.put(`${BASE}/me/items/:id/progress`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const body = await readJson<{ positionMs: number; completed: boolean; expectedVersion: number }>(request);
      const updated = state.reportProgress(String(params.id), {
        positionMs: Number(body.positionMs) || 0,
        completed: Boolean(body.completed),
        expectedVersion: Number(body.expectedVersion) || 0,
      });
      return HttpResponse.json(updated);
    }),

    /* ------------------------------------------------------------ media-info, playback & subtitles */
    http.get(`${BASE}/items/:id/media-info`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return HttpResponse.json(state.mediaInfo(String(params.id)));
    }),

    http.post(`${BASE}/items/:id/video-playback`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const itemId = String(params.id);
      const body = await readJson<PlaybackRequest>(request);
      const sessionId = `vs-${sessionSeq++}`;
      const startMs = Math.max(0, Math.floor(Number(body.startMs) || 0));
      state.sessions.set(sessionId, { itemId, startMs, createdAt: Date.now() });

      const capped = typeof body.maxHeight === "number" && body.maxHeight > 0;
      const mode: PlaybackSession["mode"] =
        body.mode === "transcode" || capped ? "transcode" : "direct";
      const sep = sampleVideoUrl.includes("?") ? "&" : "?";
      const session: PlaybackSession = {
        sessionId,
        mode,
        protocol: "file",
        url: `${sampleVideoUrl}${sep}ticket=mock-${sessionId}`,
        expiresAt: new Date(Date.now() + 3_600_000).toISOString(),
        startMs: 0,
        reason: capped ? `maxHeight ${body.maxHeight}p` : body.mode === "transcode" ? "requested transcode" : null,
      };
      return HttpResponse.json(session);
    }),

    http.delete(`${BASE}/video-sessions/:sessionId`, ({ request, params }) => {
      const denied = guard(request);
      if (denied) return denied;
      state.sessions.delete(String(params.sessionId));
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/items/:id/subtitles`, async ({ request }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      return HttpResponse.json({ tracks: state.subtitles(), truncated: false });
    }),

    http.get(`${BASE}/items/:id/subtitles/:track`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const content = state.subtitleContent(String(params.track));
      if (!content) return fail(404, "not_found", "Subtitle track not found");
      return new HttpResponse(content, {
        headers: { "Content-Type": "text/vtt; charset=utf-8", "Cache-Control": "public, max-age=3600" },
      });
    }),

    /* ------------------------------------------------------------ artwork */
    http.get(`${BASE}/items/:id/poster`, ({ request, params }) => {
      const denied = guard(request);
      if (denied) return denied;
      const entry = state.artByItemId.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Poster not found");
      const level = new URL(request.url).searchParams.get("level");
      return svgResponse(renderPosterSvg(entry.art, level === "season" ? entry.seasonNumber : undefined));
    }),

    http.get(`${BASE}/items/:id/backdrop`, ({ request, params }) => {
      const denied = guard(request);
      if (denied) return denied;
      const entry = state.artByItemId.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Backdrop not found");
      return svgResponse(renderBackdropSvg(entry.art));
    }),

    http.get(`${BASE}/items/:id/thumb`, ({ request, params }) => {
      const denied = guard(request);
      if (denied) return denied;
      const entry = state.artByItemId.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Thumb not found");
      return svgResponse(renderThumbSvg(entry.art, entry.thumbVariant ?? 1));
    }),

    http.get(`${BASE}/items/:id/segments`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const itemId = String(params.id);
      return HttpResponse.json({ segments: segmentsByItem.get(itemId) ?? [] });
    }),

    http.put(`${BASE}/items/:id/segments`, async ({ request, params }) => {
      await wait();
      const denied = guard(request);
      if (denied) return denied;
      const itemId = String(params.id);
      const body = await readJson<PutVideoSegmentsRequest>(request);
      const raw = Array.isArray(body.segments) ? body.segments : [];
      const saved: VideoSegment[] = raw
        .filter((s) => typeof s.startMs === "number" && typeof s.endMs === "number" && s.endMs > s.startMs)
        .map((s) => ({
          id: `seg-${segmentSeq++}`,
          itemId,
          kind: s.kind,
          startMs: s.startMs,
          endMs: s.endMs,
          source: s.source ?? "manual",
          updatedAt: new Date(db.now()).toISOString(),
        }))
        .sort((a, b) => a.startMs - b.startMs);
      segmentsByItem.set(itemId, saved);
      return HttpResponse.json({ segments: saved });
    }),

    http.get(`${BASE}/items/:id/trickplay`, ({ request, params }) => {
      const denied = guard(request);
      if (denied) return denied;
      const entry = state.artByItemId.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Trickplay frame unavailable");
      const atMs = Number(new URL(request.url).searchParams.get("atMs") ?? "0") || 0;
      return svgResponse(renderThumbSvg(entry.art, (Math.floor(atMs / 10_000) % 3) + 1));
    }),

    http.get(`${BASE}/libraries/:id/videos/people/:key/image`, ({ request, params }) => {
      const denied = guard(request);
      if (denied) return denied;
      const person = state.peopleByKey.get(String(params.key));
      if (!person) return fail(404, "not_found", "Person image not found");
      return svgResponse(renderPersonSvg(person));
    }),
  ];
}
