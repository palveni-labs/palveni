import { Button, buttonVariants, Chip, Input } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { FileCode2, Puzzle, ShieldAlert, ShieldCheck, type LucideIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { usePlugins } from "@/api/queries/plugins";
import { useInstalledSkins } from "@/api/queries/skins";
import type { InstalledSkin, Plugin } from "@/api/types";
import { currentLanguage } from "@/i18n";
import { useFeature } from "@/modules/access";
import { localized } from "@/skins/schema";
import { OfficialSkinCatalog, UploadSkinButton } from "./themes-page";
import { PageFrame } from "@/ui/frame";
import { StatusLabel } from "@/ui/marks";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

const RUNTIME_ICON: Record<Plugin["runtime"], LucideIcon> = {
  none: ShieldCheck,
  recipe: FileCode2,
  wasm: ShieldCheck,
  sidecar: ShieldAlert,
};

/** Extension points with a name of their own; anything else is shown as its identifier. */
const KNOWN_POINTS = new Set([
  "storage",
  "content",
  "metadata",
  "lyrics",
  "subtitles",
  "danmaku",
  "sync",
  "notify",
  "downloader",
  "indexer",
  "auth",
  "ai",
  "hooks",
  "schedule",
  "importer",
  "http",
  "tool",
  "home",
  "skin",
  "widget",
]);

function PluginRow({ plugin, skin }: { plugin: Plugin; skin?: InstalledSkin }) {
  const { t } = useTranslation();
  const inset = useRowInset();
  const RuntimeIcon = RUNTIME_ICON[plugin.runtime];
  return (
    <li className={cn("grid gap-x-6 gap-y-2 py-4 md:grid-cols-[minmax(0,24rem)_minmax(0,1fr)_auto] md:items-start", inset)}>
      <div className="flex min-w-0 items-start gap-3">
        <span className="grid size-9 shrink-0 place-items-center rounded-tile bg-sunken text-icon">
          <Puzzle className="size-[1.125rem]" strokeWidth={1.8} aria-hidden />
        </span>
        <div className="grid min-w-0 gap-0.5">
          <h3 className="flex flex-wrap items-baseline gap-x-2 text-md font-strong text-fg">
            <span className="min-w-0 truncate">{skin ? localized(skin.name, currentLanguage()) : plugin.name}</span>
            <span className="text-sm font-body text-fg-muted tabular-nums">{t("plugins.version", { version: plugin.version })}</span>
          </h3>
          {skin ? <p className="text-sm text-fg-muted">{t(skin.source === "upload" ? "appearance.unverified" : "plugins.official")}</p> : null}
          <p className="truncate font-mono text-sm text-fg-muted" title={plugin.id}>
            {plugin.id}
          </p>
        </div>
      </div>
      <div className="grid min-w-0 gap-2 max-md:ps-12">
        <p className="flex items-start gap-1.5 text-sm text-fg">
          <RuntimeIcon className={cn("mt-0.5 size-4 shrink-0", plugin.runtime === "sidecar" ? "text-degraded" : "text-icon")} aria-hidden />
          <span>
            <span className="font-mid">{t(`plugins.runtimes.${plugin.runtime}`)}</span>
            <span className="text-fg-muted"> · {t(`plugins.isolation.${plugin.runtime}`)}</span>
          </span>
        </p>
        {plugin.provides.length > 0 ? (
          <ul aria-label={t("plugins.provides")} className="flex flex-wrap gap-1.5">
            {plugin.provides.map((point) => (
              <li key={point}>
                <Chip size="sm" variant="soft" className="bg-sunken text-fg">
                  {KNOWN_POINTS.has(point) ? t(`plugins.points.${point}` as "plugins.points.storage") : <span className="font-mono">{point}</span>}
                </Chip>
              </li>
            ))}
          </ul>
        ) : null}
      </div>
      <div className="grid justify-items-start gap-2 max-md:ps-12">
        <StatusLabel tone={plugin.enabled ? "ok" : "idle"} className="text-sm">
          {plugin.enabled ? t("plugins.enabled") : t("plugins.disabled")}
        </StatusLabel>
        {skin && <Link to="/apps/themes" className={buttonVariants({ variant: "outline", size: "sm" })}>{t("plugins.manageSkin")}</Link>}
      </div>
    </li>
  );
}

function LoadingPlugins() {
  const inset = useRowInset();
  return <LoadingRows rows={3} className={inset} />;
}

export function PluginsPage() {
  const { t } = useTranslation();
  const query = usePlugins();
  const skinsEnabled = useFeature("skins");
  const installed = useInstalledSkins(skinsEnabled);
  const skins = new Map((installed.data?.skins ?? []).map((skin) => [skin.pluginId, skin]));
  const [input, setInput] = useState("");
  const [filter, setFilter] = useState<"all" | "available" | "installed">("all");
  const search = input.trim().toLocaleLowerCase();
  const installedPlugins = new Map((query.data?.plugins ?? []).map((plugin) => [plugin.id, plugin]));
  for (const skin of skins.values()) {
    if (!installedPlugins.has(skin.pluginId)) {
      installedPlugins.set(skin.pluginId, {
        id: skin.pluginId, name: localized(skin.name, currentLanguage()), version: skin.version,
        runtime: "none", provides: ["skin"], enabled: true,
      });
    }
  }
  const plugins = Array.from(installedPlugins.values());
  const matches = plugins.filter((plugin) => {
    const skin = skins.get(plugin.id);
    const fields = [plugin.name, plugin.id, ...plugin.provides, ...(skin ? Object.values(skin.name) : [])];
    for (const point of plugin.provides) {
      if (KNOWN_POINTS.has(point)) fields.push(t(`plugins.points.${point}` as "plugins.points.storage"));
    }
    return fields.some((field) => field.toLocaleLowerCase().includes(search));
  });
  const enabled = plugins.filter((plugin) => plugin.enabled).length;
  return (
    <PageFrame>
      <PageHeader
        title={t("plugins.storeTitle")}
        actions={skinsEnabled ? <UploadSkinButton /> : undefined}
        summary={query.data && plugins.length > 0 ? t("plugins.summary", { count: plugins.length, enabled }) : undefined}
      />
      <div className="flex flex-wrap items-center gap-3">
        <Input type="search" aria-label={t("plugins.search")} placeholder={t("plugins.search")} className="w-full sm:max-w-sm" value={input} onChange={(event) => setInput(event.target.value)} />
        <div role="group" aria-label={t("plugins.filterLabel")} className="flex flex-wrap gap-1">
          {(["all", "available", "installed"] as const).map((value) => (
            <Button key={value} size="sm" variant={filter === value ? "primary" : "outline"} aria-pressed={filter === value} onPress={() => setFilter(value)}>
              {t(`plugins.filter.${value}`)}
            </Button>
          ))}
        </div>
        <a className="text-sm font-mid text-link underline" href="https://github.com/palveni-labs/plugins#submit-a-plugin" target="_blank" rel="noreferrer">
          {t("plugins.contribute")}
        </a>
      </div>
      {filter !== "installed" && <OfficialSkinCatalog enabled={skinsEnabled} search={search} store />}
      {filter !== "available" && <Section title={t("plugins.installedTitle")} count={query.data ? matches.length : undefined}>
        {query.isPending ? (
          <LoadingPlugins />
        ) : query.isError ? (
          <ErrorState compact error={query.error} onRetry={() => void query.refetch()} />
        ) : matches.length === 0 && search ? (
          <EmptyState compact icon={Puzzle} title={t("plugins.noMatches")} />
        ) : plugins.length === 0 ? (
          <EmptyState
            compact
            icon={Puzzle}
            title={t("plugins.empty")}
            detail={t("plugins.emptyDetail")}
            action={
              <Link to="/apps/themes" className={buttonVariants({ variant: "outline", size: "sm" })}>
                {t("plugins.toAppearance")}
              </Link>
            }
          />
        ) : (
          <SectionRows>
            {matches.map((plugin) => (
              <PluginRow key={plugin.id} plugin={plugin} skin={skins.get(plugin.id)} />
            ))}
          </SectionRows>
        )}
      </Section>}
    </PageFrame>
  );
}
