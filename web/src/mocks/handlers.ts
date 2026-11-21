import { delay, http, HttpResponse } from "msw";
import type {
  ApiErrorDetails, ApiErrorCode, CreateLibraryRequest, Credentials, LoginRequest, RuntimeSettings, User } from "@/api/types";
import { ITEM_KIND_ICON, LIBRARY_KINDS } from "@/lib/kinds";
import type { MockDb } from "./db";

const BASE = "/api/v1";
const ITEM_KINDS = Object.keys(ITEM_KIND_ICON);

export interface HandlerOptions {
  /** Simulated network latency in ms (0 in tests). */
  latency?: number;
}

function fail(status: number, code: ApiErrorCode, message: string, details?: ApiErrorDetails) {
  return HttpResponse.json({ error: details ? { code, message, details } : { code, message } }, { status });
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

function isAbsolute(path: string) {
  return /^\//.test(path) || /^[A-Za-z]:[\\/]/.test(path) || /^\\\\[^\\]+\\/.test(path);
}

function encodeCursor(offset: number) {
  return btoa(`o:${offset}`);
}

function decodeCursor(cursor: string): number | null {
  try {
    const match = /^o:(\d+)$/.exec(atob(cursor));
    return match ? Number(match[1]) : null;
  } catch {
    return null;
  }
}

export function createHandlers(db: MockDb, options: HandlerOptions = {}) {
  const latency = options.latency ?? 0;
  const wait = () => (latency > 0 ? delay(latency + Math.round(Math.random() * latency)) : Promise.resolve());

  type Guarded = { user: User; token: string } | { response: Response };
  const authenticate = (request: Request, adminOnly = false): Guarded => {
    const token = bearer(request);
    const user = db.userForToken(token);
    if (!user || !token) return { response: fail(401, "unauthorized", "Missing or invalid access token") };
    if (adminOnly && user.role !== "admin") return { response: fail(403, "forbidden", "Administrator role required") };
    return { user, token };
  };

  return [
    /* ------------------------------------------------------------ system */
    http.get(`${BASE}/system/info`, async () => {
      await wait();
      return HttpResponse.json({
        name: db.runtimeSettings.serverName ?? db.name,
        version: db.version,
        serverId: db.serverId,
        setupCompleted: db.setupCompleted,
        features: db.features,
      });
    }),

    http.get(`${BASE}/system/health`, () => HttpResponse.json({ status: "ok" })),

    http.get(`${BASE}/system/settings`, ({ request }) => {
      const auth = authenticate(request, true);
      return "response" in auth ? auth.response : HttpResponse.json(db.runtimeSettings);
    }),

    http.put(`${BASE}/system/settings`, async ({ request }) => {
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const body = await readJson<RuntimeSettings>(request);
      const name = body?.serverName;
      const tier = body?.perfTier;
      if (name !== null && (typeof name !== "string" || !name.trim() || [...name.trim()].length > 64)) {
        return fail(400, "validation_failed", "Invalid server name", { field: "serverName" });
      }
      if (tier !== null && (!tier || !["tiny", "lite", "standard", "performance"].includes(tier))) {
        return fail(400, "validation_failed", "Invalid performance tier", { field: "perfTier" });
      }
      db.runtimeSettings = { serverName: name?.trim() ?? null, perfTier: tier };
      return HttpResponse.json(db.runtimeSettings);
    }),

    http.get(`${BASE}/system/hardware`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      if (new URL(request.url).searchParams.get("refresh") === "true") {
        await delay(latency > 0 ? 1600 : 0);
        db.hardware = { ...db.hardware, probedAt: db.iso() };
      }
      return HttpResponse.json({ ...db.hardware, perfTier: db.runtimeSettings.perfTier ?? db.hardware.perfTier });
    }),

    /* ------------------------------------------------------------ setup and auth */
    http.post(`${BASE}/setup`, async ({ request }) => {
      await wait();
      if (db.setupCompleted) return fail(409, "setup_already_completed", "Setup has already been completed");
      const body = await readJson<Credentials>(request);
      const username = typeof body?.username === "string" ? body.username : "";
      const password = typeof body?.password === "string" ? body.password : "";
      if (username.length < 1 || [...username].length > 64 || /\s/.test(username)) {
        return fail(400, "validation_failed", "username must be 1-64 characters without whitespace");
      }
      if ([...password].length < 8) return fail(400, "validation_failed", "password must be at least 8 characters");
      const user = db.addUser(username, password, "admin");
      return HttpResponse.json({ user }, { status: 201 });
    }),

    http.post(`${BASE}/auth/login`, async ({ request }) => {
      await wait();
      const body = await readJson<LoginRequest>(request);
      if (typeof body?.username !== "string" || typeof body.password !== "string") {
        return fail(400, "validation_failed", "username and password are required");
      }
      const user = db.findUserByCredentials(body.username, body.password);
      if (!user) return fail(401, "invalid_credentials", "Invalid username or password");
      return HttpResponse.json({ accessToken: db.issueToken(user), tokenType: "Bearer", expiresAt: null, user });
    }),

    http.post(`${BASE}/auth/logout`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      db.revoke(auth.token);
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/auth/me`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      return HttpResponse.json(auth.user);
    }),

    /* ------------------------------------------------------------ libraries */
    http.get(`${BASE}/libraries`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ libraries: db.libraries });
    }),

    http.post(`${BASE}/libraries`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const body = await readJson<CreateLibraryRequest>(request);
      const name = typeof body?.name === "string" ? body.name.trim() : "";
      const kind = body?.kind;
      const paths = Array.isArray(body?.paths) ? body.paths.filter((path): path is string => typeof path === "string") : [];
      if (!name) return fail(400, "validation_failed", "name is required", { field: "name" });
      if (!kind || !LIBRARY_KINDS.includes(kind)) return fail(400, "validation_failed", "Unknown library kind", { field: "kind" });
      if (paths.length === 0) return fail(400, "validation_failed", "at least one path is required", { field: "paths" });
      const relative = paths.findIndex((path) => !isAbsolute(path));
      if (relative !== -1) return fail(400, "validation_failed", `path must be absolute: ${paths[relative]}`, { field: "paths", index: relative });
      // Mock file system: every absolute path exists except those mentioning "missing".
      const missing = paths.findIndex((path) => /missing|nonexistent/i.test(path));
      if (missing !== -1) {
        const path = paths[missing] ?? "";
        return fail(400, "path_not_found", `Path does not exist: ${path}`, { field: "paths", index: missing, path });
      }
      if (db.libraries.some((library) => library.name.toLowerCase() === name.toLowerCase())) {
        return fail(409, "conflict", `A library named "${name}" already exists`, { field: "name" });
      }
      return HttpResponse.json(db.createLibrary(name, kind, paths), { status: 201 });
    }),

    http.get(`${BASE}/libraries/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const library = db.getLibrary(String(params.id));
      return library ? HttpResponse.json(library) : fail(404, "not_found", "Library not found");
    }),

    http.delete(`${BASE}/libraries/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      if (!db.getLibrary(String(params.id))) return fail(404, "not_found", "Library not found");
      db.deleteLibrary(String(params.id));
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/libraries/:id/scan`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      if (!db.getLibrary(String(params.id))) return fail(404, "not_found", "Library not found");
      const job = db.startScan(String(params.id));
      return HttpResponse.json({ jobId: job.id }, { status: 202 });
    }),

    /* ------------------------------------------------------------ items */
    http.get(`${BASE}/items`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const search = new URL(request.url).searchParams;
      const libraryId = search.get("libraryId");
      if (!libraryId) return fail(400, "validation_failed", "libraryId is required");
      if (!db.getLibrary(libraryId)) return fail(404, "not_found", "Library not found");

      const parentId = search.get("parentId");
      if (parentId) {
        const parent = db.getItem(parentId);
        if (!parent || parent.libraryId !== libraryId) return fail(404, "not_found", "Parent item not found");
      }
      const limitParam = search.get("limit");
      const requested = limitParam === null ? 50 : Number(limitParam);
      // Above 200 is treated as 200, like the server; zero, negative or not a number is refused.
      if (!Number.isInteger(requested) || requested < 1) {
        return fail(400, "validation_failed", "limit must be a positive integer", { field: "limit" });
      }
      const cursor = search.get("cursor");
      const offset = cursor ? decodeCursor(cursor) : 0;
      if (offset === null) return fail(400, "validation_failed", "invalid cursor");
      const kind = search.get("kind");
      if (kind && !ITEM_KINDS.includes(kind)) return fail(400, "validation_failed", "unknown kind");

      let children = db.listChildren(libraryId, parentId);
      if (kind) children = children.filter((item) => item.kind === kind);
      const limit = Math.min(requested, 200);
      const page = children.slice(offset, offset + limit);
      const next = offset + limit < children.length ? encodeCursor(offset + limit) : null;
      const isAdmin = auth.user.role === "admin";
      return HttpResponse.json({
        items: isAdmin ? page : page.map((item) => ({ ...item, path: null })),
        nextCursor: next,
      });
    }),

    http.get(`${BASE}/items/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const item = db.getItem(String(params.id));
      if (!item) return fail(404, "not_found", "Item not found");
      return HttpResponse.json(auth.user.role === "admin" ? item : { ...item, path: null });
    }),

    /* ------------------------------------------------------------ jobs and plugins */
    http.get(`${BASE}/jobs`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const limitParam = new URL(request.url).searchParams.get("limit");
      const requested = limitParam === null ? 50 : Number(limitParam);
      if (!Number.isInteger(requested) || requested < 1) {
        return fail(400, "validation_failed", "limit must be a positive integer", { field: "limit" });
      }
      const limit = Math.min(requested, 200);
      const jobs = [...db.jobs].sort((a, b) => b.createdAt.localeCompare(a.createdAt)).slice(0, limit);
      return HttpResponse.json({ jobs });
    }),

    http.get(`${BASE}/plugins`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ plugins: db.plugins });
    }),

    /* ------------------------------------------------------------ events */
    http.get(`${BASE}/events`, ({ request }) => {
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const { user } = auth;
      const encoder = new TextEncoder();
      let cleanup = () => {};

      const stream = new ReadableStream<Uint8Array>({
        start(controller) {
          let closed = false;
          const send = (chunk: string) => {
            if (closed) return;
            try {
              controller.enqueue(encoder.encode(chunk));
            } catch {
              cleanup();
            }
          };
          const unsubscribe = db.subscribe((event, data) => {
            // Job events go to administrators only.
            if (event === "job.updated" && user.role !== "admin") return;
            send(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
          });
          const ping = setInterval(() => send("event: ping\ndata: {}\n\n"), 20_000);
          cleanup = () => {
            if (closed) return;
            closed = true;
            unsubscribe();
            clearInterval(ping);
            try {
              controller.close();
            } catch {
              // Already closed by the consumer.
            }
          };
          request.signal.addEventListener("abort", () => cleanup());
          send(": connected\n\n");
        },
        cancel() {
          cleanup();
        },
      });

      return new HttpResponse(stream, {
        headers: { "Content-Type": "text/event-stream", "Cache-Control": "no-cache" },
      });
    }),
  ];
}
