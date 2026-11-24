import { http, HttpResponse } from "msw";
import type {
  AddMediaCollectionItemRequest,
  CreateMediaCollectionRequest,
  CreateShareRequest,
  GlobalSearchHit,
  MediaCollectionDetail,
  MediaCollectionEntry,
  MediaCollectionSummary,
  PublicShareItem,
  RelatedWork,
  ShareLinkSummary,
  UpdateMediaCollectionRequest,
} from "@/api/queries/shares-collections";
import type { ApiErrorCode } from "@/api/types";
import type { MockDb } from "../db";
import { stableUuid } from "../fixtures";
import type { HandlerOptions } from "../handlers";

const BASE = "/api/v1";

interface StoredShare extends ShareLinkSummary {
  password: string | null;
  items: PublicShareItem[];
}

interface StoredCollection {
  summary: MediaCollectionSummary;
  items: MediaCollectionEntry[];
}

function fail(status: number, code: ApiErrorCode, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

function bearer(request: Request): string | null {
  const header = request.headers.get("Authorization");
  const match = header ? /^Bearer\s+(.+)$/i.exec(header) : null;
  return match?.[1] ?? null;
}

async function readJson<T>(request: Request): Promise<Partial<T> | null> {
  try {
    const body: unknown = await request.json();
    return typeof body === "object" && body !== null ? body : null;
  } catch {
    return null;
  }
}

export function createSharesCollectionsSearchHandlers(db: MockDb, _options: HandlerOptions = {}) {
  const shares = new Map<string, StoredShare>();
  const collections = new Map<string, StoredCollection>();

  const authUser = (request: Request) => db.userForToken(bearer(request));

  const sampleRelatedFor = (title: string, excludeKind?: string | null): RelatedWork[] => {
    const norm = title.trim().toLowerCase();
    if (!norm) return [];
    const musicLib = db.libraries.find((l) => l.kind === "music")?.id ?? stableUuid("lib:music");
    const bookLib = db.libraries.find((l) => l.kind === "book")?.id ?? stableUuid("lib:book");
    const videoLib = db.libraries.find((l) => l.kind === "video")?.id ?? stableUuid("lib:video");
    const baseName = title.replace(/\s*(原声带|OST|（.*）|\(.*\))$/i, "").trim() || title;
    const candidates: RelatedWork[] = [
      {
        entryKind: "video",
        targetId: "work_suzume",
        libraryId: videoLib,
        title: baseName,
        subtitle: "2022",
        artworkId: null,
      },
      {
        entryKind: "music_album",
        targetId: `${ "a".repeat(64) }.${ "b".repeat(64) }`,
        libraryId: musicLib,
        title: `${baseName} 原声带`,
        subtitle: "RADWIMPS",
        artworkId: null,
      },
      {
        entryKind: "book",
        targetId: stableUuid(`book:${baseName}`),
        libraryId: bookLib,
        title: `${baseName}（官方小说）`,
        subtitle: "原著文库",
        artworkId: null,
      },
    ];
    return candidates.filter((c) => c.entryKind !== excludeKind);
  };

  return [
    /* ---------------------------------------------------------- shares */
    http.get(`${BASE}/shares`, ({ request }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const list = [...shares.values()]
        .filter((s) => user.role === "admin" || s.creatorId === user.id)
        .map(({ password: _pw, items: _it, ...rest }) => rest);
      return HttpResponse.json({ shares: list });
    }),

    http.post(`${BASE}/shares`, async ({ request }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const body = await readJson<CreateShareRequest>(request);
      if (!body?.targetKind || !body.targetId) {
        return fail(400, "validation_failed", "targetKind and targetId are required");
      }
      const id = crypto.randomUUID();
      const token = id.replace(/-/g, "").slice(0, 24);
      const stripGps = body.stripGps ?? true;
      const allowDownload = body.allowDownload ?? false;
      const expiresAt = body.expiresInHours
        ? db.iso(body.expiresInHours * 3_600_000)
        : (body.expiresAt ?? null);
      const summary: ShareLinkSummary = {
        id,
        token,
        sharePath: `/s/${token}`,
        creatorId: user.id,
        targetKind: body.targetKind,
        targetId: body.targetId,
        libraryId: body.libraryId ?? null,
        title: body.title?.trim() || "Shared Media",
        hasPassword: Boolean(body.password),
        allowDownload,
        stripGps,
        expiresAt,
        maxViews: body.maxViews ?? null,
        viewCount: 0,
        revokedAt: null,
        expired: false,
        createdAt: db.iso(),
      };
      const items: PublicShareItem[] =
        body.targetKind === "photo_album"
          ? [
              {
                itemId: stableUuid(`share-photo:${id}`),
                mediaKind: "photo",
                title: "巡礼风景 01",
                camera: "SONY ILCE-7C",
                latitude: stripGps ? null : 33.2794,
                longitude: stripGps ? null : 131.5006,
                hasCover: true,
              },
            ]
          : [
              {
                itemId: stableUuid(`share-video:${id}`),
                mediaKind: "video",
                title: summary.title,
                subtitle: "2024",
                durationMs: 7_200_000,
                hasCover: true,
              },
            ];
      shares.set(id, {
        ...summary,
        password: body.password || null,
        items,
      });
      return HttpResponse.json(summary, { status: 201 });
    }),

    http.delete(`${BASE}/shares/:id`, ({ request, params }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const entry = shares.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Share link not found");
      if (user.role !== "admin" && entry.creatorId !== user.id) {
        return fail(403, "forbidden", "Forbidden");
      }
      entry.revokedAt = db.iso();
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/public/shares/:token`, ({ request, params }) => {
      const token = String(params.token);
      const entry = [...shares.values()].find((s) => s.token === token);
      if (!entry) return fail(404, "not_found", "Share link not found");
      if (entry.revokedAt || entry.expired) {
        return fail(410, "unavailable", "Share link expired or revoked");
      }
      const proof = new URL(request.url).searchParams.get("proof");
      const expectedProof = `proof_${entry.token}`;
      if (entry.password && proof !== expectedProof) {
        if (entry.maxViews != null && entry.viewCount >= entry.maxViews) {
          return fail(410, "unavailable", "Max views reached");
        }
        return HttpResponse.json({
          token: entry.token,
          title: entry.title,
          targetKind: entry.targetKind,
          locked: true,
          allowDownload: false,
          stripGps: entry.stripGps,
          expiresAt: entry.expiresAt,
          items: [],
        });
      }
      if (proof !== expectedProof) {
        if (entry.maxViews != null && entry.viewCount >= entry.maxViews) {
          return fail(410, "unavailable", "Max views reached");
        }
        entry.viewCount += 1;
      }
      return HttpResponse.json({
        token: entry.token,
        title: entry.title,
        targetKind: entry.targetKind,
        locked: false,
        allowDownload: entry.allowDownload,
        stripGps: entry.stripGps,
        expiresAt: entry.expiresAt,
        proof: expectedProof,
        items: entry.items,
      });
    }),

    http.post(`${BASE}/public/shares/:token/unlock`, async ({ request, params }) => {
      const token = String(params.token);
      const entry = [...shares.values()].find((s) => s.token === token);
      if (!entry) return fail(404, "not_found", "Share link not found");
      if (entry.revokedAt || entry.expired || (entry.maxViews != null && entry.viewCount >= entry.maxViews)) {
        return fail(410, "unavailable", "Share link expired or revoked");
      }
      const body = await readJson<{ password: string }>(request);
      if (entry.password && body?.password !== entry.password) {
        return fail(401, "invalid_credentials", "Invalid share password");
      }
      entry.viewCount += 1;
      return HttpResponse.json({
        proof: `proof_${entry.token}`,
        expiresAt: db.iso(2 * 3_600_000),
      });
    }),

    http.get(`${BASE}/public/shares/:token/media/:itemId`, ({ request, params }) => {
      const token = String(params.token);
      const entry = [...shares.values()].find((s) => s.token === token);
      if (!entry || entry.revokedAt) return fail(404, "not_found", "Not found");
      const proof = new URL(request.url).searchParams.get("proof");
      if (proof !== `proof_${entry.token}`) return fail(401, "unauthorized", "Missing proof");
      return new HttpResponse(new Uint8Array([0xff, 0xd8, 0xff, 0xd9]), {
        headers: { "Content-Type": "image/jpeg" },
      });
    }),

    http.get(`${BASE}/public/shares/:token/cover/:itemId`, ({ request, params }) => {
      const token = String(params.token);
      const entry = [...shares.values()].find((s) => s.token === token);
      if (!entry || entry.revokedAt) return fail(404, "not_found", "Not found");
      const proof = new URL(request.url).searchParams.get("proof");
      if (proof !== `proof_${entry.token}`) return fail(401, "unauthorized", "Missing proof");
      return new HttpResponse(new Uint8Array([0xff, 0xd8, 0xff, 0xd9]), {
        headers: { "Content-Type": "image/jpeg" },
      });
    }),

    /* ---------------------------------------------------------- collections & related */
    http.get(`${BASE}/collections`, ({ request }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const list = [...collections.values()]
        .filter((c) => c.summary.ownerId === user.id)
        .map((c) => c.summary);
      return HttpResponse.json({ collections: list });
    }),

    http.post(`${BASE}/collections`, async ({ request }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const body = await readJson<CreateMediaCollectionRequest>(request);
      const title = body?.title?.trim();
      if (!title) return fail(400, "validation_failed", "title required");
      const id = crypto.randomUUID();
      const summary: MediaCollectionSummary = {
        id,
        ownerId: user.id,
        title,
        description: body?.description?.trim() || null,
        coverArtworkId: body?.coverArtworkId ?? null,
        previewArtworkIds: [],
        itemCount: 0,
        kinds: [],
        createdAt: db.iso(),
        updatedAt: db.iso(),
      };
      collections.set(id, { summary, items: [] });
      return HttpResponse.json(summary, { status: 201 });
    }),

    http.get(`${BASE}/collections/:id`, ({ request, params }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const entry = collections.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Collection not found");
      const existingSet = new Set(entry.items.map((i) => `${i.entryKind}:${i.targetId}`));
      const autoRelated = sampleRelatedFor(entry.summary.title).filter(
        (r) => !existingSet.has(`${r.entryKind}:${r.targetId}`),
      );
      const detail: MediaCollectionDetail = {
        collection: entry.summary,
        items: entry.items,
        autoRelated,
      };
      return HttpResponse.json(detail);
    }),

    http.patch(`${BASE}/collections/:id`, async ({ request, params }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const entry = collections.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Collection not found");
      const body = await readJson<UpdateMediaCollectionRequest>(request);
      if (body?.title?.trim()) entry.summary.title = body.title.trim();
      if (body?.description !== undefined) entry.summary.description = body.description || null;
      if (Array.isArray(body?.itemOrder)) {
        const order = body.itemOrder;
        entry.items.sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
        entry.items.forEach((it, idx) => {
          it.position = idx;
        });
      }
      entry.summary.updatedAt = db.iso();
      return HttpResponse.json(entry.summary);
    }),

    http.delete(`${BASE}/collections/:id`, ({ request, params }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      if (!collections.delete(String(params.id))) return fail(404, "not_found", "Not found");
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/collections/:id/items`, async ({ request, params }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const entry = collections.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Collection not found");
      const body = await readJson<AddMediaCollectionItemRequest>(request);
      if (!body?.entryKind || !body.targetId) {
        return fail(400, "validation_failed", "entryKind and targetId required");
      }
      const item: MediaCollectionEntry = {
        id: crypto.randomUUID(),
        collectionId: entry.summary.id,
        entryKind: body.entryKind,
        targetId: body.targetId,
        libraryId: body.libraryId ?? null,
        title: body.title ?? body.targetId,
        subtitle: body.subtitle ?? null,
        artworkId: body.artworkId ?? null,
        position: entry.items.length,
        addedAt: db.iso(),
      };
      entry.items.push(item);
      entry.summary.itemCount = entry.items.length;
      if (!entry.summary.kinds.includes(item.entryKind)) {
        entry.summary.kinds.push(item.entryKind);
      }
      return HttpResponse.json(item, { status: 201 });
    }),

    http.delete(`${BASE}/collections/:id/items/:entryId`, ({ request, params }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const entry = collections.get(String(params.id));
      if (!entry) return fail(404, "not_found", "Collection not found");
      const idx = entry.items.findIndex((i) => i.id === String(params.entryId));
      if (idx === -1) return fail(404, "not_found", "Item not found");
      entry.items.splice(idx, 1);
      entry.summary.itemCount = entry.items.length;
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/related`, ({ request }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const url = new URL(request.url);
      const title = url.searchParams.get("title") ?? "";
      const excludeKind = url.searchParams.get("excludeKind");
      return HttpResponse.json({ works: sampleRelatedFor(title, excludeKind) });
    }),

    /* ---------------------------------------------------------- global search */
    http.get(`${BASE}/search`, ({ request }) => {
      const user = authUser(request);
      if (!user) return fail(401, "unauthorized", "Unauthorized");
      const q = (new URL(request.url).searchParams.get("q") ?? "").trim();
      if (!q) return HttpResponse.json({ query: "", hits: [] });

      const videoLib = db.libraries.find((l) => l.kind === "video")?.id ?? stableUuid("lib:video");
      const musicLib = db.libraries.find((l) => l.kind === "music")?.id ?? stableUuid("lib:music");
      const bookLib = db.libraries.find((l) => l.kind === "book")?.id ?? stableUuid("lib:book");

      const sampleHits: GlobalSearchHit[] = [
        {
          kind: "video_work",
          id: "work_suzume",
          libraryId: videoLib,
          title: "铃芽之旅",
          subtitle: "电影 · 2022",
        },
        {
          kind: "music_album",
          id: `${ "a".repeat(64) }.${ "b".repeat(64) }`,
          libraryId: musicLib,
          title: "铃芽之旅 原声带",
          subtitle: "RADWIMPS",
        },
        {
          kind: "book",
          id: stableUuid("book:suzume"),
          libraryId: bookLib,
          title: "铃芽之旅（官方小说）",
          subtitle: "新海诚",
        },
        {
          kind: "home_device",
          id: "dev_living_light",
          sourceId: "living",
          title: "客厅主灯",
          subtitle: "客厅 · light",
        },
      ];
      const lower = q.toLowerCase();
      const hits = sampleHits.filter(
        (h) =>
          h.title.toLowerCase().includes(lower) ||
          (h.subtitle?.toLowerCase().includes(lower) ?? false),
      );
      return HttpResponse.json({ query: q, hits });
    }),
  ];
}
