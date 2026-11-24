import { delay, http, HttpResponse } from "msw";
import type { components } from "@/api/generated";
import type { ApiErrorCode, User } from "@/api/types";
import type { MockDb } from "./db";
import type { HandlerOptions } from "./handlers";

type Schemas = components["schemas"];

const BASE = "/api/v1";

function bearer(request: Request): string | null {
  const header = request.headers.get("Authorization");
  const match = header ? /^Bearer\s+(.+)$/i.exec(header) : null;
  return match?.[1] ?? null;
}

function fail(status: number, code: ApiErrorCode, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

export function createSyncCastMqttHandlers(db: MockDb, options: HandlerOptions = {}) {
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(latency) : Promise.resolve());

  const authenticate = (
    request: Request,
    adminOnly = false,
  ): { user: User } | { response: Response } => {
    const token = bearer(request);
    const user = db.userForToken(token);
    if (!user || !token) return { response: fail(401, "unauthorized", "Missing access token") };
    if (adminOnly && user.role !== "admin") {
      return { response: fail(403, "forbidden", "Administrator role required") };
    }
    return { user };
  };

  let syncRevision = 10;
  const sessions = new Map<string, Schemas["PlaybackSession"]>();
  const mqttBridges = new Map<string, Schemas["MqttBridgeSummary"]>();
  let seeded = false;

  function ensureSeeded(userId: string) {
    if (seeded) return;
    seeded = true;
    sessions.set("living-tv-demo", {
      id: "living-tv-demo",
      userId,
      deviceName: "客厅 Apple TV",
      clientKind: "tvos",
      mediaKind: "audio",
      title: "晴天",
      subtitle: "周杰伦",
      state: "playing",
      positionMs: 64_000,
      durationMs: 269_000,
      volume: 65,
      ambientHex: "#D97706",
      updatedAt: db.iso(-5_000),
    });
  }

  return [
    // Incremental Sync
    http.get(`${BASE}/sync/changes`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const url = new URL(request.url);
      const since = Number.parseInt(url.searchParams.get("since") ?? "0", 10);
      if (!Number.isInteger(since) || since < 0) {
        return fail(400, "validation_failed", "since must be a nonnegative integer");
      }
      const libraries: Schemas["SyncLibraryDelta"][] =
        since < syncRevision
          ? db.libraries.map((lib, idx) => ({
              id: lib.id,
              name: lib.name,
              kind: lib.kind,
              lastScanAt: lib.lastScanAt,
              updatedAt: lib.createdAt,
              revision: idx + 1,
              deleted: false,
            }))
          : [];
      const payload: Schemas["SyncChangesResponse"] = {
        latestRevision: syncRevision,
        since,
        hasMore: false,
        libraries,
        items: [],
        userStates: [],
        playlists: [],
        readingBookmarks: [],
      };
      return HttpResponse.json(payload);
    }),

    http.post(`${BASE}/sync/push`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const body = (await request.json().catch(() => null)) as Schemas["SyncPushRequest"] | null;
      if (!body || !body.clientId?.trim()) {
        return fail(400, "validation_failed", "clientId is required");
      }
      syncRevision += 1;
      const response: Schemas["SyncPushResponse"] = {
        applied: body.mutations?.length ?? 0,
        conflicts: [],
        latestRevision: syncRevision,
      };
      return HttpResponse.json(response);
    }),

    // Playback Sessions & Remote Cast
    http.post(`${BASE}/sessions/playback/heartbeat`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      ensureSeeded(auth.user.id);
      const body = (await request.json().catch(() => null)) as Schemas["PlaybackHeartbeatRequest"] | null;
      if (!body || !body.sessionId?.trim() || !body.deviceName?.trim()) {
        return fail(400, "validation_failed", "sessionId and deviceName are required");
      }
      const existing = sessions.get(body.sessionId);
      if (existing && existing.userId !== auth.user.id) {
        return fail(403, "forbidden", "Playback session belongs to another user");
      }
      const session: Schemas["PlaybackSession"] = {
        id: body.sessionId.trim(),
        userId: auth.user.id,
        deviceName: body.deviceName.trim(),
        clientKind: body.clientKind,
        mediaKind: body.mediaKind,
        itemId: body.itemId ?? null,
        libraryId: body.libraryId ?? null,
        title: body.title ?? null,
        subtitle: body.subtitle ?? null,
        artworkId: body.artworkId ?? null,
        state: body.state,
        positionMs: Math.max(0, body.positionMs ?? 0),
        durationMs: Math.max(0, body.durationMs ?? 0),
        volume: Math.max(0, Math.min(100, body.volume ?? 80)),
        ambientHex: body.ambientHex ?? null,
        updatedAt: db.iso(0),
      };
      sessions.set(session.id, session);
      db.emit("playback.session.updated", {
        sessionId: session.id,
        deviceName: session.deviceName,
        state: session.state,
        itemId: session.itemId,
        title: session.title,
        positionMs: session.positionMs,
        volume: session.volume,
        ambientHex: session.ambientHex,
      });
      return HttpResponse.json(session);
    }),

    http.get(`${BASE}/sessions/playback`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      ensureSeeded(auth.user.id);
      const all = new URL(request.url).searchParams.get("all") === "true" && auth.user.role === "admin";
      const list = [...sessions.values()].filter((s) => all || s.userId === auth.user.id);
      return HttpResponse.json({ sessions: list } satisfies Schemas["PlaybackSessionList"]);
    }),

    http.delete(`${BASE}/sessions/playback/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const id = String(params.id);
      const existing = sessions.get(id);
      if (!existing) return fail(404, "not_found", "Playback session not found");
      if (existing.userId !== auth.user.id && auth.user.role !== "admin") {
        return fail(403, "forbidden", "Cannot delete another user's playback session");
      }
      sessions.delete(id);
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/sessions/playback/:id/command`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      ensureSeeded(auth.user.id);
      const id = String(params.id);
      const target = sessions.get(id);
      if (!target) return fail(404, "not_found", "Playback session not found");
      if (target.userId !== auth.user.id && auth.user.role !== "admin") {
        return fail(403, "forbidden", "Cannot control another user's playback session");
      }
      const body = (await request.json().catch(() => null)) as Schemas["PlaybackCommandRequest"] | null;
      if (!body || !body.command) {
        return fail(400, "validation_failed", "command is required");
      }
      if (body.command === "play") target.state = "playing";
      else if (body.command === "pause") target.state = "paused";
      else if (body.command === "toggle") target.state = target.state === "playing" ? "paused" : "playing";
      else if (body.command === "volume" && typeof body.volume === "number") target.volume = body.volume;
      else if (body.command === "seek" && typeof body.positionMs === "number") target.positionMs = body.positionMs;
      target.updatedAt = db.iso(0);

      db.emit("playback.command", {
        targetSessionId: target.id,
        command: body.command,
        positionMs: body.positionMs ?? null,
        volume: body.volume ?? null,
        itemId: body.itemId ?? null,
        issuedBy: auth.user.username,
      });

      const ack: Schemas["PlaybackCommandAck"] = {
        accepted: true,
        targetSessionId: target.id,
        command: body.command,
      };
      return HttpResponse.json(ack, { status: 202 });
    }),

    // MQTT Bridges
    http.get(`${BASE}/home/mqtt`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({
        bridges: [...mqttBridges.values()],
      } satisfies Schemas["MqttBridgeList"]);
    }),

    http.post(`${BASE}/home/mqtt`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const body = (await request.json().catch(() => null)) as Schemas["SaveMqttBridgeRequest"] | null;
      if (!body || !body.name?.trim() || !body.host?.trim()) {
        return fail(400, "validation_failed", "name and host are required");
      }
      const id = body.id?.trim() || `mqtt-${Date.now().toString(36)}-${Math.random().toString(16).slice(2, 6)}`;
      const existing = mqttBridges.get(id);
      const summary: Schemas["MqttBridgeSummary"] = {
        id,
        name: body.name.trim(),
        host: body.host.trim(),
        port: body.port ?? 1883,
        username: body.username?.trim() || null,
        passwordConfigured: Boolean(body.password?.trim() || existing?.passwordConfigured),
        topicPrefix: body.topicPrefix?.trim() || "palveni",
        ambientLightDeviceId: body.ambientLightDeviceId?.trim() || null,
        publishPlayback: body.publishPlayback ?? true,
        enabled: body.enabled ?? true,
        lastPublishedAt: existing?.lastPublishedAt ?? null,
        createdAt: existing?.createdAt ?? db.iso(0),
      };
      mqttBridges.set(id, summary);
      return HttpResponse.json(summary, { status: 201 });
    }),

    http.delete(`${BASE}/home/mqtt/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const id = String(params.id);
      if (!mqttBridges.delete(id)) {
        return fail(404, "not_found", "MQTT bridge not found");
      }
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/home/mqtt/:id/test`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const id = String(params.id);
      const bridge = mqttBridges.get(id);
      if (!bridge) return fail(404, "not_found", "MQTT bridge not found");
      bridge.lastPublishedAt = db.iso(0);
      const result: Schemas["MqttTestResponse"] = {
        ok: true,
        topic: `${bridge.topicPrefix}/status`,
        error: null,
      };
      return HttpResponse.json(result);
    }),
  ];
}
