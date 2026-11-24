/**
 * MSW handlers for the PROVISIONAL APIs (see src/provisional/types.ts). Only installed by the
 * mock server; each group answers 404 unless its feature flag is reported, like a real M0 server.
 */
import { delay, http, HttpResponse } from "msw";
import type { DevicePatch } from "@/blocks/types";
import type {
  ApproveRequest,
  CreateDownloaderClientRequest,
  CreateDownloadTaskRequest,
  CreateHomeDeviceRequest,
  CreateHomeRuleRequest,
  CreateSubscriptionRequest,
  OrganizeApplyRequest,
  OrganizePreviewRequest,
  SaveHomeBridgeRequest,
  SendMessageRequest,
  UpdateAssistantConfigRequest,
} from "@/provisional/types";
import type { MockDb } from "../db";
import type { ProvisionalState } from "./state";

const BASE = "/api/v1";

/** Mock cover art (original, drawn for this project): artwork id → dev-server URL. */
const ARTWORK = Object.fromEntries(
  Object.entries(import.meta.glob<string>("./artwork/*.png", { eager: true, import: "default", query: "?url" })).map(([path, url]) => [
    path.slice("./artwork/".length, -".png".length),
    url,
  ]),
);

async function serveFile(url: string | undefined, contentType: string) {
  if (!url) return fail(404, "not_found", "Not found");
  const response = await fetch(url);
  if (!response.ok) return fail(404, "not_found", "Not found");
  return new HttpResponse(await response.arrayBuffer(), { headers: { "Content-Type": contentType, "Cache-Control": "public, max-age=3600" } });
}

function fail(status: number, code: string, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

async function body<T>(request: Request): Promise<Partial<T>> {
  try {
    const parsed: unknown = await request.json();
    return typeof parsed === "object" && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

export function createProvisionalHandlers(db: MockDb, state: ProvisionalState, options: { latency?: number } = {}) {
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(latency) : Promise.resolve());
  const guard = (request: Request, feature: string) => {
    const header = request.headers.get("Authorization");
    const token = header?.replace(/^Bearer\s+/i, "") ?? null;
    if (!db.userForToken(token)) return fail(401, "unauthorized", "Missing or invalid access token");
    if (!db.features.includes(feature)) return fail(404, "not_found", "Not found");
    return null;
  };
  const adminGuard = (request: Request, feature: string) => {
    const denied = guard(request, feature);
    if (denied) return denied;
    const token = request.headers.get("Authorization")?.replace(/^Bearer\s+/i, "") ?? null;
    return db.userForToken(token)?.role === "admin" ? null : fail(403, "forbidden", "Admins only");
  };
  /** Public skin package URLs: no token, but only while the server offers skins. */
  const publicSkin = (id: string) => (db.features.includes("skins") && state.skins.isInstalled(id) ? null : fail(404, "not_found", "Not found"));

  return [
    /* ------------------------------------------------------------ tasks */
    http.get(`${BASE}/tasks`, async ({ request }) => {
      await wait();
      return guard(request, "tasks") ?? HttpResponse.json({ tasks: state.tasks.slice(0, 100) });
    }),
    http.post(`${BASE}/tasks/:id/approve`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "tasks");
      if (denied) return denied;
      const { selectedRowIds } = await body<ApproveRequest>(request);
      const task = state.approveTask(String(params.id), Array.isArray(selectedRowIds) ? selectedRowIds : []);
      return task ? HttpResponse.json(task) : fail(404, "not_found", "Task not found");
    }),
    http.post(`${BASE}/tasks/:id/cancel`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "tasks");
      if (denied) return denied;
      const task = state.cancelTask(String(params.id));
      return task ? HttpResponse.json(task) : fail(404, "not_found", "Task not found");
    }),

    /* ------------------------------------------------------------ assistant */
    http.get(`${BASE}/assistant/conversation`, async ({ request }) => {
      await wait();
      return guard(request, "assistant") ?? HttpResponse.json(state.conversation);
    }),
    http.post(`${BASE}/assistant/conversation/reset`, async ({ request }) => {
      await wait();
      return guard(request, "assistant") ?? HttpResponse.json(state.reset());
    }),
    http.get(`${BASE}/assistant/suggestions`, async ({ request }) => {
      await wait();
      return guard(request, "assistant") ?? HttpResponse.json({ suggestions: state.suggestions });
    }),
    http.post(`${BASE}/assistant/messages`, async ({ request }) => {
      await wait();
      const denied = guard(request, "assistant");
      if (denied) return denied;
      const { text, context } = await body<SendMessageRequest>(request);
      if (typeof text !== "string" || !text.trim()) return fail(400, "validation_failed", "text is required");
      return HttpResponse.json(state.send(text.trim(), context));
    }),
    http.post(`${BASE}/assistant/runs/:id/approve`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "assistant");
      if (denied) return denied;
      const { selectedRowIds } = await body<ApproveRequest>(request);
      const run = state.approveRun(String(params.id), Array.isArray(selectedRowIds) ? selectedRowIds : []);
      return run ? HttpResponse.json(run) : fail(404, "not_found", "Run not found");
    }),
    http.post(`${BASE}/assistant/runs/:id/cancel`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "assistant");
      if (denied) return denied;
      const run = state.cancelRun(String(params.id));
      return run ? HttpResponse.json(run) : fail(404, "not_found", "Run not found");
    }),
    http.post(`${BASE}/assistant/runs/:id/background`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "assistant");
      if (denied) return denied;
      const result = state.background(String(params.id));
      return result ? HttpResponse.json(result) : fail(404, "not_found", "Run not found");
    }),
    http.get(`${BASE}/assistant/config`, async ({ request }) => {
      await wait();
      return guard(request, "assistant") ?? HttpResponse.json(state.assistantConfig);
    }),
    http.put(`${BASE}/assistant/config`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "assistant");
      if (denied) return denied;
      const payload = await body<UpdateAssistantConfigRequest>(request);
      return HttpResponse.json(state.updateConfig(payload));
    }),
    http.get(`${BASE}/assistant/memories`, async ({ request }) => {
      await wait();
      return guard(request, "assistant") ?? HttpResponse.json({ memories: state.memories });
    }),
    http.post(`${BASE}/assistant/memories`, async ({ request }) => {
      await wait();
      const denied = guard(request, "assistant");
      if (denied) return denied;
      const { content } = await body<{ content: string }>(request);
      if (typeof content !== "string" || !content.trim()) return fail(400, "validation_failed", "content is required");
      return HttpResponse.json(state.addMemory(content), { status: 201 });
    }),
    http.delete(`${BASE}/assistant/memories/:id`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "assistant");
      if (denied) return denied;
      return state.deleteMemory(String(params.id)) ? new HttpResponse(null, { status: 204 }) : fail(404, "not_found", "Memory not found");
    }),

    /* ------------------------------------------------------------ home */
    http.get(`${BASE}/home`, async ({ request }) => {
      await wait();
      return guard(request, "home") ?? HttpResponse.json(state.home);
    }),
    http.post(`${BASE}/home/devices`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "home");
      if (denied) return denied;
      const payload = await body<CreateHomeDeviceRequest>(request);
      if (!payload.roomName?.trim() || !payload.name?.trim() || !payload.kind) {
        return fail(400, "validation_failed", "roomName, name and kind are required");
      }
      return HttpResponse.json(state.createDevice(payload as CreateHomeDeviceRequest), { status: 201 });
    }),
    http.post(`${BASE}/home/devices/:id`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "home");
      if (denied) return denied;
      const device = state.updateDevice(String(params.id), (await body<DevicePatch>(request)) as DevicePatch);
      if (!device) return fail(404, "not_found", "Device not found");
      return device.online ? HttpResponse.json(device) : fail(409, "conflict", "Device is offline");
    }),
    http.delete(`${BASE}/home/devices/:id`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "home");
      if (denied) return denied;
      return state.deleteDevice(String(params.id)) ? new HttpResponse(null, { status: 204 }) : fail(404, "not_found", "Device not found");
    }),
    http.post(`${BASE}/home/scenes/:id/activate`, async ({ request, params }) => {
      await wait();
      return guard(request, "home") ?? HttpResponse.json(state.activateScene(String(params.id)));
    }),
    http.post(`${BASE}/home/rules`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "home");
      if (denied) return denied;
      const payload = await body<CreateHomeRuleRequest>(request);
      if (!payload.trigger?.trim() || !payload.action?.trim()) {
        return fail(400, "validation_failed", "trigger and action are required");
      }
      return HttpResponse.json(state.createRule(payload as CreateHomeRuleRequest), { status: 201 });
    }),
    http.post(`${BASE}/home/rules/:id`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "home");
      if (denied) return denied;
      const { enabled } = await body<{ enabled: boolean }>(request);
      const rule = state.setRule(String(params.id), enabled === true);
      return rule ? HttpResponse.json(rule) : fail(404, "not_found", "Rule not found");
    }),
    http.delete(`${BASE}/home/rules/:id`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "home");
      if (denied) return denied;
      return state.deleteRule(String(params.id)) ? new HttpResponse(null, { status: 204 }) : fail(404, "not_found", "Rule not found");
    }),
    http.get(`${BASE}/home/bridges`, async ({ request }) => {
      await wait();
      return adminGuard(request, "home") ?? HttpResponse.json({ bridges: state.bridges });
    }),
    http.post(`${BASE}/home/bridges`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "home");
      if (denied) return denied;
      const payload = await body<SaveHomeBridgeRequest>(request);
      if (!payload.name?.trim() || !payload.baseUrl?.trim()) {
        return fail(400, "validation_failed", "name and baseUrl are required");
      }
      return HttpResponse.json(state.saveBridge(payload as SaveHomeBridgeRequest), { status: 201 });
    }),
    http.delete(`${BASE}/home/bridges/:id`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "home");
      if (denied) return denied;
      return state.deleteBridge(String(params.id)) ? new HttpResponse(null, { status: 204 }) : fail(404, "not_found", "Bridge not found");
    }),
    http.post(`${BASE}/home/bridges/:id/sync`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "home");
      if (denied) return denied;
      const result = state.syncBridge(String(params.id));
      return result ? HttpResponse.json(result) : fail(404, "not_found", "Bridge not found");
    }),

    /* ------------------------------------------------------------ storage and metrics */
    http.get(`${BASE}/system/storage`, async ({ request }) => {
      await wait();
      return guard(request, "storage") ?? HttpResponse.json(state.storage);
    }),
    http.post(`${BASE}/system/storage/mounts/:id/reconnect`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "storage");
      if (denied) return denied;
      const mount = state.reconnectMount(String(params.id));
      return mount ? HttpResponse.json(mount) : fail(404, "not_found", "Mount not found");
    }),
    http.get(`${BASE}/system/metrics`, async ({ request }) => {
      await wait();
      return guard(request, "metrics") ?? HttpResponse.json(state.metrics);
    }),

    /* ------------------------------------------------------------ cover art */
    http.get(`${BASE}/catalog`, async ({ request }) => {
      await wait();
      return guard(request, "artwork") ?? HttpResponse.json(state.catalog);
    }),
    http.get(`${BASE}/artwork/:id`, async ({ request, params }) => {
      const denied = guard(request, "artwork");
      if (denied) return denied;
      return serveFile(ARTWORK[String(params.id)], "image/png");
    }),

    /* ------------------------------------------------------------ skin plugins */
    http.get(`${BASE}/skins/installed`, async ({ request }) => {
      await wait();
      return guard(request, "skins") ?? HttpResponse.json({ skins: state.skins.listInstalled() });
    }),
    http.get(`${BASE}/skins/registry`, async ({ request }) => {
      await wait();
      return adminGuard(request, "skins") ?? HttpResponse.json({ skins: state.skins.listRegistry() });
    }),
    http.post(`${BASE}/skins/registry/:id/install`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "skins");
      if (denied) return denied;
      // A real server downloads, verifies and unpacks the package here.
      await delay(latency > 0 ? 600 : 0);
      const installed = state.skins.install(String(params.id));
      return installed ? HttpResponse.json(installed) : fail(404, "not_found", "Skin not found");
    }),
    http.delete(`${BASE}/skins/installed/:id`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "skins");
      if (denied) return denied;
      return state.skins.uninstall(String(params.id)) ? new HttpResponse(null, { status: 204 }) : fail(404, "not_found", "Skin not installed");
    }),
    http.get(`${BASE}/skins/installed/:id/skin.json`, ({ params }) => {
      const id = String(params.id);
      return publicSkin(id) ?? HttpResponse.json(state.skins.find(id)?.manifest as Record<string, unknown>, { headers: { "X-Palveni-Skin-Source": "official" } });
    }),
    http.get(`${BASE}/skins/installed/:id/assets/fonts/:file`, ({ params, request }) => {
      const id = String(params.id);
      const denied = publicSkin(id);
      if (denied) return denied;
      const url = state.skins.assetUrl(id, `fonts/${String(params.file)}`);
      if (!url || url.startsWith("data:")) return serveFile(url, "font/woff2");
      return HttpResponse.redirect(new URL(url, request.url).href, 307);
    }),
    http.get(`${BASE}/skins/installed/:id/assets/:file`, ({ params }) => {
      const id = String(params.id);
      return publicSkin(id) ?? serveFile(state.skins.assetUrl(id, String(params.file)), "image/png");
    }),
    http.get(`${BASE}/skins/registry/:id/preview`, ({ params }) => {
      if (!db.features.includes("skins")) return fail(404, "not_found", "Not found");
      return serveFile(state.skins.find(String(params.id))?.files.get("preview.png"), "image/png");
    }),

    /* ------------------------------------------------------------ MCP, downloads & organizer */
    http.get(`${BASE}/mcp/info`, async ({ request }) => {
      await wait();
      const denied = guard(request, "mcp");
      if (denied) return denied;
      const token = request.headers.get("Authorization")?.replace(/^Bearer\s+/i, "") ?? null;
      const isAdmin = db.userForToken(token)?.role === "admin";
      return HttpResponse.json(state.mcpInfo(isAdmin));
    }),
    http.post(`${BASE}/mcp`, async ({ request }) => {
      await wait();
      const denied = guard(request, "mcp");
      if (denied) return denied;
      const rpc = await body<{ jsonrpc?: string; id?: unknown; method?: string }>(request);
      if (rpc.id === undefined) return new HttpResponse(null, { status: 202 });
      return HttpResponse.json({
        jsonrpc: "2.0",
        id: rpc.id,
        result: rpc.method === "initialize" ? { protocolVersion: "2025-03-26", capabilities: { tools: {} } } : {},
      });
    }),

    http.get(`${BASE}/downloads`, async ({ request }) => {
      await wait();
      return guard(request, "downloads") ?? HttpResponse.json(state.downloadsOverview());
    }),
    http.post(`${BASE}/downloads/clients`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "downloads");
      if (denied) return denied;
      const payload = await body<CreateDownloaderClientRequest>(request);
      if (!payload.name?.trim() || !payload.baseUrl?.trim() || !payload.kind) {
        return fail(400, "validation_failed", "name, kind and baseUrl are required");
      }
      return HttpResponse.json(state.saveDownloaderClient(payload as CreateDownloaderClientRequest), { status: 201 });
    }),
    http.delete(`${BASE}/downloads/clients/:id`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "downloads");
      if (denied) return denied;
      return state.deleteDownloaderClient(String(params.id))
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Downloader client not found");
    }),
    http.post(`${BASE}/downloads/tasks`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "downloads");
      if (denied) return denied;
      const payload = await body<CreateDownloadTaskRequest>(request);
      if (!payload.uri?.trim()) return fail(400, "validation_failed", "uri is required");
      const res = state.createDownloadTask(payload as CreateDownloadTaskRequest);
      return res ? HttpResponse.json(res) : fail(400, "validation_failed", "No enabled downloader client");
    }),
    http.post(`${BASE}/downloads/subscriptions`, async ({ request }) => {
      await wait();
      const denied = guard(request, "downloads");
      if (denied) return denied;
      const token = request.headers.get("Authorization")?.replace(/^Bearer\s+/i, "") ?? null;
      const user = db.userForToken(token);
      const payload = await body<CreateSubscriptionRequest>(request);
      if (!payload.title?.trim() || !payload.mediaKind) {
        return fail(400, "validation_failed", "title and mediaKind are required");
      }
      return HttpResponse.json(state.createSubscription(user?.id ?? "u-1", payload as CreateSubscriptionRequest), { status: 201 });
    }),
    http.delete(`${BASE}/downloads/subscriptions/:id`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "downloads");
      if (denied) return denied;
      return state.deleteSubscription(String(params.id))
        ? new HttpResponse(null, { status: 204 })
        : fail(404, "not_found", "Subscription not found");
    }),
    http.post(`${BASE}/downloads/subscriptions/:id/check`, async ({ request, params }) => {
      await wait();
      const denied = guard(request, "downloads");
      if (denied) return denied;
      const res = state.checkSubscription(String(params.id));
      return res ? HttpResponse.json(res) : fail(404, "not_found", "Subscription not found");
    }),

    http.post(`${BASE}/organizer/preview`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "organizer");
      if (denied) return denied;
      const payload = await body<OrganizePreviewRequest>(request);
      if (!payload.libraryId || !payload.template?.trim()) {
        return fail(400, "validation_failed", "libraryId and template are required");
      }
      const res = state.previewOrganize(payload as OrganizePreviewRequest);
      return res ? HttpResponse.json(res) : fail(404, "not_found", "Library not found");
    }),
    http.post(`${BASE}/organizer/apply`, async ({ request }) => {
      await wait();
      const denied = adminGuard(request, "organizer");
      if (denied) return denied;
      const token = request.headers.get("Authorization")?.replace(/^Bearer\s+/i, "") ?? null;
      const user = db.userForToken(token);
      const payload = await body<OrganizeApplyRequest>(request);
      if (!payload.libraryId || !payload.template?.trim() || !Array.isArray(payload.selectedItemIds)) {
        return fail(400, "validation_failed", "libraryId, template and selectedItemIds are required");
      }
      const batch = state.applyOrganize(user?.id ?? "u-admin", payload as OrganizeApplyRequest);
      return batch ? HttpResponse.json(batch) : fail(404, "not_found", "Library not found");
    }),
    http.get(`${BASE}/organizer/batches`, async ({ request }) => {
      await wait();
      return adminGuard(request, "organizer") ?? HttpResponse.json({ batches: state.organizeBatches, presets: state.organizerPresets });
    }),
    http.post(`${BASE}/organizer/batches/:id/undo`, async ({ request, params }) => {
      await wait();
      const denied = adminGuard(request, "organizer");
      if (denied) return denied;
      const result = state.undoOrganizeBatch(String(params.id));
      if (!result) return fail(404, "not_found", "Organize batch not found");
      if (result === "conflict") return fail(409, "conflict", "Organize batch already undone");
      return HttpResponse.json(result);
    }),
  ];
}
