import { afterEach, describe, expect, it, vi } from "vitest";
import workcolor from "./builtin/workcolor";
import { fetchInstalledSkin, installedManifestUrl, MAX_MANIFEST_BYTES, SkinLoadError } from "./loader";
import { DEFAULT_SKIN_ID } from "./registry";
import { getSkinState, initSkin, resetSkinStore, setSkin, setSkinLoader } from "./store";

function packageManifest(id = "paper"): Record<string, unknown> & { colors: { light: Record<string, string> } } {
  const base = structuredClone(workcolor.manifest) as { colors: { light: Record<string, string> } };
  return { ...base, id, name: { en: "Paper" }, fonts: [{ family: "Paper Serif", src: "fonts/paper.woff2", weight: "400", style: "normal" }] };
}

function respond(body: unknown, init: ResponseInit = {}) {
  return vi.fn((_input: RequestInfo | URL, _init?: RequestInit) =>
    Promise.resolve(new Response(typeof body === "string" ? body : JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" }, ...init })),
  );
}

afterEach(() => {
  resetSkinStore();
  window.localStorage.clear();
});

describe("fetchInstalledSkin", () => {
  it("fetches the public manifest without credentials and maps font files to the package's asset URLs", async () => {
    const fetchImpl = respond(packageManifest());
    const entry = await fetchInstalledSkin("paper", { fetchImpl });
    expect(fetchImpl).toHaveBeenCalledWith(installedManifestUrl("paper"), expect.objectContaining({ credentials: "omit" }));
    const init = fetchImpl.mock.calls[0]?.[1];
    expect(new Headers(init?.headers).has("Authorization")).toBe(false);
    expect(entry.source).toBe("installed");
    expect(await entry.loadFonts()).toEqual([expect.objectContaining({ src: "/api/v1/skins/installed/paper/assets/fonts/paper.woff2" })]);
  });

  it("says why a skin cannot be loaded", async () => {
    await expect(fetchInstalledSkin("paper", { fetchImpl: respond("", { status: 404 }) })).rejects.toMatchObject({ reason: "missing" });
    await expect(fetchInstalledSkin("paper", { fetchImpl: vi.fn(() => Promise.reject(new TypeError("offline"))) })).rejects.toMatchObject({ reason: "unreachable" });
    await expect(fetchInstalledSkin("paper", { fetchImpl: respond("{not json") })).rejects.toMatchObject({ reason: "invalid" });
    await expect(fetchInstalledSkin("paper", { fetchImpl: respond("x".repeat(MAX_MANIFEST_BYTES + 1)) })).rejects.toMatchObject({ reason: "invalid" });
    await expect(fetchInstalledSkin("other", { fetchImpl: respond(packageManifest("paper")) })).rejects.toMatchObject({ reason: "invalid" });
    await expect(fetchInstalledSkin("../../etc", { fetchImpl: respond(packageManifest()) })).rejects.toMatchObject({ reason: "invalid" });
  });

  it("refuses a package that breaks a rule, as a whole", async () => {
    const bad = packageManifest();
    bad.colors.light.fg = "oklch(0.9 0 0)";
    const error = await fetchInstalledSkin("paper", { fetchImpl: respond(bad) }).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(SkinLoadError);
    expect((error as SkinLoadError).reason).toBe("invalid");
    expect((error as SkinLoadError).message).toMatch(/fg on bg/);
  });
});

describe("the skin store", () => {
  it("applies an installed skin through CSSOM, with no stylesheet text", async () => {
    setSkinLoader((id) => fetchInstalledSkin(id, { fetchImpl: respond(packageManifest(id)) }));
    await setSkin("paper");
    expect(getSkinState()).toMatchObject({ skinId: "paper", status: "ready", failure: null });
    expect(getSkinState().active.manifest.id).toBe("paper");
    expect(document.documentElement.dataset.skin).toBe("paper");
    expect(document.documentElement.style.getPropertyValue("--pv-bg")).not.toBe("");
    expect(document.querySelectorAll("style").length).toBe(0);
    expect(window.localStorage.getItem("palveni.skin")).toBe("paper");
  });

  it("falls back to the built-in skin when the chosen one is gone, and forgets it", async () => {
    setSkinLoader(() => Promise.reject(new SkinLoadError("missing", "uninstalled")));
    await expect(setSkin("paper")).rejects.toBeInstanceOf(SkinLoadError);
    expect(getSkinState()).toMatchObject({ skinId: DEFAULT_SKIN_ID, status: "fallback", failure: "missing" });
    expect(getSkinState().active.manifest.id).toBe(DEFAULT_SKIN_ID);
    expect(window.localStorage.getItem("palveni.skin")).toBeNull();
  });

  it("falls back while the server is unreachable but keeps the choice for next time", async () => {
    window.localStorage.setItem("palveni.skin", "paper");
    setSkinLoader(() => Promise.reject(new SkinLoadError("unreachable", "offline")));
    await initSkin();
    expect(getSkinState()).toMatchObject({ status: "fallback", failure: "unreachable" });
    expect(getSkinState().active.manifest.id).toBe(DEFAULT_SKIN_ID);
    expect(window.localStorage.getItem("palveni.skin")).toBe("paper");
  });

  it("keeps light/dark independent of the skin", async () => {
    window.localStorage.setItem("palveni.theme", "dark");
    await initSkin();
    setSkinLoader((id) => fetchInstalledSkin(id, { fetchImpl: respond(packageManifest(id)) }));
    await setSkin("paper");
    expect(getSkinState()).toMatchObject({ mode: "dark", resolved: "dark" });
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
