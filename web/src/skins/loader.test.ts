import { afterEach, describe, expect, it, vi } from "vitest";
import workcolor from "./builtin/workcolor";
import { fetchInstalledSkin, installedManifestUrl, MAX_MANIFEST_BYTES, SkinLoadError } from "./loader";
import { BUILTIN_SKIN, DEFAULT_SKIN_ID } from "./registry";
import { getSkinState, initSkin, resetSkinStore, setNav, setSkin, setSkinLoader } from "./store";

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
    expect(fetchImpl).toHaveBeenCalledWith(installedManifestUrl("paper"), expect.objectContaining({ credentials: "omit", mode: "same-origin", redirect: "error" }));
    const init = fetchImpl.mock.calls[0]?.[1];
    expect(new Headers(init?.headers).has("Authorization")).toBe(false);
    expect(entry.source).toBe("installed");
    expect(await entry.loadFonts()).toEqual([expect.objectContaining({ src: "/api/v1/skins/installed/paper/assets/fonts/paper.woff2" })]);
  });

  it.each([
    [undefined, "upload"],
    ["upload", "upload"],
    ["unrecognized", "upload"],
    ["official", "official"],
  ])("reads package provenance from the host response (%s)", async (source, expected) => {
    const headers = new Headers();
    if (source) headers.set("X-Palveni-Skin-Source", source);
    const skin = { ...packageManifest("native"), author: "Palveni" };
    const entry = await fetchInstalledSkin("native", { fetchImpl: respond(skin, { headers }) });
    expect(entry.packageSource).toBe(expected);
  });

  it("keeps identically named faces private to each installed skin, including previewed skins", async () => {
    const fromPackage = async (id: string) => {
      const skin = {
        ...packageManifest(id),
        type: { ...BUILTIN_SKIN.manifest.type, heading: ["figtree", "serif"], ui: ["FIGTREE", "sans-serif"], mono: ["monospace"] },
        fonts: [
          { family: "Figtree", src: "fonts/regular.woff2", weight: "400", style: "normal" },
          { family: "FIGTREE", src: "fonts/bold.woff2", weight: "700", style: "normal" },
        ],
      };
      return fetchInstalledSkin(id, { fetchImpl: respond(skin) });
    };
    const first = await fromPackage("type.paper");
    const second = await fromPackage("type-paper");
    const family = (await first.loadFonts())[0]?.family;
    expect(family).toBeTruthy();
    expect(family?.toLowerCase()).not.toBe("figtree");
    expect((await first.loadFonts())[1]?.family).toBe(family);
    expect((await second.loadFonts())[0]?.family).not.toBe(family);
    expect(first.manifest.type.heading).toEqual([family, "figtree", "serif"]);
    expect(first.manifest.type.ui).toEqual([family, "FIGTREE", "sans-serif"]);
    expect(first.manifest.type.mono).toEqual(["monospace"]);
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
  it("restores the navigation preference across restarts and preserves the skin's mobile navigation", async () => {
    const skin = {
      ...packageManifest(),
      slots: { ...BUILTIN_SKIN.manifest.slots, nav: "topbar", navPhone: "top-scroll", pageTitle: "content" },
    };
    setNav("sidebar-right");
    setSkinLoader((id) => fetchInstalledSkin(id, { fetchImpl: respond(skin) }));
    await setSkin("paper");
    const root = document.documentElement;
    expect(root.dataset).toMatchObject({ navSide: "right", slotNav: "sidebar", slotNavPhone: "top-scroll", slotPageTitle: "toolbar" });
    resetSkinStore();
    setSkinLoader((id) => fetchInstalledSkin(id, { fetchImpl: respond(skin) }));
    await initSkin();
    expect(root.dataset).toMatchObject({ skin: "paper", navSide: "right", slotNav: "sidebar" });
    setNav("skin");
    expect(root.dataset).toMatchObject({ navSide: "left", slotNav: "topbar", slotNavPhone: "top-scroll", slotPageTitle: "content" });
    expect(window.localStorage.getItem("palveni.nav")).toBeNull();
  });

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
