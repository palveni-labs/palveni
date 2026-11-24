import { delay, http, HttpResponse } from "msw";
import type {
  MetadataConfigResponse,
  MetadataOverride,
  MetadataSearchCandidate,
  MetadataSearchRequest,
  SaveMetadataOverrideRequest,
  ServerLogEntry,
  ServerLogLevel,
  SystemDoctorReport,
  UpdateMetadataConfigRequest,
} from "@/api/types";
import type { MockDb } from "./db";
import type { HandlerOptions } from "./handlers";

const BASE = "/api/v1";

function bearer(request: Request): string | null {
  const header = request.headers.get("Authorization");
  const match = header ? /^Bearer\s+(.+)$/i.exec(header) : null;
  return match?.[1] ?? null;
}

function fail(status: number, code: string, message: string, field?: string) {
  return HttpResponse.json(
    { error: field ? { code, message, details: { field } } : { code, message } },
    { status },
  );
}

export function createWebdavMetadataObservabilityHandlers(
  db: MockDb,
  options: HandlerOptions = {},
) {
  const latency = options.latency ?? 0;
  const wait = () =>
    latency > 0 ? delay(latency + Math.round(Math.random() * latency)) : Promise.resolve();

  let tmdbKey = "";
  let bangumiToken = "";
  let preferredLanguage = "zh-CN";
  let autoFetchPosters = true;

  const overrides = new Map<string, MetadataOverride>();
  const runtimeLogs: ServerLogEntry[] = [
    {
      id: "rt-1",
      timestamp: db.iso(),
      level: "info",
      source: "runtime",
      target: "system.observability",
      message: "Server observability log ring buffer initialized",
    },
    {
      id: "rt-2",
      timestamp: db.iso(-180_000),
      level: "warn",
      source: "webdav.auth",
      target: "webdav.auth",
      message: "Rejected unauthenticated WebDAV Basic Auth request",
    },
    {
      id: "rt-3",
      timestamp: db.iso(-420_000),
      level: "error",
      source: "job",
      target: "video.thumbnail",
      message: "job video.thumbnail [failed] (Demo): ffprobe timed out on sample stream",
    },
  ];

  const pushLog = (level: ServerLogLevel, target: string, message: string) => {
    runtimeLogs.unshift({
      id: `rt-${runtimeLogs.length + 1}`,
      timestamp: db.iso(),
      level,
      source: "runtime",
      target,
      message,
    });
  };

  const requireAuth = (request: Request, adminOnly = false) => {
    const token = bearer(request);
    const user = db.userForToken(token);
    if (!user || !token) return { response: fail(401, "unauthorized", "Authentication required") };
    if (adminOnly && user.role !== "admin") {
      return { response: fail(403, "forbidden", "Administrator privileges required") };
    }
    return { user };
  };

  const publicConfig = (): MetadataConfigResponse => ({
    tmdbConfigured: tmdbKey.length > 0,
    bangumiConfigured: bangumiToken.length > 0,
    preferredLanguage,
    autoFetchPosters,
  });

  return [
    http.get(`${BASE}/metadata/config`, async ({ request }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      return HttpResponse.json(publicConfig());
    }),

    http.put(`${BASE}/metadata/config`, async ({ request }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      const body = (await request.json().catch(() => ({}))) as UpdateMetadataConfigRequest;
      if (body.clearTmdbKey) {
        tmdbKey = "";
      } else if (typeof body.tmdbApiKey === "string" && body.tmdbApiKey.trim()) {
        tmdbKey = body.tmdbApiKey.trim();
      }
      if (body.clearBangumiToken) {
        bangumiToken = "";
      } else if (typeof body.bangumiToken === "string" && body.bangumiToken.trim()) {
        bangumiToken = body.bangumiToken.trim();
      }
      if (typeof body.preferredLanguage === "string" && body.preferredLanguage.trim()) {
        preferredLanguage = body.preferredLanguage.trim();
      }
      if (typeof body.autoFetchPosters === "boolean") {
        autoFetchPosters = body.autoFetchPosters;
      }
      pushLog("info", "metadata.config", `Metadata config updated (lang=${preferredLanguage})`);
      return HttpResponse.json(publicConfig());
    }),

    http.post(`${BASE}/metadata/search`, async ({ request }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      const body = (await request.json().catch(() => ({}))) as Partial<MetadataSearchRequest>;
      const provider = body.provider ?? "bangumi";
      const query = (body.query ?? "").trim();
      if (!query) {
        return fail(400, "validation_failed", "Search query is required", "query");
      }
      if (provider === "tmdb" && !tmdbKey) {
        return fail(
          422,
          "validation_failed",
          "TMDB API Key is not configured. Save a TMDB API Key in System Maintenance before searching TMDB.",
          "provider",
        );
      }

      const candidates: MetadataSearchCandidate[] =
        provider === "openlibrary"
          ? [
              {
                provider: "openlibrary",
                externalId: "/works/OL82563W",
                title: `${query}（典藏版）`,
                originalTitle: query,
                year: body.year ?? 2008,
                overview: "来自 OpenLibrary 的示例书目简介，涵盖作者、主题与首版信息。",
                rating: 8.8,
                genres: ["科幻", "文学"],
                posterUrl: "https://covers.openlibrary.org/b/id/8231856-L.jpg",
                backdropUrl: null,
                mediaType: "book",
              },
            ]
          : provider === "bangumi"
            ? [
                {
                  provider: "bangumi",
                  externalId: "253",
                  title: `${query} · 番组计划条目`,
                  originalTitle: `${query} Original`,
                  year: body.year ?? 2023,
                  overview: "来自 Bangumi 的标准化条目简介与评分信息。",
                  rating: 8.6,
                  genres: ["动画", "科幻"],
                  posterUrl: "https://lain.bgm.tv/pic/cover/l/253.jpg",
                  backdropUrl: null,
                  mediaType: body.mediaType ?? "anime",
                },
              ]
            : [
                {
                  provider: "tmdb",
                  externalId: "438631",
                  title: `${query} · TMDB 匹配`,
                  originalTitle: `${query} (Original)`,
                  year: body.year ?? 2021,
                  overview: "来自 TMDB 的标准化影视简介、评分与海报元数据。",
                  rating: 8.4,
                  genres: ["科幻", "冒险"],
                  posterUrl: "https://image.tmdb.org/t/p/w500/dune.jpg",
                  backdropUrl: "https://image.tmdb.org/t/p/w1280/dune-bg.jpg",
                  mediaType: body.mediaType ?? "movie",
                },
              ];

      return HttpResponse.json({ provider, query, candidates });
    }),

    http.get(`${BASE}/items/:id/metadata-override`, async ({ request, params }) => {
      await wait();
      const auth = requireAuth(request, false);
      if ("response" in auth) return auth.response;
      const itemId = String(params.id);
      return HttpResponse.json({
        itemId,
        override: overrides.get(itemId) ?? null,
      });
    }),

    http.put(`${BASE}/items/:id/metadata-override`, async ({ request, params }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      const itemId = String(params.id);
      const body = (await request.json().catch(() => ({}))) as SaveMetadataOverrideRequest;
      for (const [field, url] of [
        ["posterUrl", body.posterUrl],
        ["backdropUrl", body.backdropUrl],
      ] as const) {
        if (url && url.trim() && !url.trim().startsWith("https://")) {
          return fail(400, "validation_failed", `${field} must use https://`, field);
        }
      }
      const saved: MetadataOverride = {
        itemId,
        workKey: body.workKey ?? null,
        provider: body.provider ?? "manual",
        externalId: body.externalId ?? null,
        title: body.title?.trim() || null,
        originalTitle: body.originalTitle?.trim() || null,
        year: body.year ?? null,
        overview: body.overview?.trim() || null,
        rating: body.rating ?? null,
        genres: body.genres ?? [],
        posterUrl: body.posterUrl?.trim() || null,
        backdropUrl: body.backdropUrl?.trim() || null,
        updatedAt: db.iso(),
      };
      overrides.set(itemId, saved);
      pushLog(
        "info",
        "metadata.override",
        `Saved metadata override for item ${itemId} (title=${saved.title ?? ""})`,
      );
      return HttpResponse.json(saved);
    }),

    http.delete(`${BASE}/items/:id/metadata-override`, async ({ request, params }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      const itemId = String(params.id);
      if (!overrides.delete(itemId)) {
        return fail(404, "not_found", "Metadata override not found");
      }
      pushLog("info", "metadata.override", `Removed metadata override for item ${itemId}`);
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/system/prometheus`, async ({ request }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      const text = [
        "# HELP palveni_uptime_seconds Process uptime in seconds.",
        "# TYPE palveni_uptime_seconds gauge",
        "palveni_uptime_seconds 3600.000",
        "# HELP palveni_libraries_total Total configured non-deleted media libraries.",
        "# TYPE palveni_libraries_total gauge",
        `palveni_libraries_total ${db.libraries.length}`,
        "# HELP palveni_items_total Total live indexed leaf media items.",
        "# TYPE palveni_items_total gauge",
        `palveni_items_total ${db.allLeafItems().length}`,
        "# HELP palveni_users_total Total user accounts.",
        "# TYPE palveni_users_total gauge",
        `palveni_users_total ${db.users.length}`,
        "# HELP palveni_active_sessions Active client sessions.",
        "# TYPE palveni_active_sessions gauge",
        "palveni_active_sessions 1",
        "# HELP palveni_cpu_load_ratio Normalized 1-minute load average.",
        "# TYPE palveni_cpu_load_ratio gauge",
        "palveni_cpu_load_ratio 0.2400",
        "# HELP palveni_memory_used_bytes Memory used in bytes.",
        "# TYPE palveni_memory_used_bytes gauge",
        "palveni_memory_used_bytes 2147483648",
        "# HELP palveni_memory_total_bytes Total memory in bytes.",
        "# TYPE palveni_memory_total_bytes gauge",
        "palveni_memory_total_bytes 17179869184",
        "# HELP palveni_mounts_total Total tracked mounts.",
        "# TYPE palveni_mounts_total gauge",
        "palveni_mounts_total 1",
      ].join("\n");
      return new HttpResponse(text, {
        status: 200,
        headers: { "Content-Type": "text/plain; version=0.0.4; charset=utf-8" },
      });
    }),

    http.get(`${BASE}/system/doctor`, async ({ request }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      const report: SystemDoctorReport = {
        overallStatus: "healthy",
        generatedAt: db.iso(),
        checks: [
          {
            id: "database",
            name: "Database",
            status: "healthy",
            summary: "sqlite connected (revision #42)",
            detail: "Backend: sqlite · Current catalog revision: 42",
          },
          {
            id: "ffmpeg",
            name: "FFmpeg & Transcoding",
            status: "healthy",
            summary: "FFmpeg ready (2 hardware encoders verified: h264_videotoolbox, hevc_videotoolbox)",
            detail: "ffmpeg: /usr/bin/ffmpeg (7.1) · ffprobe: /usr/bin/ffprobe",
          },
          {
            id: "storage",
            name: "Storage & Mounts",
            status: "healthy",
            summary: "1 volume(s) healthy · 1 network mount(s) online",
            detail: "/srv/media (412.5 GiB free, 42% used)",
          },
          {
            id: "libraries",
            name: "Libraries & Index",
            status: "healthy",
            summary: `${db.libraries.length} libraries indexed (${db.allLeafItems().length} items)`,
            detail: `Libraries: ${db.libraries.length} · Live leaf items: ${db.allLeafItems().length}`,
          },
          {
            id: "protocols",
            name: "Protocol Surfaces",
            status: "healthy",
            summary: "5 protocol surfaces active",
            detail: "Active endpoints: Native API (/api/v1), WebDAV (/webdav/), Subsonic (/rest), OPDS (/opds), MCP (/api/v1/mcp)",
          },
        ],
      };
      return HttpResponse.json(report);
    }),

    http.get(`${BASE}/system/logs`, async ({ request }) => {
      await wait();
      const auth = requireAuth(request, true);
      if ("response" in auth) return auth.response;
      const url = new URL(request.url);
      const level = url.searchParams.get("level");
      const limit = Number(url.searchParams.get("limit") ?? "100");
      const auditMapped: ServerLogEntry[] = db.auditLogs.slice(0, 20).map((entry) => ({
        id: `audit-${entry.id}`,
        timestamp: entry.occurredAt,
        level: entry.action === "auth.login_failed" ? "warn" : "info",
        source: "audit",
        target: `audit/${entry.action}`,
        message: `[audit] ${entry.action}${entry.target ? ` target=${entry.target}` : ""}`,
      }));
      let entries = [...runtimeLogs, ...auditMapped];
      if (level && level !== "all") {
        entries = entries.filter((e) => e.level === level);
      }
      entries.sort((a, b) => b.timestamp.localeCompare(a.timestamp));
      const total = entries.length;
      return HttpResponse.json({
        entries: entries.slice(0, Number.isFinite(limit) && limit > 0 ? limit : 100),
        total,
      });
    }),
  ];
}
