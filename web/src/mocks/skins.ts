import type { InstalledSkin, RegistrySkin } from "@/api/types";

const manifests = import.meta.glob<unknown>("../../../plugins/skin-*/skin.json", { eager: true, import: "default" });
const tomls = import.meta.glob<string>("../../../plugins/skin-*/plugin.toml", { eager: true, import: "default", query: "?raw" });
const fonts = import.meta.glob<string>("../../../plugins/skin-*/fonts/*.woff2", { eager: true, import: "default", query: "?url" });
const previews = import.meta.glob<string>("../../../plugins/skin-*/preview.png", { eager: true, import: "default", query: "?url" });

interface Package {
  info: Omit<RegistrySkin, "installed">;
  manifest: unknown;
  /** Package-relative path → URL the dev server serves it from. */
  files: Map<string, string>;
}

function field(toml: string, key: string): string {
  return new RegExp(`^${key}\\s*=\\s*"([^"]*)"`, "m").exec(toml)?.[1] ?? "";
}

function loadPackages(): Package[] {
  const packages: Package[] = [];
  for (const [path, manifest] of Object.entries(manifests)) {
    const dir = path.slice(0, -"/skin.json".length);
    const record = manifest as { id?: string; name?: RegistrySkin["name"]; description?: RegistrySkin["description"]; fonts?: Array<{ bytes?: number }> };
    const id = record.id ?? dir.split("/").pop()?.replace(/^skin-/, "") ?? "";
    const toml = tomls[`${dir}/plugin.toml`] ?? "";
    const files = new Map<string, string>();
    for (const [file, url] of Object.entries(fonts)) if (file.startsWith(`${dir}/fonts/`)) files.set(`fonts/${file.split("/").pop()}`, url);
    const preview = previews[`${dir}/preview.png`];
    if (preview) files.set("preview.png", preview);
    const fontBytes = (record.fonts ?? []).reduce((sum, font) => sum + (font.bytes ?? 0), 0);
    packages.push({
      manifest,
      files,
      info: {
        id,
        pluginId: field(toml, "id"),
        name: record.name ?? { en: id },
        description: record.description ?? { en: "" },
        version: field(toml, "version") || "1.0.0",
        sizeBytes: fontBytes + JSON.stringify(manifest).length + (preview ? 180_000 : 0),
        previewUrl: preview ? `/api/v1/skins/registry/${id}/preview` : null,
        downloadUrl: "",
        sha256: "",
        publishedAt: "2026-11-20T00:00:00Z",
      },
    });
  }
  return packages.sort((a, b) => ["native", "ink", "gear"].indexOf(a.info.id) - ["native", "ink", "gear"].indexOf(b.info.id));
}

export class SkinRegistryState {
  readonly packages = loadPackages();
  private installed = new Map<string, string>();
  private readonly now: () => string;
  private readonly persist?: (ids: string[]) => void;

  constructor(now: () => string, initiallyInstalled: readonly string[] = [], persist?: (ids: string[]) => void) {
    this.now = now;
    this.persist = persist;
    for (const id of initiallyInstalled) if (this.find(id)) this.installed.set(id, now());
  }

  find(id: string): Package | undefined {
    return this.packages.find((entry) => entry.info.id === id);
  }

  isInstalled(id: string): boolean {
    return this.installed.has(id);
  }

  listInstalled(): InstalledSkin[] {
    return this.packages.flatMap((entry) => {
      const at = this.installed.get(entry.info.id);
      if (!at) return [];
      const { publishedAt: _published, downloadUrl: _download, sha256: _digest, ...info } = entry.info;
      return [{ ...info, previewUrl: entry.files.has("preview.png") ? `/api/v1/skins/installed/${entry.info.id}/assets/preview.png` : null, source: "official" as const, installedAt: at }];
    });
  }

  listRegistry(): RegistrySkin[] {
    return this.packages.map((entry) => ({ ...entry.info, installed: this.installed.has(entry.info.id) }));
  }

  install(id: string): InstalledSkin | undefined {
    if (!this.find(id)) return undefined;
    if (!this.installed.has(id)) this.installed.set(id, this.now());
    this.persist?.([...this.installed.keys()]);
    return this.listInstalled().find((entry) => entry.id === id);
  }

  uninstall(id: string): boolean {
    const removed = this.installed.delete(id);
    this.persist?.([...this.installed.keys()]);
    return removed;
  }

  /** The URL a package file is served from, if the path is one a skin may ship. */
  assetUrl(id: string, path: string): string | undefined {
    if (!/^(fonts\/[A-Za-z0-9][A-Za-z0-9._-]*\.woff2|preview\.png)$/.test(path)) return undefined;
    return this.find(id)?.files.get(path);
  }
}
