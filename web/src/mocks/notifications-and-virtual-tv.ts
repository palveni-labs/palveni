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

export function createNotificationsAndVirtualTvHandlers(
  db: MockDb,
  options: HandlerOptions = {},
) {
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(latency) : Promise.resolve());

  const authenticate = (
    request: Request,
    adminOnly = false,
  ): { user: User } | { response: Response } => {
    const token = bearer(request);
    const user = db.userForToken(token);
    if (!user || !token) return { response: fail(401, "unauthorized", "Missing access token") };
    if (adminOnly && user.role !== "admin")
      return { response: fail(403, "forbidden", "Administrator role required") };
    return { user };
  };

  let notifications: Schemas["NotificationDelivery"][] = [
    {
      id: "019a4000-0000-7000-8000-000000000001",
      ownerId: null,
      channelId: null,
      eventType: "library.scanned",
      title: "媒体库扫描完成：电影",
      body: "已索引最新 4K 影片与海报元数据。",
      status: "in_app",
      error: null,
      readAt: null,
      createdAt: db.iso(-600_000),
    },
  ];

  let channels: Schemas["NotificationChannel"][] = [];
  let routines: Schemas["ScheduledRoutine"][] = [
    {
      id: "019a4000-0000-7000-8000-000000000010",
      ownerId: "00000000-0000-4000-8000-000000000001",
      name: "凌晨媒体巡检",
      scheduleKind: "daily",
      scheduleExpr: "03:00",
      actionKind: "scan_all",
      actionPayload: "",
      enabled: true,
      lastRunAt: null,
      lastStatus: null,
      createdAt: db.iso(-3_600_000),
    },
  ];
  let webhooks: Schemas["InboundWebhook"][] = [];
  const webhookTokens = new Map<string, string>();

  const defaultVideoLib = () => db.libraries.find((lib) => lib.kind === "video") ?? db.libraries[0];
  let virtualChannels: Schemas["VirtualChannel"][] = [];
  let seededVirtual = false;

  function ensureSeededVirtual() {
    if (seededVirtual) return;
    seededVirtual = true;
    const vlib = defaultVideoLib();
    if (vlib) {
      virtualChannels.push({
        id: "019a4000-0000-7000-8000-000000000020",
        number: 101,
        name: "家庭科幻影院",
        libraryId: vlib.id,
        genreFilter: "科幻",
        enabled: true,
        createdAt: db.iso(-7_200_000),
      });
    }
  }

  return [
    http.get(`${BASE}/notifications`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const unreadCount = notifications.filter(
        (n) => n.status === "in_app" && n.readAt === null,
      ).length;
      return HttpResponse.json({ notifications, unreadCount });
    }),

    http.post(`${BASE}/notifications/read-all`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      let markedRead = 0;
      const readAt = db.iso();
      notifications = notifications.map((n) => {
        if (n.readAt === null) {
          markedRead += 1;
          return { ...n, readAt };
        }
        return n;
      });
      return HttpResponse.json({ markedRead });
    }),

    http.get(`${BASE}/notifications/channels`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ channels });
    }),

    http.post(`${BASE}/notifications/channels`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const body = (await request.json()) as Schemas["CreateNotificationChannelRequest"];
      const now = db.iso();
      const created: Schemas["NotificationChannel"] = {
        id: crypto.randomUUID(),
        ownerId: auth.user.id,
        name: body.name.trim(),
        kind: body.kind,
        targetHint: body.config.target ?? body.config.url ?? null,
        hasSecret: Boolean(body.config.token || body.config.url),
        events: body.events?.length ? body.events : ["*"],
        quietHours: body.quietHours ?? null,
        enabled: body.enabled ?? true,
        createdAt: now,
        updatedAt: now,
      };
      channels = [...channels, created];
      return HttpResponse.json(created, { status: 201 });
    }),

    http.patch(`${BASE}/notifications/channels/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const patch = (await request.json()) as Schemas["UpdateNotificationChannelRequest"];
      const target = channels.find((c) => c.id === String(params.id));
      if (!target) return fail(404, "not_found", "Channel not found");
      const updated: Schemas["NotificationChannel"] = {
        ...target,
        name: patch.name?.trim() || target.name,
        events: patch.events ?? target.events,
        quietHours:
          patch.quietHours === ""
            ? null
            : patch.quietHours !== undefined
              ? patch.quietHours
              : target.quietHours,
        enabled: patch.enabled ?? target.enabled,
        updatedAt: db.iso(),
      };
      channels = channels.map((c) => (c.id === updated.id ? updated : c));
      return HttpResponse.json(updated);
    }),

    http.delete(`${BASE}/notifications/channels/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      if (!channels.some((c) => c.id === String(params.id)))
        return fail(404, "not_found", "Channel not found");
      channels = channels.filter((c) => c.id !== String(params.id));
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/notifications/channels/:id/test`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const ch = channels.find((c) => c.id === String(params.id));
      if (!ch) return fail(404, "not_found", "Channel not found");
      const delivery: Schemas["NotificationDelivery"] = {
        id: crypto.randomUUID(),
        ownerId: auth.user.id,
        channelId: ch.id,
        eventType: "notification.test",
        title: "Palveni 通知测试",
        body: `通知渠道「${ch.name}」测试消息发送成功。`,
        status: "sent",
        error: null,
        readAt: db.iso(),
        createdAt: db.iso(),
      };
      notifications = [delivery, ...notifications];
      return HttpResponse.json({ delivery });
    }),

    http.get(`${BASE}/webhooks/inbound`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ webhooks });
    }),

    http.post(`${BASE}/webhooks/inbound`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const body = (await request.json()) as Schemas["CreateInboundWebhookRequest"];
      const token = `pwh_${crypto.randomUUID().replace(/-/g, "")}`;
      const webhook: Schemas["InboundWebhook"] = {
        id: crypto.randomUUID(),
        name: body.name.trim(),
        sourceKind: body.sourceKind,
        libraryId: body.libraryId ?? null,
        enabled: body.enabled ?? true,
        lastTriggeredAt: null,
        createdAt: db.iso(),
      };
      webhooks = [webhook, ...webhooks];
      webhookTokens.set(token, webhook.id);
      return HttpResponse.json(
        {
          webhook,
          token,
          triggerPath: `/api/v1/webhooks/inbound/${token}/trigger`,
        },
        { status: 201 },
      );
    }),

    http.delete(`${BASE}/webhooks/inbound/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      if (!webhooks.some((w) => w.id === String(params.id)))
        return fail(404, "not_found", "Inbound webhook not found");
      webhooks = webhooks.filter((w) => w.id !== String(params.id));
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/webhooks/inbound/:token/trigger`, async ({ params }) => {
      await wait();
      const id = webhookTokens.get(String(params.token));
      const wh = webhooks.find((w) => w.id === id);
      if (!wh) return fail(404, "not_found", "Inbound webhook not found");
      const vlib = defaultVideoLib();
      return HttpResponse.json({
        accepted: true,
        sourceKind: wh.sourceKind,
        summary: `${wh.name} 已触发增量扫描`,
        queuedLibraryIds: vlib ? [vlib.id] : [],
      });
    }),

    http.get(`${BASE}/routines`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ routines });
    }),

    http.post(`${BASE}/routines`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const body = (await request.json()) as Schemas["CreateRoutineRequest"];
      const created: Schemas["ScheduledRoutine"] = {
        id: crypto.randomUUID(),
        ownerId: auth.user.id,
        name: body.name.trim(),
        scheduleKind: body.scheduleKind,
        scheduleExpr: body.scheduleExpr.trim(),
        actionKind: body.actionKind,
        actionPayload: body.actionPayload ?? "",
        enabled: body.enabled ?? true,
        lastRunAt: null,
        lastStatus: null,
        createdAt: db.iso(),
      };
      routines = [...routines, created];
      return HttpResponse.json(created, { status: 201 });
    }),

    http.patch(`${BASE}/routines/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const patch = (await request.json()) as Schemas["UpdateRoutineRequest"];
      const existing = routines.find((r) => r.id === String(params.id));
      if (!existing) return fail(404, "not_found", "Routine not found");
      const updated: Schemas["ScheduledRoutine"] = {
        ...existing,
        name: patch.name?.trim() || existing.name,
        scheduleKind: patch.scheduleKind ?? existing.scheduleKind,
        scheduleExpr: patch.scheduleExpr ?? existing.scheduleExpr,
        actionKind: patch.actionKind ?? existing.actionKind,
        actionPayload: patch.actionPayload ?? existing.actionPayload,
        enabled: patch.enabled ?? existing.enabled,
      };
      routines = routines.map((r) => (r.id === updated.id ? updated : r));
      return HttpResponse.json(updated);
    }),

    http.delete(`${BASE}/routines/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      if (!routines.some((r) => r.id === String(params.id)))
        return fail(404, "not_found", "Routine not found");
      routines = routines.filter((r) => r.id !== String(params.id));
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/routines/:id/run`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const existing = routines.find((r) => r.id === String(params.id));
      if (!existing) return fail(404, "not_found", "Routine not found");
      const updated: Schemas["ScheduledRoutine"] = {
        ...existing,
        lastRunAt: db.iso(),
        lastStatus: "succeeded",
      };
      routines = routines.map((r) => (r.id === updated.id ? updated : r));
      return HttpResponse.json({
        routine: updated,
        detail: `已完成「${updated.name}」`,
      });
    }),

    http.get(`${BASE}/virtual-channels`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      ensureSeededVirtual();
      return HttpResponse.json({ channels: virtualChannels });
    }),

    http.post(`${BASE}/virtual-channels`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      ensureSeededVirtual();
      const body = (await request.json()) as Schemas["CreateVirtualChannelRequest"];
      if (virtualChannels.some((c) => c.number === body.number))
        return fail(409, "conflict", `Channel ${body.number} already exists`);
      const created: Schemas["VirtualChannel"] = {
        id: crypto.randomUUID(),
        number: body.number,
        name: body.name.trim(),
        libraryId: body.libraryId,
        genreFilter: body.genreFilter ?? null,
        enabled: body.enabled ?? true,
        createdAt: db.iso(),
      };
      virtualChannels = [...virtualChannels, created].sort((a, b) => a.number - b.number);
      return HttpResponse.json(created, { status: 201 });
    }),

    http.delete(`${BASE}/virtual-channels/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      ensureSeededVirtual();
      if (!virtualChannels.some((c) => c.id === String(params.id)))
        return fail(404, "not_found", "Channel not found");
      virtualChannels = virtualChannels.filter((c) => c.id !== String(params.id));
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/virtual-channels/guide`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      ensureSeededVirtual();
      const nowMs = db.now();
      const guideChannels: Schemas["VirtualChannelGuide"][] = virtualChannels
        .filter((c) => c.enabled)
        .map((channel) => {
          const itemId = "23587a91-9e35-4821-8a48-3aeddba77d11";
          const durationMs = 2_700_000;
          const offsetMs = 600_000;
          const startedAt = new Date(nowMs - offsetMs).toISOString();
          const endsAt = new Date(nowMs - offsetMs + durationMs).toISOString();
          const nextEnd = new Date(nowMs - offsetMs + durationMs * 2).toISOString();
          return {
            channel,
            totalItems: 4,
            cycleDurationMs: durationMs * 4,
            nowPlaying: {
              itemId,
              libraryId: channel.libraryId,
              title: "天鹅绒地平线 (4K)",
              workTitle: "天鹅绒地平线",
              season: null,
              episode: null,
              offsetMs,
              durationMs,
              startedAt,
              endsAt,
            },
            programmes: [
              {
                itemId,
                libraryId: channel.libraryId,
                title: "深海回响",
                workTitle: "深海回响",
                season: null,
                episode: null,
                offsetMs: 0,
                durationMs,
                startedAt: endsAt,
                endsAt: nextEnd,
              },
            ],
          };
        });
      return HttpResponse.json({
        generatedAt: new Date(nowMs).toISOString(),
        channels: guideChannels,
        playlistUrl: "/api/v1/virtual-channels/playlist.m3u?ticket=mock-feed-ticket",
        epgUrl: "/api/v1/virtual-channels/epg.xml?ticket=mock-feed-ticket",
      });
    }),

    http.get(`${BASE}/virtual-channels/playlist.m3u`, () =>
      new HttpResponse("#EXTM3U\n", {
        headers: { "Content-Type": "audio/x-mpegurl; charset=utf-8" },
      }),
    ),

    http.get(`${BASE}/virtual-channels/epg.xml`, () =>
      new HttpResponse('<?xml version="1.0" encoding="UTF-8"?><tv></tv>', {
        headers: { "Content-Type": "application/xml; charset=utf-8" },
      }),
    ),
  ];
}
