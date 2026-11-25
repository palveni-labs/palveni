import { Button, buttonVariants, Chip, Input, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import {
  CheckCircle2,
  Database,
  Download,
  FileCode2,
  History,
  LayoutGrid,
  Palette,
  Puzzle,
  Settings2,
  ShieldAlert,
  ShieldCheck,
  Subtitles,
  Trash2,
  X,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useInstallPlugin,
  usePluginCatalog,
  usePlugins,
  useRollbackPlugin,
  useUninstallPlugin,
  useUpdatePlugin,
} from "@/api/queries/plugins";
import { useInstalledSkins, useSkinRegistry } from "@/api/queries/skins";
import type {
  InstalledSkin,
  Plugin,
  PluginCatalogEntry,
  PluginRiskLevel,
  PluginRuntimeKind,
} from "@/api/types";
import { currentLanguage } from "@/i18n";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { localized } from "@/skins/schema";
import { PageFrame } from "@/ui/frame";
import { StatusLabel } from "@/ui/marks";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { OfficialSkinCatalog, UploadSkinButton } from "./themes-page";

/** How isolated each runtime is, in words (docs/plugins.md §2). */
const RUNTIME_ICON: Record<PluginRuntimeKind, LucideIcon> = {
  none: ShieldCheck,
  recipe: FileCode2,
  widget: LayoutGrid,
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

type PluginCategory = "all" | "metadata" | "mediaEnhance" | "widget" | "skin" | "other";

const CATEGORY_ICON: Record<Exclude<PluginCategory, "all">, LucideIcon> = {
  metadata: Database,
  mediaEnhance: Subtitles,
  widget: LayoutGrid,
  skin: Palette,
  other: Puzzle,
};

function classifyPluginCategory(
  provides: readonly string[],
  runtime?: PluginRuntimeKind,
): Exclude<PluginCategory, "all"> {
  if (provides.includes("skin")) return "skin";
  if (provides.includes("widget") || runtime === "widget") return "widget";
  if (provides.some((point) => point === "metadata" || point === "content")) return "metadata";
  if (provides.some((point) => point === "lyrics" || point === "subtitles" || point === "danmaku")) {
    return "mediaEnhance";
  }
  return "other";
}

function RiskBadge({ level }: { level: PluginRiskLevel }) {
  const { t } = useTranslation();
  const styles: Record<PluginRiskLevel, string> = {
    low: "border-emerald-600/25 bg-emerald-500/12 text-emerald-800 dark:text-emerald-300",
    medium: "border-amber-600/30 bg-amber-500/14 text-amber-900 dark:text-amber-300",
    high: "border-red-600/30 bg-red-500/14 text-red-800 dark:text-red-300",
  };
  return (
    <span
      data-risk={level}
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-full border px-2.5 py-1 text-xs leading-none font-strong whitespace-nowrap",
        styles[level],
      )}
    >
      {level === "low" ? (
        <ShieldCheck className="size-3.5 shrink-0" aria-hidden />
      ) : (
        <ShieldAlert className="size-3.5 shrink-0" aria-hidden />
      )}
      {t(`plugins.risk.${level}`)}
    </span>
  );
}

function localizedDescription(map: Record<string, string> | null | undefined): string | null {
  if (!map) return null;
  const lang = currentLanguage();
  return map[lang] ?? map["zh-CN"] ?? map.en ?? Object.values(map)[0] ?? null;
}

function stringifySetting(val: unknown): string {
  if (typeof val === "string") return val;
  if (typeof val === "number" || typeof val === "boolean") return String(val);
  return "";
}

function PluginInstallReviewModal({
  entry,
  onClose,
}: {
  entry: PluginCatalogEntry;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const install = useInstallPlugin();
  const [checked, setChecked] = useState<Set<string>>(() => new Set());

  const requiredPerms = entry.permissions;
  const allGranted = requiredPerms.every((perm) => checked.has(perm));
  const desc = localizedDescription(entry.description);

  const togglePermission = (perm: string) => {
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(perm)) next.delete(perm);
      else next.add(perm);
      return next;
    });
  };

  const handleConfirm = () => {
    if (!allGranted) return;
    install.mutate(
      {
        pluginId: entry.id,
        grantedPermissions: [...checked],
        settings: entry.defaultSettings,
      },
      {
        onSuccess: () => {
          toast.success(t("plugins.toast.installed", { name: entry.name }));
          onClose();
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("plugins.review.title", { name: entry.name })}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
    >
      <div className="grid w-full max-w-lg gap-4 rounded-panel border border-line-strong bg-raised p-5 text-fg shadow-panel">
        <div className="flex items-start justify-between gap-3">
          <div className="grid min-w-0 gap-1">
            <h2 className="font-heading text-lg font-title text-heading">
              {t("plugins.review.title", { name: entry.name })}
            </h2>
            <p className="truncate font-mono text-xs text-fg-muted">
              {entry.id} · v{entry.version}
            </p>
          </div>
          <button
            type="button"
            aria-label={t("plugins.review.cancel")}
            onClick={onClose}
            className="grid size-8 shrink-0 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>

        {desc ? <p className="text-sm text-fg-muted">{desc}</p> : null}

        <div className="flex items-center justify-between rounded-tile border border-line bg-sunken px-3.5 py-2.5">
          <span className="text-sm font-mid text-fg">{t("plugins.review.riskHeading")}</span>
          <RiskBadge level={entry.riskLevel} />
        </div>

        <div className="grid gap-2">
          <p className="text-sm font-strong text-fg">{t("plugins.review.permissionsHeading")}</p>
          {requiredPerms.length === 0 ? (
            <p className="rounded-tile border border-line bg-surface px-3 py-2.5 text-sm text-fg-muted">
              {t("plugins.review.zeroPermissions")}
            </p>
          ) : (
            <ul className="grid gap-2">
              {requiredPerms.map((perm) => {
                const isChecked = checked.has(perm);
                return (
                  <li key={perm}>
                    <label className="flex cursor-pointer items-center gap-2.5 rounded-tile border border-line bg-surface px-3 py-2 text-sm hover:border-line-strong">
                      <input
                        type="checkbox"
                        checked={isChecked}
                        onChange={() => togglePermission(perm)}
                        className="pv-checkbox"
                      />
                      <span className="font-mono text-xs text-fg">{perm}</span>
                    </label>
                  </li>
                );
              })}
            </ul>
          )}
          {!allGranted ? (
            <p className="text-xs text-fg-muted">{t("plugins.review.grantRequired")}</p>
          ) : null}
        </div>

        <div className="flex flex-wrap justify-end gap-2 pt-1">
          <Button size="sm" variant="outline" isDisabled={install.isPending} onPress={onClose}>
            {t("plugins.review.cancel")}
          </Button>
          <Button
            size="sm"
            variant="primary"
            isDisabled={!allGranted || install.isPending}
            onPress={handleConfirm}
          >
            <Download className="size-4" aria-hidden />
            {install.isPending ? t("plugins.review.installing") : t("plugins.review.confirm")}
          </Button>
        </div>
      </div>
    </div>
  );
}

function PluginSettingsDrawer({
  plugin,
  onClose,
}: {
  plugin: Plugin;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const update = useUpdatePlugin();
  const properties = plugin.settingsSchema?.properties ?? {};
  const propEntries = Object.entries(properties);

  const [values, setValues] = useState<Record<string, unknown>>(() => {
    const initial: Record<string, unknown> = {};
    for (const [key, spec] of propEntries) {
      initial[key] = plugin.settings?.[key] ?? spec.default ?? (spec.type === "boolean" ? false : "");
    }
    return initial;
  });

  const handleSave = (event: React.FormEvent) => {
    event.preventDefault();
    update.mutate(
      { id: plugin.id, body: { settings: values } },
      {
        onSuccess: () => {
          toast.success(t("plugins.toast.updated", { name: plugin.name }));
          onClose();
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("plugins.settings.title", { name: plugin.name })}
      className="fixed inset-0 z-50 flex justify-end bg-black/50"
    >
      <div className="flex h-full w-full max-w-md flex-col border-s border-line-strong bg-raised text-fg shadow-panel">
        <div className="flex items-start justify-between gap-3 border-b border-line px-5 py-4">
          <div className="grid min-w-0 gap-0.5">
            <h2 className="font-heading text-lg font-title text-heading">
              {t("plugins.settings.title", { name: plugin.name })}
            </h2>
            <p className="truncate font-mono text-xs text-fg-muted">
              {plugin.id} · v{plugin.version}
            </p>
          </div>
          <button
            type="button"
            aria-label={t("plugins.settings.cancel")}
            onClick={onClose}
            className="grid size-8 shrink-0 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>

        <form onSubmit={handleSave} className="flex min-h-0 flex-1 flex-col">
          <div className="grid flex-1 content-start gap-4 overflow-y-auto px-5 py-4">
            {propEntries.length === 0 ? (
              <p className="text-sm text-fg-muted">{t("plugins.settings.empty")}</p>
            ) : (
              propEntries.map(([key, spec]) => {
                const label = spec.title ?? key;
                const val = values[key];
                const inputId = `plugin-setting-${key}`;

                if (spec.type === "boolean") {
                  return (
                    <label
                      key={key}
                      htmlFor={inputId}
                      className="flex cursor-pointer items-center justify-between gap-3 rounded-tile border border-line bg-surface px-3.5 py-2.5"
                    >
                      <span className="text-sm font-mid text-fg">{label}</span>
                      <input
                        id={inputId}
                        type="checkbox"
                        checked={Boolean(val)}
                        onChange={(e) => setValues((prev) => ({ ...prev, [key]: e.target.checked }))}
                        className="pv-checkbox"
                      />
                    </label>
                  );
                }

                if (Array.isArray(spec.enum) && spec.enum.length > 0) {
                  return (
                    <div key={key} className="grid gap-1.5">
                      <label htmlFor={inputId} className="text-sm font-mid text-fg">
                        {label}
                      </label>
                      <select
                        id={inputId}
                        value={stringifySetting(val)}
                        onChange={(e) => setValues((prev) => ({ ...prev, [key]: e.target.value }))}
                        className="pv-select w-full"
                      >
                        {spec.enum.map((opt) => (
                          <option key={String(opt)} value={String(opt)}>
                            {String(opt)}
                          </option>
                        ))}
                      </select>
                    </div>
                  );
                }

                if (spec.type === "integer" || spec.type === "number") {
                  return (
                    <div key={key} className="grid gap-1.5">
                      <label htmlFor={inputId} className="text-sm font-mid text-fg">
                        {label}
                      </label>
                      <input
                        id={inputId}
                        type="number"
                        value={stringifySetting(val)}
                        min={spec.minimum}
                        max={spec.maximum}
                        onChange={(e) =>
                          setValues((prev) => ({
                            ...prev,
                            [key]: spec.type === "integer" ? parseInt(e.target.value, 10) : Number(e.target.value),
                          }))
                        }
                        className="pv-field w-full tabular-nums"
                      />
                    </div>
                  );
                }

                return (
                  <div key={key} className="grid gap-1.5">
                    <label htmlFor={inputId} className="text-sm font-mid text-fg">
                      {label}
                    </label>
                    <input
                      id={inputId}
                      type="text"
                      value={stringifySetting(val)}
                      minLength={spec.minLength}
                      maxLength={spec.maxLength}
                      onChange={(e) => setValues((prev) => ({ ...prev, [key]: e.target.value }))}
                      className="pv-field w-full"
                    />
                  </div>
                );
              })
            )}
          </div>

          <div className="flex flex-wrap justify-end gap-2 border-t border-line px-5 py-3.5">
            <Button size="sm" variant="outline" isDisabled={update.isPending} onPress={onClose}>
              {t("plugins.settings.cancel")}
            </Button>
            {propEntries.length > 0 ? (
              <Button size="sm" variant="primary" type="submit" isDisabled={update.isPending}>
                {update.isPending ? t("plugins.settings.saving") : t("plugins.settings.save")}
              </Button>
            ) : null}
          </div>
        </form>
      </div>
    </div>
  );
}

function PluginRow({
  plugin,
  skin,
  onOpenSettings,
}: {
  plugin: Plugin;
  skin?: InstalledSkin;
  onOpenSettings: (plugin: Plugin) => void;
}) {
  const { t } = useTranslation();
  const inset = useRowInset();
  const update = useUpdatePlugin();
  const rollback = useRollbackPlugin();
  const uninstall = useUninstallPlugin();
  const RuntimeIcon = RUNTIME_ICON[plugin.runtime] ?? Puzzle;
  const CategoryIcon = CATEGORY_ICON[classifyPluginCategory(plugin.provides, plugin.runtime)];
  const isManaged = Boolean(plugin.managed);
  const desc = localizedDescription(plugin.description);

  return (
    <li className={cn("grid gap-x-6 gap-y-3 py-4 md:grid-cols-[minmax(0,22rem)_minmax(0,1fr)_auto] md:items-start", inset)}>
      <div className="flex min-w-0 items-start gap-3 overflow-hidden">
        <span className="grid size-9 shrink-0 place-items-center rounded-tile bg-sunken text-icon">
          <CategoryIcon className="size-[1.125rem]" strokeWidth={1.8} aria-hidden />
        </span>
        <div className="grid min-w-0 flex-1 grid-cols-[minmax(0,1fr)] gap-1">
          <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
            <h3 className="min-w-0 max-w-full break-words text-[0.9375rem] leading-snug font-strong text-heading">
              {skin ? localized(skin.name, currentLanguage()) : plugin.name}
            </h3>
            <span className="shrink-0 text-xs font-body whitespace-nowrap text-fg-muted tabular-nums">
              {t("plugins.version", { version: plugin.version })}
            </span>
          </div>
          {skin ? (
            <p className="text-sm text-fg-muted">
              {t(skin.source === "upload" ? "appearance.unverified" : "plugins.official")}
            </p>
          ) : desc ? (
            <p className="break-words text-sm leading-relaxed text-fg-muted">{desc}</p>
          ) : null}
          <p className="truncate font-mono text-xs text-fg-subtle" title={plugin.id}>
            {plugin.id}
          </p>
        </div>
      </div>

      <div className="grid min-w-0 gap-2 max-md:ps-12">
        <div className="flex flex-wrap items-center gap-2">
          <p className="flex items-start gap-1.5 text-sm text-fg">
            <RuntimeIcon
              className={cn("mt-0.5 size-4 shrink-0", plugin.runtime === "sidecar" ? "text-degraded" : "text-icon")}
              aria-hidden
            />
            <span>
              <span className="font-mid">{t(`plugins.runtimes.${plugin.runtime}`)}</span>
              <span className="text-fg-muted"> · {t(`plugins.isolation.${plugin.runtime}`)}</span>
            </span>
          </p>
          {isManaged && plugin.riskLevel ? <RiskBadge level={plugin.riskLevel} /> : null}
        </div>
        {plugin.provides.length > 0 ? (
          <ul aria-label={t("plugins.provides")} className="flex flex-wrap gap-1.5">
            {plugin.provides.map((point) => (
              <li key={point}>
                <Chip size="sm" variant="soft" className="bg-sunken text-fg">
                  {KNOWN_POINTS.has(point) ? (
                    t(`plugins.points.${point}` as "plugins.points.storage")
                  ) : (
                    <span className="font-mono">{point}</span>
                  )}
                </Chip>
              </li>
            ))}
          </ul>
        ) : null}
      </div>

      <div className="grid justify-items-start gap-2 max-md:ps-12 md:justify-items-end">
        <div className="flex items-center gap-2">
          <StatusLabel tone={plugin.enabled ? "ok" : "idle"} className="text-sm">
            {plugin.enabled ? t("plugins.enabled") : t("plugins.disabled")}
          </StatusLabel>
          {isManaged ? (
            <button
              type="button"
              role="switch"
              aria-checked={plugin.enabled}
              aria-label={t("plugins.actions.toggle", { name: plugin.name })}
              disabled={update.isPending}
              onClick={() =>
                update.mutate(
                  { id: plugin.id, body: { enabled: !plugin.enabled } },
                  {
                    onSuccess: () => toast.success(t("plugins.toast.toggled", { name: plugin.name })),
                    onError: (err) => toast.danger(errorText(t, err)),
                  },
                )
              }
              className={cn(
                "relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-transparent transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                plugin.enabled ? "bg-emerald-600" : "bg-line-strong",
              )}
            >
              <span
                className={cn(
                  "pointer-events-none block size-4 rounded-full bg-white shadow-xs transition-transform",
                  plugin.enabled ? "translate-x-4" : "translate-x-0.5",
                )}
              />
            </button>
          ) : null}
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {skin ? (
            <Link to="/apps/themes" className={buttonVariants({ variant: "outline", size: "sm" })}>
              {t("plugins.manageSkin")}
            </Link>
          ) : null}

          {isManaged ? (
            <>
              <Button size="sm" variant="outline" onPress={() => onOpenSettings(plugin)}>
                <Settings2 className="size-3.5" aria-hidden />
                {t("plugins.actions.settings")}
              </Button>
              {plugin.previousVersion ? (
                <Button
                  size="sm"
                  variant="outline"
                  isDisabled={rollback.isPending}
                  onPress={() =>
                    rollback.mutate(plugin.id, {
                      onSuccess: (rolled) =>
                        toast.success(
                          t("plugins.toast.rolledBack", { name: plugin.name, version: rolled.version }),
                        ),
                      onError: (err) => toast.danger(errorText(t, err)),
                    })
                  }
                >
                  <History className="size-3.5" aria-hidden />
                  {t("plugins.actions.rollbackTo", { version: plugin.previousVersion })}
                </Button>
              ) : null}
              <Button
                size="sm"
                variant="outline"
                isDisabled={uninstall.isPending}
                onPress={() =>
                  uninstall.mutate(plugin.id, {
                    onSuccess: () => toast.success(t("plugins.toast.uninstalled", { name: plugin.name })),
                    onError: (err) => toast.danger(errorText(t, err)),
                  })
                }
              >
                <Trash2 className="size-3.5" aria-hidden />
                {t("plugins.actions.uninstall")}
              </Button>
            </>
          ) : null}
        </div>
      </div>
    </li>
  );
}

function PluginCategoryFilter({
  value,
  onChange,
  counts,
  showSkin,
}: {
  value: PluginCategory;
  onChange: (next: PluginCategory) => void;
  counts: Record<PluginCategory, number>;
  showSkin: boolean;
}) {
  const { t } = useTranslation();
  const options: PluginCategory[] = ["all", "metadata", "mediaEnhance", "widget"];
  if (showSkin) options.push("skin");
  if (counts.other > 0 || value === "other") options.push("other");

  return (
    <div
      role="group"
      aria-label={t("plugins.categoryLabel")}
      className="flex flex-wrap items-center gap-2"
    >
      {options.map((cat) => {
        const active = value === cat;
        const Icon = cat === "all" ? null : CATEGORY_ICON[cat];
        return (
          <button
            key={cat}
            type="button"
            aria-pressed={active}
            onClick={() => onChange(cat)}
            className={cn(
              "inline-flex h-8 shrink-0 items-center gap-1.5 rounded-full border px-3 text-xs font-mid whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              active
                ? "border-transparent bg-action font-strong text-action-fg shadow-xs"
                : "border-line bg-surface text-fg-muted hover:border-line-strong hover:text-fg",
            )}
          >
            {Icon ? <Icon className="size-3.5 shrink-0" aria-hidden /> : null}
            <span>{t(`plugins.categories.${cat}`)}</span>
            <span
              data-num
              className={cn(
                "rounded-full px-1.5 py-0.5 text-[0.6875rem] leading-none tabular-nums",
                active ? "bg-current/15 text-current" : "bg-sunken text-fg-subtle",
              )}
            >
              {counts[cat]}
            </span>
          </button>
        );
      })}
    </div>
  );
}

function RecipeStoreSection({
  search,
  category,
  onSelectInstall,
}: {
  search: string;
  category: PluginCategory;
  onSelectInstall: (entry: PluginCatalogEntry) => void;
}) {
  const { t } = useTranslation();
  const catalog = usePluginCatalog(true);
  const all = catalog.data?.plugins ?? [];
  const filtered = all.filter((entry) => {
    if (category !== "all" && classifyPluginCategory(entry.provides, entry.runtime) !== category) {
      return false;
    }
    if (!search) return true;
    const fields = [
      entry.name,
      entry.id,
      ...entry.provides,
      ...entry.permissions,
      ...Object.values(entry.description),
    ];
    for (const point of entry.provides) {
      if (KNOWN_POINTS.has(point)) fields.push(t(`plugins.points.${point}` as "plugins.points.storage"));
    }
    return fields.some((f) => f.toLocaleLowerCase().includes(search));
  });

  return (
    <Section title={t("plugins.store.title")} count={catalog.data ? filtered.length : undefined}>
      {catalog.isPending ? (
        <LoadingRows rows={3} />
      ) : catalog.isError ? (
        <ErrorState compact error={catalog.error} onRetry={() => void catalog.refetch()} />
      ) : filtered.length === 0 ? (
        <EmptyState compact icon={Puzzle} title={t("plugins.noMatches")} />
      ) : (
        <div className="grid grid-cols-1 gap-4 p-4 md:grid-cols-2 xl:grid-cols-3">
          {filtered.map((entry) => {
            const RuntimeIcon = RUNTIME_ICON[entry.runtime] ?? Puzzle;
            const CategoryIcon = CATEGORY_ICON[classifyPluginCategory(entry.provides, entry.runtime)];
            const desc = localizedDescription(entry.description);
            return (
              <article
                key={entry.id}
                className="flex min-w-0 flex-col justify-between gap-3.5 rounded-panel border border-line bg-surface p-4 transition-colors hover:border-line-strong"
              >
                <div className="grid min-w-0 gap-2.5">
                  <div className="flex items-start justify-between gap-2.5">
                    <div className="flex min-w-0 flex-1 items-start gap-2.5">
                      <span className="mt-0.5 grid size-8 shrink-0 place-items-center rounded-tile bg-sunken text-icon">
                        <CategoryIcon className="size-4" aria-hidden />
                      </span>
                      <div className="grid min-w-0 flex-1 gap-0.5">
                        <h3 className="line-clamp-2 text-[0.9375rem] leading-snug font-strong text-heading">
                          {entry.name}
                        </h3>
                        <p className="truncate font-mono text-xs text-fg-subtle" title={entry.id}>
                          {entry.id}
                        </p>
                      </div>
                    </div>
                    <RiskBadge level={entry.riskLevel} />
                  </div>

                  {desc ? <p className="text-sm leading-relaxed text-fg-muted">{desc}</p> : null}

                  <div className="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-xs text-fg-muted">
                    <span className="inline-flex shrink-0 items-center gap-1 font-mid whitespace-nowrap text-fg">
                      <RuntimeIcon className="size-3.5 shrink-0 text-icon" aria-hidden />
                      {t(`plugins.runtimes.${entry.runtime}`)}
                    </span>
                    <span aria-hidden>·</span>
                    <span className="shrink-0 whitespace-nowrap tabular-nums">v{entry.version}</span>
                    <span aria-hidden>·</span>
                    <span className="whitespace-nowrap">
                      {entry.permissions.length === 0
                        ? t("plugins.store.noPermissions")
                        : t("plugins.store.permissionsCount", { count: entry.permissions.length })}
                    </span>
                  </div>

                  <div className="flex flex-wrap items-center gap-1.5">
                    {entry.provides.map((point) => (
                      <Chip key={point} size="sm" variant="soft" className="shrink-0 whitespace-nowrap bg-sunken text-fg">
                        {KNOWN_POINTS.has(point)
                          ? t(`plugins.points.${point}` as "plugins.points.storage")
                          : point}
                      </Chip>
                    ))}
                  </div>
                </div>

                <div className="flex items-center justify-between gap-2 border-t border-line pt-3">
                  {entry.installed ? (
                    <span className="inline-flex min-w-0 items-center gap-1.5 text-xs font-strong whitespace-nowrap text-emerald-700 dark:text-emerald-300">
                      <CheckCircle2 className="size-4 shrink-0" aria-hidden />
                      <span className="truncate">
                        {t("plugins.store.installedVersion", {
                          version: entry.installedVersion ?? entry.version,
                        })}
                      </span>
                    </span>
                  ) : (
                    <span className="truncate text-xs text-fg-muted">{t("plugins.official")}</span>
                  )}
                  <Button
                    size="sm"
                    className="shrink-0"
                    variant={entry.installed ? "outline" : "primary"}
                    onPress={() => onSelectInstall(entry)}
                  >
                    <Download className="size-3.5" aria-hidden />
                    {entry.installed ? t("plugins.store.installed") : t("plugins.store.install")}
                  </Button>
                </div>
              </article>
            );
          })}
        </div>
      )}
    </Section>
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
  const [tab, setTab] = useState<"installed" | "store">("installed");
  const [category, setCategory] = useState<PluginCategory>("all");
  const [reviewEntry, setReviewEntry] = useState<PluginCatalogEntry | null>(null);
  const [settingsPlugin, setSettingsPlugin] = useState<Plugin | null>(null);
  const storeCatalog = usePluginCatalog(tab === "store");
  const skinRegistry = useSkinRegistry(tab === "store" && skinsEnabled);

  const search = input.trim().toLocaleLowerCase();
  const installedPlugins = new Map((query.data?.plugins ?? []).map((plugin) => [plugin.id, plugin]));
  for (const skin of skins.values()) {
    if (!installedPlugins.has(skin.pluginId)) {
      installedPlugins.set(skin.pluginId, {
        id: skin.pluginId,
        name: localized(skin.name, currentLanguage()),
        version: skin.version,
        runtime: "none",
        provides: ["skin"],
        enabled: true,
      });
    }
  }
  const plugins = Array.from(installedPlugins.values());
  const searchMatches = plugins.filter((plugin) => {
    const skin = skins.get(plugin.id);
    const fields = [plugin.name, plugin.id, ...plugin.provides, ...(skin ? Object.values(skin.name) : [])];
    for (const point of plugin.provides) {
      if (KNOWN_POINTS.has(point)) fields.push(t(`plugins.points.${point}` as "plugins.points.storage"));
    }
    return fields.some((field) => field.toLocaleLowerCase().includes(search));
  });
  const matches =
    category === "all"
      ? searchMatches
      : searchMatches.filter((plugin) => classifyPluginCategory(plugin.provides, plugin.runtime) === category);

  const installedCounts: Record<PluginCategory, number> = {
    all: searchMatches.length,
    metadata: 0,
    mediaEnhance: 0,
    widget: 0,
    skin: 0,
    other: 0,
  };
  for (const plugin of searchMatches) {
    installedCounts[classifyPluginCategory(plugin.provides, plugin.runtime)] += 1;
  }

  const storeEntries = (storeCatalog.data?.plugins ?? []).filter((entry) => {
    if (!search) return true;
    const fields = [
      entry.name,
      entry.id,
      ...entry.provides,
      ...entry.permissions,
      ...Object.values(entry.description),
    ];
    for (const point of entry.provides) {
      if (KNOWN_POINTS.has(point)) fields.push(t(`plugins.points.${point}` as "plugins.points.storage"));
    }
    return fields.some((field) => field.toLocaleLowerCase().includes(search));
  });
  const storeSkinsCount = (skinRegistry.data?.skins ?? []).filter((skin) => {
    if (skin.installed) return false;
    const fields = [
      skin.id,
      skin.pluginId,
      ...Object.values(skin.name),
      ...Object.values(skin.description),
      t("plugins.points.skin"),
    ];
    return fields.some((field) => field.toLocaleLowerCase().includes(search));
  }).length;
  const storeCounts: Record<PluginCategory, number> = {
    all: storeEntries.length + storeSkinsCount,
    metadata: 0,
    mediaEnhance: 0,
    widget: 0,
    skin: storeSkinsCount,
    other: 0,
  };
  for (const entry of storeEntries) {
    storeCounts[classifyPluginCategory(entry.provides, entry.runtime)] += 1;
  }

  const enabled = plugins.filter((plugin) => plugin.enabled).length;
  const switchTab = (next: "installed" | "store") => {
    setTab(next);
    setCategory("all");
  };

  return (
    <PageFrame>
      <PageHeader
        title={t("plugins.storeTitle")}
        actions={skinsEnabled ? <UploadSkinButton /> : undefined}
        summary={query.data && plugins.length > 0 ? t("plugins.summary", { count: plugins.length, enabled }) : undefined}
      />

      <div className="grid gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <div role="tablist" aria-label={t("plugins.tabs.label")} className="pv-segmented">
            <button
              type="button"
              role="tab"
              aria-selected={tab === "installed"}
              onClick={() => switchTab("installed")}
            >
              {t("plugins.tabs.installed")}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={tab === "store"}
              onClick={() => switchTab("store")}
            >
              {t("plugins.tabs.store")}
            </button>
          </div>

          <Input
            type="search"
            aria-label={t("plugins.search")}
            placeholder={t("plugins.search")}
            className="w-full sm:max-w-sm"
            value={input}
            onChange={(event) => setInput(event.target.value)}
          />

          <a
            className="text-sm font-mid text-link underline"
            href="https://github.com/palveni-labs/plugins#submit-a-plugin"
            target="_blank"
            rel="noreferrer"
          >
            {t("plugins.contribute")}
          </a>
        </div>

        {tab === "store" || plugins.length > 0 ? (
          <PluginCategoryFilter
            value={category}
            onChange={setCategory}
            counts={tab === "store" ? storeCounts : installedCounts}
            showSkin={skinsEnabled || installedCounts.skin > 0}
          />
        ) : null}
      </div>

      {tab === "installed" ? (
        <Section title={t("plugins.installedTitle")} count={query.data ? matches.length : undefined}>
          {query.isPending ? (
            <LoadingPlugins />
          ) : query.isError ? (
            <ErrorState compact error={query.error} onRetry={() => void query.refetch()} />
          ) : matches.length === 0 && (search || category !== "all") ? (
            <EmptyState compact icon={Puzzle} title={t("plugins.noMatches")} />
          ) : plugins.length === 0 ? (
            <EmptyState
              compact
              icon={Puzzle}
              title={t("plugins.empty")}
              detail={t("plugins.emptyDetail")}
              action={
                <div className="flex flex-wrap items-center gap-2">
                  <Button size="sm" variant="primary" onPress={() => switchTab("store")}>
                    {t("plugins.store.browseStore")}
                  </Button>
                  <Link to="/apps/themes" className={buttonVariants({ variant: "outline", size: "sm" })}>
                    {t("plugins.toAppearance")}
                  </Link>
                </div>
              }
            />
          ) : (
            <SectionRows>
              {matches.map((plugin) => (
                <PluginRow
                  key={plugin.id}
                  plugin={plugin}
                  skin={skins.get(plugin.id)}
                  onOpenSettings={setSettingsPlugin}
                />
              ))}
            </SectionRows>
          )}
        </Section>
      ) : (
        <div className="grid gap-6">
          {category !== "skin" ? (
            <RecipeStoreSection search={search} category={category} onSelectInstall={setReviewEntry} />
          ) : null}
          {category === "all" || category === "skin" ? (
            <OfficialSkinCatalog enabled={skinsEnabled} search={search} store />
          ) : null}
        </div>
      )}

      {reviewEntry ? (
        <PluginInstallReviewModal entry={reviewEntry} onClose={() => setReviewEntry(null)} />
      ) : null}

      {settingsPlugin ? (
        <PluginSettingsDrawer plugin={settingsPlugin} onClose={() => setSettingsPlugin(null)} />
      ) : null}
    </PageFrame>
  );
}
