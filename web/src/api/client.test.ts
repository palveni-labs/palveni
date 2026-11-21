import { http, HttpResponse } from "msw";
import { describe, expect, it, vi } from "vitest";
import { server } from "@/test/msw-server";
import { request, setUnauthorizedHandler } from "./client";
import { api } from "./endpoints";
import { ApiError } from "./errors";
import { getToken, setSession } from "./session";

describe("api client", () => {
  it("sends the bearer token and parses JSON", async () => {
    setSession({ accessToken: "token-1", expiresAt: null });
    let seen: string | null = null;
    server.use(
      http.get("/api/v1/auth/me", ({ request: req }) => {
        seen = req.headers.get("Authorization");
        return HttpResponse.json({ id: "u1", username: "admin", role: "admin", createdAt: "2026-11-20T08:30:00Z" });
      }),
    );
    await expect(api.auth.me()).resolves.toMatchObject({ username: "admin" });
    expect(seen).toBe("Bearer token-1");
  });

  it("maps the contract error body onto ApiError (status, code, message)", async () => {
    server.use(
      http.get("/api/v1/libraries/missing", () =>
        HttpResponse.json({ error: { code: "not_found", message: "Library not found" } }, { status: 404 }),
      ),
    );
    const error = await api.libraries.get("missing").catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({ status: 404, code: "not_found", message: "Library not found" });
  });

  it("keeps the error's details when the server gives them, and none otherwise", async () => {
    server.use(
      http.post("/api/v1/libraries", () =>
        HttpResponse.json(
          { error: { code: "path_not_found", message: "Path does not exist: /m", details: { field: "paths", index: 1, path: "/m", extra: { no: 1 } } } },
          { status: 400 },
        ),
      ),
    );
    const error = await api.libraries.create({ name: "x", kind: "video", paths: ["/a", "/m"] }).catch((caught: unknown) => caught);
    expect(error).toMatchObject({ code: "path_not_found", details: { field: "paths", index: 1, path: "/m" } });
    expect((error as ApiError).details).not.toHaveProperty("extra");
    server.use(http.get("/api/v1/libraries/gone", () => HttpResponse.json({ error: { code: "not_found", message: "gone" } }, { status: 404 })));
    expect(await api.libraries.get("gone").catch((caught: unknown) => caught)).toMatchObject({ details: null });
  });

  it("falls back to a status-based code when the body is not the error shape", async () => {
    server.use(http.get("/api/v1/plugins", () => new HttpResponse("<html>Bad Gateway</html>", { status: 502, statusText: "Bad Gateway" })));
    await expect(api.plugins.list()).rejects.toMatchObject({ status: 502, code: "internal", message: "Bad Gateway" });
  });

  it("reports transport failures as network_error with status 0", async () => {
    server.use(http.get("/api/v1/system/health", () => HttpResponse.error()));
    await expect(api.system.health()).rejects.toMatchObject({ status: 0, code: "network_error" });
  });

  it("returns undefined for 204 responses", async () => {
    setSession({ accessToken: "token-1", expiresAt: null });
    server.use(http.post("/api/v1/auth/logout", () => new HttpResponse(null, { status: 204 })));
    await expect(api.auth.logout()).resolves.toBeUndefined();
  });

  it("clears the token and notifies the app when a signed-in request gets 401", async () => {
    const onUnauthorized = vi.fn();
    setUnauthorizedHandler(onUnauthorized);
    setSession({ accessToken: "stale", expiresAt: null });
    server.use(
      http.get("/api/v1/libraries", () =>
        HttpResponse.json({ error: { code: "unauthorized", message: "token revoked" } }, { status: 401 }),
      ),
    );
    await expect(api.libraries.list()).rejects.toMatchObject({ status: 401, code: "unauthorized" });
    expect(getToken()).toBeNull();
    expect(onUnauthorized).toHaveBeenCalledTimes(1);
  });

  it("does not treat failed credentials as a lost session", async () => {
    const onUnauthorized = vi.fn();
    setUnauthorizedHandler(onUnauthorized);
    setSession({ accessToken: "still-valid", expiresAt: null });
    let authorization: string | null = "unset";
    server.use(
      http.post("/api/v1/auth/login", ({ request: req }) => {
        authorization = req.headers.get("Authorization");
        return HttpResponse.json({ error: { code: "invalid_credentials", message: "nope" } }, { status: 401 });
      }),
    );
    await expect(api.auth.login({ username: "a", password: "b" })).rejects.toMatchObject({ code: "invalid_credentials" });
    expect(authorization).toBeNull();
    expect(getToken()).toBe("still-valid");
    expect(onUnauthorized).not.toHaveBeenCalled();
  });

  it("treats an expired session as signed out", () => {
    setSession({ accessToken: "old", expiresAt: "2020-01-01T00:00:00Z" });
    expect(getToken()).toBeNull();
  });

  it("encodes query parameters and skips empty ones", async () => {
    setSession({ accessToken: "t", expiresAt: null });
    let url = "";
    server.use(
      http.get("/api/v1/items", ({ request: req }) => {
        url = req.url;
        return HttpResponse.json({ items: [], nextCursor: null });
      }),
    );
    await request("/items", { query: { libraryId: "lib 1", parentId: null, cursor: undefined, limit: 100 } });
    const search = new URL(url).searchParams;
    expect(search.get("libraryId")).toBe("lib 1");
    expect(search.get("limit")).toBe("100");
    expect(search.has("parentId")).toBe(false);
    expect(search.has("cursor")).toBe(false);
  });
});
