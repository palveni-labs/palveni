/**
 * MSW handlers implementing every M0 endpoint on top of MockDb.
 * Used by the dev-only browser worker (`npm run dev:mock`) and by the test suite.
 */
import { delay, http, HttpResponse } from "msw";
import type {
  ApiErrorDetails,
  ApiErrorCode,
  ChangePasswordRequest,
  CreateBackupRequest,
  CreateLibraryRequest,
  CreateUserRequest,
  ExternalImportRequest,
  ExternalImportSource,
  UpdateLibraryRequest,
  UpdateUserRequest,
  Credentials,
  LoginRequest,
  RuntimeSettings,
  User,
} from "@/api/types";
import { ITEM_KIND_ICON, LIBRARY_KINDS } from "@/lib/kinds";
import { stableUuid } from "./fixtures";
import type { MockDb } from "./db";
import { OFFICIAL_PLUGIN_CATALOG } from "./plugins-catalog";
import { createMusicHandlers } from "./music/handlers";
import { createNotificationsAndVirtualTvHandlers } from "./notifications-and-virtual-tv";
import { createPhotosHandlers } from "./photos/handlers";
import { createReadingHandlers } from "./reading/handlers";
import { createSharesCollectionsSearchHandlers } from "./shares-collections-search/handlers";
import { createSyncCastMqttHandlers } from "./sync-cast-mqtt";
import { createVideoHandlers } from "./video/handlers";
import { createWebdavMetadataObservabilityHandlers } from "./webdav-metadata-observability";

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
  const musicClientUsers = new Map<string, boolean>();
  const clientApiKeys = new Map<
    string,
    Array<{
      id: string;
      name: string;
      clientKind: "jellyfin" | "webdav" | "general";
      keyPrefix: string;
      lastUsedAt: string | null;
      revokedAt: string | null;
      createdAt: string;
    }>
  >();
  let apiKeySeq = 1;
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
      const deviceName = typeof body.deviceName === "string" && body.deviceName.trim() ? body.deviceName.trim() : "Browser";
      return HttpResponse.json({ accessToken: db.issueToken(user, deviceName), tokenType: "Bearer", expiresAt: null, user });
    }),

    http.post(`${BASE}/auth/logout`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      db.revoke(auth.token);
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/me/subsonic-password`, ({ request }) => {
      const auth = authenticate(request);
      return "response" in auth ? auth.response : HttpResponse.json({ enabled: musicClientUsers.has(auth.user.id), personal: musicClientUsers.get(auth.user.id) ?? false });
    }),
    http.post(`${BASE}/me/subsonic-password`, ({ request }) => {
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      musicClientUsers.set(auth.user.id, new URL(request.url).searchParams.get("personal") === "true");
      return HttpResponse.json({ username: auth.user.username, password: crypto.randomUUID() }, { status: 201 });
    }),
    http.delete(`${BASE}/me/subsonic-password`, ({ request }) => {
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      musicClientUsers.delete(auth.user.id);
      return new HttpResponse(null, { status: 204 });
    }),
    http.get(`${BASE}/auth/me`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      return HttpResponse.json(auth.user);
    }),

    /* ------------------------------------------------------------ users, sessions & maintenance */
    http.get(`${BASE}/users`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ users: db.managedUsers() });
    }),

    http.post(`${BASE}/users`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const body = await readJson<CreateUserRequest>(request);
      const username = typeof body?.username === "string" ? body.username.trim() : "";
      const password = typeof body?.password === "string" ? body.password : "";
      const role = body?.role === "admin" ? "admin" : "user";
      if (!username || [...username].length > 64 || /\s/.test(username)) {
        return fail(400, "validation_failed", "Invalid username", { field: "username" });
      }
      if ([...password].length < 8) {
        return fail(400, "validation_failed", "Password must be at least 8 characters", { field: "password" });
      }
      if (db.users.some((u) => u.username.toLowerCase() === username.toLowerCase())) {
        return fail(409, "conflict", "That username is already taken", { field: "username" });
      }
      const created = db.addUser(
        username,
        password,
        role,
        db.iso(),
        body?.policy ?? {
          allLibraries: true,
          allowDownload: true,
          allowTranscode: true,
          dailyAiLimit: 100,
        },
        Array.isArray(body?.allowedLibraryIds) ? body.allowedLibraryIds : [],
      );
      const stored = db.users.find((u) => u.id === created.id)!;
      db.recordAudit("user.created", auth.user, stored.username, { role: stored.role, allLibraries: stored.policy.allLibraries });
      return HttpResponse.json(db.toManagedUser(stored), { status: 201 });
    }),

    http.patch(`${BASE}/users/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const target = db.users.find((u) => u.id === String(params.id));
      if (!target) return fail(404, "not_found", "User not found");
      const body = await readJson<UpdateUserRequest>(request);
      if (!body) return fail(400, "validation_failed", "Invalid request body");

      const nextRole = body.role ?? target.role;
      const nextDisabled = body.disabled ?? target.disabled;
      if (target.role === "admin" && !target.disabled && (nextRole !== "admin" || nextDisabled)) {
        const otherEnabledAdmins = db.users.filter((u) => u.id !== target.id && u.role === "admin" && !u.disabled).length;
        if (otherEnabledAdmins === 0) {
          return fail(409, "conflict", "Cannot disable or demote the last enabled administrator");
        }
      }
      if (typeof body.password === "string") {
        if ([...body.password].length < 8) {
          return fail(400, "validation_failed", "Password must be at least 8 characters", { field: "password" });
        }
        target.password = body.password;
        db.revokeSessionsForUser(target.id, target.id === auth.user.id ? auth.token : undefined);
      }
      target.role = nextRole;
      target.disabled = nextDisabled;
      if (nextDisabled) {
        db.revokeSessionsForUser(target.id);
      }
      if (body.policy) {
        target.policy = {
          allLibraries: body.policy.allLibraries ?? target.policy.allLibraries,
          allowDownload: body.policy.allowDownload ?? target.policy.allowDownload,
          allowTranscode: body.policy.allowTranscode ?? target.policy.allowTranscode,
          dailyAiLimit: body.policy.dailyAiLimit ?? target.policy.dailyAiLimit,
        };
      }
      if (Array.isArray(body.allowedLibraryIds)) {
        target.allowedLibraryIds = [...body.allowedLibraryIds];
      }
      target.updatedAt = db.iso();
      db.recordAudit("user.updated", auth.user, target.username, { role: target.role, disabled: target.disabled });
      return HttpResponse.json(db.toManagedUser(target));
    }),

    http.delete(`${BASE}/users/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const targetId = String(params.id);
      if (targetId === auth.user.id) {
        return fail(409, "conflict", "Cannot delete your currently signed-in administrator account");
      }
      const target = db.users.find((u) => u.id === targetId);
      if (!target) return fail(404, "not_found", "User not found");
      if (target.role === "admin" && !target.disabled) {
        const otherEnabledAdmins = db.users.filter((u) => u.id !== target.id && u.role === "admin" && !u.disabled).length;
        if (otherEnabledAdmins === 0) {
          return fail(409, "conflict", "Cannot delete the last enabled administrator");
        }
      }
      db.revokeSessionsForUser(target.id);
      db.users = db.users.filter((u) => u.id !== targetId);
      db.recordAudit("user.deleted", auth.user, target.username, { userId: target.id });
      return new HttpResponse(null, { status: 204 });
    }),

    http.put(`${BASE}/me/password`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const body = await readJson<ChangePasswordRequest>(request);
      const currentPassword = typeof body?.currentPassword === "string" ? body.currentPassword : "";
      const newPassword = typeof body?.newPassword === "string" ? body.newPassword : "";
      if ([...newPassword].length < 8) {
        return fail(400, "validation_failed", "Password must be at least 8 characters", { field: "newPassword" });
      }
      const stored = db.users.find((u) => u.id === auth.user.id);
      if (!stored || stored.password !== currentPassword) {
        return fail(401, "invalid_credentials", "Invalid username or password", { field: "currentPassword" });
      }
      stored.password = newPassword;
      stored.updatedAt = db.iso();
      if (body?.revokeOtherSessions !== false) {
        db.revokeSessionsForUser(stored.id, auth.token);
      }
      db.recordAudit("auth.password_changed", auth.user, auth.user.username, null);
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/me/sessions`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ sessions: db.listSessionsForUser(auth.user.id, auth.token) });
    }),

    http.delete(`${BASE}/me/sessions/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      if (!db.revokeUserSession(auth.user.id, String(params.id))) {
        return fail(404, "not_found", "Session not found");
      }
      db.recordAudit("auth.session_revoked", auth.user, String(params.id), null);
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/system/audit`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const url = new URL(request.url);
      const prefix = url.searchParams.get("action")?.trim() ?? "";
      const limit = Math.min(200, Math.max(1, Number(url.searchParams.get("limit") ?? 50) || 50));
      const cursor = url.searchParams.get("cursor");
      let filtered = prefix ? db.auditLogs.filter((e) => e.action.startsWith(prefix)) : db.auditLogs;
      if (cursor) {
        const idx = filtered.findIndex((e) => e.id === cursor);
        if (idx >= 0) filtered = filtered.slice(idx + 1);
      }
      const entries = filtered.slice(0, limit);
      const nextCursor = filtered.length > limit ? entries[entries.length - 1]?.id ?? null : null;
      return HttpResponse.json({ entries, nextCursor });
    }),

    http.get(`${BASE}/system/backups`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ backups: db.backups.map((b) => b.summary) });
    }),

    http.post(`${BASE}/system/backups`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const body = await readJson<CreateBackupRequest>(request);
      const id = stableUuid(`backup:${db.now()}:${db.backups.length}:${Math.random()}`);
      const name = typeof body?.name === "string" && body.name.trim() ? body.name.trim() : `Snapshot ${db.iso()}`;
      const snapshotData = {
        formatVersion: 1,
        serverVersion: db.version,
        createdAt: db.iso(),
        libraries: structuredClone(db.libraries),
        runtimeSettings: structuredClone(db.runtimeSettings),
      };
      const payloadJson = JSON.stringify(snapshotData, null, 2);
      const sha256 = stableUuid(`sha:${payloadJson}`).replace(/-/g, "").padEnd(64, "a");
      const summary = {
        id,
        name,
        creatorId: auth.user.id,
        creatorUsername: auth.user.username,
        sizeBytes: payloadJson.length,
        sha256,
        counts: {
          libraries: db.libraries.length,
          settings: 1,
          playlists: 0,
          itemStates: 0,
          photoAlbums: 0,
          bookmarks: 0,
          homeDevices: 0,
          homeScenes: 0,
          homeRules: 0,
          memories: 0,
        },
        createdAt: db.iso(),
      };
      db.backups.unshift({
        summary,
        payloadJson,
        snapshot: {
          libraries: structuredClone(db.libraries),
          runtimeSettings: structuredClone(db.runtimeSettings),
        },
      });
      db.recordAudit("system.backup_created", auth.user, id, { name });
      return HttpResponse.json(summary, { status: 201 });
    }),

    http.get(`${BASE}/system/backups/:id/download`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const found = db.backups.find((b) => b.summary.id === String(params.id));
      if (!found) return fail(404, "not_found", "Backup not found");
      return new HttpResponse(found.payloadJson, {
        status: 200,
        headers: {
          "Content-Type": "application/json; charset=utf-8",
          "Content-Disposition": `attachment; filename="palveni-backup-${found.summary.id}.json"`,
        },
      });
    }),

    http.post(`${BASE}/system/backups/:id/restore`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const found = db.backups.find((b) => b.summary.id === String(params.id));
      if (!found) return fail(404, "not_found", "Backup not found");
      db.libraries = structuredClone(found.snapshot.libraries);
      db.runtimeSettings = structuredClone(found.snapshot.runtimeSettings);
      db.recordAudit("system.backup_restored", auth.user, found.summary.id, { librariesRestored: db.libraries.length });
      return HttpResponse.json({
        backupId: found.summary.id,
        sha256Verified: true,
        librariesRestored: db.libraries.length,
        settingsRestored: 1,
        playlistsRestored: 0,
        itemStatesRestored: 0,
        photoAlbumsRestored: 0,
        bookmarksRestored: 0,
        homeEntitiesRestored: 0,
        memoriesRestored: 0,
      });
    }),

    http.delete(`${BASE}/system/backups/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const targetId = String(params.id);
      const before = db.backups.length;
      db.backups = db.backups.filter((b) => b.summary.id !== targetId);
      if (db.backups.length === before) return fail(404, "not_found", "Backup not found");
      db.recordAudit("system.backup_deleted", auth.user, targetId, null);
      return new HttpResponse(null, { status: 204 });
    }),

    http.post(`${BASE}/system/import`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      const body = await readJson<ExternalImportRequest>(request);
      const content = typeof body?.content === "string" ? body.content.trim() : "";
      if (!content) return fail(400, "validation_failed", "Import content must not be empty");
      const dryRun = body?.dryRun !== false;
      const allLeaves = db.allLeafItems();

      let detectedSource: ExternalImportSource = body?.source && body.source !== "auto" ? body.source : "jellyfin";
      const candidates: Array<{ title: string; path?: string; favorite: boolean; history: boolean; isPlaylist: boolean }> = [];

      if (!content.startsWith("{") && !content.startsWith("[")) {
        detectedSource = "m3u";
        for (const line of content.split(/\r?\n/)) {
          const trimmed = line.trim();
          if (!trimmed || trimmed.startsWith("#")) continue;
          candidates.push({ title: trimmed, path: trimmed, favorite: false, history: false, isPlaylist: true });
        }
      } else {
        const pickStr = (rec: Record<string, unknown>, keys: string[], fallback: string): string => {
          for (const key of keys) {
            const val = rec[key];
            if (typeof val === "string" && val.trim()) return val.trim();
          }
          return fallback;
        };
        try {
          const parsed = JSON.parse(content) as Record<string, unknown>;
          if ("starred" in parsed || "subsonic-response" in parsed) {
            detectedSource = "navidrome";
            const arr = Array.isArray(parsed.starred) ? parsed.starred : [];
            for (const row of arr) {
              if (typeof row === "object" && row !== null) {
                const rec = row as Record<string, unknown>;
                candidates.push({
                  title: pickStr(rec, ["title", "path"], "Track"),
                  path: typeof rec.path === "string" ? rec.path : undefined,
                  favorite: true,
                  history: Number(rec.playCount ?? 0) > 0,
                  isPlaylist: false,
                });
              }
            }
          } else {
            const arr = Array.isArray(parsed) ? parsed : Array.isArray(parsed.Items) ? parsed.Items : [];
            for (const row of arr) {
              if (typeof row === "object" && row !== null) {
                const rec = row as Record<string, unknown>;
                const ud = (typeof rec.UserData === "object" && rec.UserData !== null ? rec.UserData : rec) as Record<string, unknown>;
                candidates.push({
                  title: pickStr(rec, ["Name", "Title", "Path"], "Item"),
                  path: typeof rec.Path === "string" ? rec.Path : undefined,
                  favorite: Boolean(ud.IsFavorite),
                  history: Boolean(ud.Played) || Number(ud.PlayCount ?? 0) > 0 || Number(ud.PlaybackPositionTicks ?? 0) > 0,
                  isPlaylist: false,
                });
              }
            }
          }
        } catch {
          return fail(400, "validation_failed", "Invalid JSON or M3U import content");
        }
      }

      let matchedItems = 0;
      let unmatchedEntries = 0;
      let favoritesImported = 0;
      let historyImported = 0;
      let playlistTracksImported = 0;
      const samples = candidates.slice(0, 25).map((entry) => {
        const normPath = entry.path?.replace(/\\/g, "/").toLowerCase();
        const fileTail = normPath?.split("/").pop();
        const hit = allLeaves.find(
          (item) =>
            (fileTail && item.path?.toLowerCase().endsWith(fileTail)) ||
            item.title.toLowerCase() === entry.title.toLowerCase(),
        );
        if (hit) {
          matchedItems++;
          if (entry.favorite) favoritesImported++;
          if (entry.history) historyImported++;
          if (entry.isPlaylist) playlistTracksImported++;
          return {
            title: entry.title,
            matched: true,
            matchedTitle: hit.title,
            kind: entry.isPlaylist ? "playlist_track" : entry.favorite ? "favorite" : "history",
          };
        }
        unmatchedEntries++;
        return {
          title: entry.title,
          matched: false,
          matchedTitle: null,
          kind: entry.isPlaylist ? "playlist_track" : "history",
        };
      });

      if (!dryRun) {
        db.recordAudit("system.imported", auth.user, detectedSource, { matchedItems });
      }

      return HttpResponse.json({
        detectedSource,
        dryRun,
        totalEntries: candidates.length,
        matchedItems,
        unmatchedEntries,
        favoritesImported,
        historyImported,
        playlistsImported: playlistTracksImported > 0 ? 1 : 0,
        playlistTracksImported,
        samples,
      });
    }),

    /* ------------------------------------------------------------ libraries */
    http.get(`${BASE}/libraries`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      return HttpResponse.json({ libraries: db.visibleLibraries(auth.user) });
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
      if (!library || !db.canAccessLibrary(auth.user, library.id)) {
        return fail(404, "not_found", "Library not found");
      }
      return HttpResponse.json(library);
    }),

    http.patch(`${BASE}/libraries/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const library = db.getLibrary(String(params.id));
      if (!library) return fail(404, "not_found", "Library not found");
      const body = await readJson<UpdateLibraryRequest>(request);
      const additions = body?.addPaths ?? [];
      if (!body || (!body.name && !additions.length) || !Array.isArray(additions))
        return fail(400, "validation_failed", "Provide a name or folders to add");
      const name = body.name === undefined ? library.name : body.name?.trim();
      if (!name || [...name].length > 100) return fail(400, "validation_failed", "Invalid name", { field: "name" });
      if (db.libraries.some((row) => row.id !== library.id && row.name.toLowerCase() === name.toLowerCase()))
        return fail(409, "conflict", "Name already in use", { field: "name" });
      const paths = [...library.paths];
      for (const [index, raw] of additions.entries()) {
        if (typeof raw !== "string" || !isAbsolute(raw)) return fail(400, "validation_failed", "Expected an absolute path", { field: "paths", index });
        const folder = raw.trim();
        if (/missing|nonexistent/i.test(folder)) return fail(400, "path_not_found", "Folder not found", { field: "paths", index });
        if (paths.includes(folder)) continue;
        if (paths.some((p) => p.startsWith(folder + "/") || folder.startsWith(p + "/")))
          return fail(400, "validation_failed", "Library folders overlap", { field: "paths", index });
        paths.push(folder);
      }
      library.name = name;
      library.paths = paths;
      library.updatedAt = db.iso();
      db.emit("library.updated", library);
      return HttpResponse.json(library);
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

    /* ------------------------------------------------------------ client api keys */
    http.get(`${BASE}/me/api-keys`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      if (!db.features.includes("jellyfin")) return fail(404, "not_found", "Feature not enabled");
      return HttpResponse.json({ keys: clientApiKeys.get(auth.user.id) ?? [] });
    }),

    http.post(`${BASE}/me/api-keys`, async ({ request }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      if (!db.features.includes("jellyfin")) return fail(404, "not_found", "Feature not enabled");
      const body = await readJson<{ name?: string; clientKind?: "jellyfin" | "webdav" | "general" }>(request);
      const name = typeof body?.name === "string" ? body.name.trim() : "";
      if (!name || [...name].length > 128) {
        return fail(400, "validation_failed", "Invalid API key name", { field: "name" });
      }
      const clientKind = body?.clientKind && ["jellyfin", "webdav", "general"].includes(body.clientKind)
        ? body.clientKind
        : "jellyfin";
      const seq = apiKeySeq++;
      const id = stableUuid(`api-key-${auth.user.id}-${seq}`);
      const token = `pvk_${seq.toString(16).padStart(8, "0")}aabbccddeeff00112233445566778899`;
      const key = {
        id,
        name,
        clientKind,
        keyPrefix: token.slice(0, 12),
        lastUsedAt: null,
        revokedAt: null,
        createdAt: new Date(db.now()).toISOString(),
      };
      const existing = clientApiKeys.get(auth.user.id) ?? [];
      clientApiKeys.set(auth.user.id, [key, ...existing]);
      return HttpResponse.json({ key, token }, { status: 201 });
    }),

    http.delete(`${BASE}/me/api-keys/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request);
      if ("response" in auth) return auth.response;
      if (!db.features.includes("jellyfin")) return fail(404, "not_found", "Feature not enabled");
      const targetId = String(params.id);
      const existing = clientApiKeys.get(auth.user.id) ?? [];
      const next = existing.filter((k) => k.id !== targetId);
      if (next.length === existing.length) {
        return fail(404, "not_found", "API key not found");
      }
      clientApiKeys.set(auth.user.id, next);
      return new HttpResponse(null, { status: 204 });
    }),

    http.get(`${BASE}/plugins/catalog`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const installedMap = new Map(db.plugins.map((plugin) => [plugin.id, plugin]));
      const plugins = OFFICIAL_PLUGIN_CATALOG.map((entry) => {
        const current = installedMap.get(entry.id);
        return {
          ...entry,
          installed: Boolean(current),
          installedVersion: current?.version ?? null,
        };
      });
      return HttpResponse.json({ plugins });
    }),

    http.post(`${BASE}/plugins/install`, async ({ request }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const body = (await request.json().catch(() => null)) as {
        pluginId?: string;
        grantedPermissions?: string[];
        settings?: Record<string, unknown>;
      } | null;
      const pluginId = body?.pluginId?.trim() ?? "";
      const blueprint = OFFICIAL_PLUGIN_CATALOG.find((item) => item.id === pluginId);
      if (!blueprint) {
        return fail(404, "not_found", `Plugin ${pluginId} not found in catalog`);
      }
      const granted = new Set((body?.grantedPermissions ?? []).map((p) => p.trim()));
      const missing = blueprint.permissions.filter((p) => !granted.has(p));
      if (missing.length > 0) {
        return fail(400, "validation_failed", `Missing required permissions: ${missing.join(", ")}`, {
          field: "grantedPermissions",
        });
      }
      const mergedSettings: Record<string, unknown> = {
        ...blueprint.defaultSettings,
        ...(body?.settings ?? {}),
      };
      const props = blueprint.settingsSchema.properties ?? {};
      for (const [key, val] of Object.entries(mergedSettings)) {
        const spec = props[key];
        if (!spec) {
          return fail(400, "validation_failed", `Unknown setting ${key}`, { field: "settings" });
        }
        if (spec.enum && !spec.enum.includes(val as string)) {
          return fail(400, "validation_failed", `Invalid value for ${key}`, { field: "settings" });
        }
      }
      const nowIso = new Date(db.now()).toISOString();
      const existingIdx = db.plugins.findIndex((p) => p.id === blueprint.id);
      const prevPlugin = existingIdx >= 0 ? db.plugins[existingIdx] : undefined;
      const previousVersion =
        prevPlugin && prevPlugin.version !== blueprint.version
          ? prevPlugin.version
          : (prevPlugin?.previousVersion ?? blueprint.previousVersion);
      const record = {
        id: blueprint.id,
        name: blueprint.name,
        version: blueprint.version,
        runtime: blueprint.runtime,
        enabled: true,
        provides: [...blueprint.provides],
        previousVersion,
        permissions: [...blueprint.permissions],
        grantedPermissions: [...blueprint.permissions],
        settingsSchema: blueprint.settingsSchema,
        settings: mergedSettings,
        source: blueprint.source,
        riskLevel: blueprint.riskLevel,
        description: blueprint.description,
        managed: true,
        installedAt: prevPlugin?.installedAt ?? nowIso,
        updatedAt: nowIso,
      };
      if (existingIdx >= 0) db.plugins[existingIdx] = record;
      else db.plugins.push(record);
      return HttpResponse.json(record, { status: 201 });
    }),

    http.patch(`${BASE}/plugins/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const id = String(params.id);
      const idx = db.plugins.findIndex((p) => p.id === id);
      if (idx < 0) return fail(404, "not_found", "Installed plugin not found");
      const body = (await request.json().catch(() => null)) as {
        enabled?: boolean;
        settings?: Record<string, unknown>;
      } | null;
      if (!body || (body.enabled === undefined && body.settings === undefined)) {
        return fail(400, "validation_failed", "Provide enabled or settings");
      }
      const current = db.plugins[idx]!;
      const nextSettings = body.settings
        ? { ...(current.settings ?? {}), ...body.settings }
        : current.settings;
      const updated = {
        ...current,
        enabled: body.enabled ?? current.enabled,
        settings: nextSettings,
        updatedAt: new Date(db.now()).toISOString(),
      };
      db.plugins[idx] = updated;
      return HttpResponse.json(updated);
    }),

    http.post(`${BASE}/plugins/:id/rollback`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const id = String(params.id);
      const idx = db.plugins.findIndex((p) => p.id === id);
      if (idx < 0) return fail(404, "not_found", "Installed plugin not found");
      const current = db.plugins[idx]!;
      if (!current.previousVersion) {
        return fail(400, "validation_failed", "No previous version is available to roll back");
      }
      const rolled = {
        ...current,
        version: current.previousVersion,
        previousVersion: null,
        updatedAt: new Date(db.now()).toISOString(),
      };
      db.plugins[idx] = rolled;
      return HttpResponse.json(rolled);
    }),

    http.delete(`${BASE}/plugins/:id`, async ({ request, params }) => {
      await wait();
      const auth = authenticate(request, true);
      if ("response" in auth) return auth.response;
      const id = String(params.id);
      const idx = db.plugins.findIndex((p) => p.id === id);
      if (idx < 0) return fail(404, "not_found", "Installed plugin not found");
      db.plugins.splice(idx, 1);
      return new HttpResponse(null, { status: 204 });
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
            // Provisional events only flow when the matching feature is reported.
            if (event.startsWith("task.") && !db.features.includes("tasks")) return;
            if (event.startsWith("assistant.") && !db.features.includes("assistant")) return;
            if (event.startsWith("home.") && !db.features.includes("home")) return;
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
    ...createPhotosHandlers(db, options),
    ...createReadingHandlers(db, options),
    ...createMusicHandlers(db, options),
    ...createVideoHandlers(db, options),
    ...createSharesCollectionsSearchHandlers(db, options),
    ...createNotificationsAndVirtualTvHandlers(db, options),
    ...createWebdavMetadataObservabilityHandlers(db, options),
    ...createSyncCastMqttHandlers(db, options),
  ];
}
