/**
 * 应用 › 插件 (M0 API, admin): the plugins installed on the server, with how each one runs and
 * which extension points it provides.
 */
import { buttonVariants, Chip } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { FileCode2, Puzzle, ShieldAlert, ShieldCheck, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { usePlugins } from "@/api/queries/plugins";
import type { Plugin } from "@/api/types";
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

function PluginRow({ plugin }: { plugin: Plugin }) {
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
            <span className="min-w-0 truncate">{plugin.name}</span>
            <span className="text-sm font-body text-fg-muted tabular-nums">{t("plugins.version", { version: plugin.version })}</span>
          </h3>
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
      <div className="max-md:ps-12">
        <StatusLabel tone={plugin.enabled ? "ok" : "idle"} className="text-sm">
          {plugin.enabled ? t("plugins.enabled") : t("plugins.disabled")}
        </StatusLabel>
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
  const plugins = query.data?.plugins ?? [];
  const enabled = plugins.filter((plugin) => plugin.enabled).length;
  return (
    <PageFrame>
      <PageHeader
        title={t("nav.plugins")}
        summary={query.data && plugins.length > 0 ? t("plugins.summary", { count: plugins.length, enabled }) : undefined}
      />
      <Section title={t("plugins.installedTitle")} count={query.data ? plugins.length : undefined}>
        {query.isPending ? (
          <LoadingPlugins />
        ) : query.isError ? (
          <ErrorState compact error={query.error} onRetry={() => void query.refetch()} />
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
            {plugins.map((plugin) => (
              <PluginRow key={plugin.id} plugin={plugin} />
            ))}
          </SectionRows>
        )}
      </Section>
    </PageFrame>
  );
}
