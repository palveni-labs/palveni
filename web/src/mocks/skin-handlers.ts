import { http, HttpResponse } from "msw";
import type { MockDb } from "./db";
import { SkinRegistryState } from "./skins";

const BASE = "/api/v1";

async function serveFile(url: string | undefined, contentType: string) {
  if (!url) return fail(404, "not_found", "Not found");
  const response = await fetch(url);
  if (!response.ok) return fail(404, "not_found", "Not found");
  return new HttpResponse(await response.arrayBuffer(), { headers: { "Content-Type": contentType, "Cache-Control": "public, max-age=3600" } });
}

function fail(status: number, code: string, message: string) {
  return HttpResponse.json({ error: { code, message } }, { status });
}

export function createSkinHandlers(db: MockDb) {
  const skins = new SkinRegistryState(() => new Date().toISOString());
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
  const publicSkin = (id: string) => (db.features.includes("skins") && skins.isInstalled(id) ? null : fail(404, "not_found", "Not found"));

  return [
    http.get(`${BASE}/skins/installed`, ({ request }) => {
      return guard(request, "skins") ?? HttpResponse.json({ skins: skins.listInstalled() });
    }),
    http.get(`${BASE}/skins/registry`, ({ request }) => {
      return adminGuard(request, "skins") ?? HttpResponse.json({ skins: skins.listRegistry() });
    }),
    http.post(`${BASE}/skins/registry/:id/install`, ({ request, params }) => {
      const denied = adminGuard(request, "skins");
      if (denied) return denied;
      // A real server downloads, verifies and unpacks the package here.
      const installed = skins.install(String(params.id));
      return installed ? HttpResponse.json(installed) : fail(404, "not_found", "Skin not found");
    }),
    http.delete(`${BASE}/skins/installed/:id`, ({ request, params }) => {
      const denied = adminGuard(request, "skins");
      if (denied) return denied;
      return skins.uninstall(String(params.id)) ? new HttpResponse(null, { status: 204 }) : fail(404, "not_found", "Skin not installed");
    }),
    http.get(`${BASE}/skins/installed/:id/skin.json`, ({ params }) => {
      const id = String(params.id);
      return publicSkin(id) ?? HttpResponse.json(skins.find(id)?.manifest as Record<string, unknown>);
    }),
    http.get(`${BASE}/skins/installed/:id/assets/fonts/:file`, ({ params, request }) => {
      const id = String(params.id);
      const denied = publicSkin(id);
      if (denied) return denied;
      const url = skins.assetUrl(id, `fonts/${String(params.file)}`);
      if (!url || url.startsWith("data:")) return serveFile(url, "font/woff2");
      return HttpResponse.redirect(new URL(url, request.url).href, 307);
    }),
    http.get(`${BASE}/skins/installed/:id/assets/:file`, ({ params }) => {
      const id = String(params.id);
      return publicSkin(id) ?? serveFile(skins.assetUrl(id, String(params.file)), "image/png");
    }),
    http.get(`${BASE}/skins/registry/:id/preview`, ({ params }) => {
      if (!db.features.includes("skins")) return fail(404, "not_found", "Not found");
      return serveFile(skins.find(String(params.id))?.files.get("preview.png"), "image/png");
    }),
  ];
}
