import { Link } from "@tanstack/react-router";
import { Clock, HardDrive, Puzzle, Quote } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import { usePlugins } from "@/api/queries/plugins";
import type { Plugin } from "@/api/types";
import { cn } from "@/lib/cn";
import { formatBytes, formatNumber } from "@/lib/format";
import { useFeature } from "@/modules/access";
import { useCatalog } from "@/provisional/queries/catalog";
import { useStorage } from "@/provisional/queries/storage";

interface DailyQuote {
  domain: "video" | "book";
  zh: { text: string; source: string };
  en: { text: string; source: string };
}

const DAILY_QUOTES: readonly DailyQuote[] = [
  {
    domain: "video",
    zh: { text: "给岁月以文明，而不是给文明以岁月。", source: "科幻影像摘录" },
    en: { text: "Give the years to civilization, not civilization to the years.", source: "Sci-Fi Cinema" },
  },
  {
    domain: "book",
    zh: { text: "所有伟大的旅程，都始于翻开第一页的片刻宁静。", source: "纸墨书摘" },
    en: { text: "Every great voyage begins in the quiet moment of turning the first page.", source: "Reading Excerpt" },
  },
  {
    domain: "video",
    zh: { text: "好电影像一束穿过暗室的光，照见未曾抵达的远方。", source: "家庭影院随笔" },
    en: { text: "Great cinema is a beam across a dark room, revealing worlds not yet visited.", source: "Cinema Notes" },
  },
  {
    domain: "book",
    zh: { text: "书架上的每一卷旧书，都是某个夜晚留下的灯火。", source: "书房笔记" },
    en: { text: "Every book on the shelf keeps a lantern lit from a quiet evening past.", source: "Study Notes" },
  },
];

function readBoolSetting(plugin: Plugin, key: string, fallback: boolean): boolean {
  const settings = plugin.settings;
  if (!settings || typeof settings !== "object" || Array.isArray(settings)) return fallback;
  const raw = settings[key];
  return typeof raw === "boolean" ? raw : fallback;
}

function readStringSetting(plugin: Plugin, key: string, fallback: string): string {
  const settings = plugin.settings;
  if (!settings || typeof settings !== "object" || Array.isArray(settings)) return fallback;
  const raw = settings[key];
  return typeof raw === "string" && raw.trim() ? raw.trim() : fallback;
}

function ClockStorageWidgetCard({ plugin }: { plugin: Plugin }) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "zh-CN";
  const use24Hour = readBoolSetting(plugin, "use24Hour", true);
  const showStorageBadge = readBoolSetting(plugin, "showStorageBadge", true);
  const hasStorage = useFeature("storage");
  const { data: storage } = useStorage(showStorageBadge && hasStorage);
  const { data: libData } = useLibraries();

  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 30_000);
    return () => window.clearInterval(id);
  }, []);

  const timeText = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        hour: "2-digit",
        minute: "2-digit",
        hour12: !use24Hour,
      }).format(now),
    [locale, now, use24Hour],
  );
  const dateText = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        month: "long",
        day: "numeric",
        weekday: "short",
      }).format(now),
    [locale, now],
  );

  const totalUsedBytes = useMemo(
    () => (storage?.byLibrary ?? []).reduce((sum, entry) => sum + entry.bytes, 0),
    [storage],
  );
  const libraryCount = libData?.libraries.length ?? 0;

  return (
    <article
      data-plugin-widget={plugin.id}
      className="pv-glass-panel pv-glow-sky flex items-center justify-between gap-4 rounded-panel border px-5 py-4"
    >
      <div className="flex min-w-0 items-center gap-3.5">
        <span className="grid size-11 shrink-0 place-items-center rounded-xl bg-[oklch(0.56_0.13_212/0.16)] text-[oklch(0.48_0.13_212)] dark:bg-[oklch(0.62_0.13_210/0.24)] dark:text-[oklch(0.80_0.12_208)]">
          <Clock className="size-5" aria-hidden />
        </span>
        <div className="grid min-w-0 gap-0.5">
          <span
            data-num
            className="font-heading text-2xl leading-tight font-title tracking-tight text-heading tabular-nums"
          >
            {timeText}
          </span>
          <span className="truncate text-xs text-fg-muted">{dateText}</span>
        </div>
      </div>

      {showStorageBadge ? (
        <div className="flex shrink-0 items-center gap-2 rounded-control border border-line/70 bg-sunken/70 px-3 py-1.5 text-xs text-fg">
          <HardDrive className="size-3.5 text-icon" aria-hidden />
          <span data-num className="font-mid tabular-nums">
            {totalUsedBytes > 0
              ? formatBytes(totalUsedBytes, locale)
              : t("overview.pluginWidgets.librariesCount", {
                  count: formatNumber(libraryCount, locale),
                })}
          </span>
        </div>
      ) : null}
    </article>
  );
}

function MediaQuoteWidgetCard({ plugin }: { plugin: Plugin }) {
  const { t, i18n } = useTranslation();
  const isZh = (i18n.resolvedLanguage ?? "zh-CN").toLowerCase().startsWith("zh");
  const sourceDomain = readStringSetting(plugin, "sourceDomain", "all");
  const hasCatalog = useFeature("catalog");
  const { data: catalog } = useCatalog(hasCatalog);
  const [dayNumber] = useState(() => Math.floor(Date.now() / 86_400_000));

  const featured = useMemo(() => {
    const recent = catalog?.recent ?? [];
    const matchingWork = recent.find((w) => {
      if (sourceDomain === "video") return w.kind === "movie" || w.kind === "series";
      if (sourceDomain === "book") return w.kind === "book";
      return w.kind === "movie" || w.kind === "series" || w.kind === "book";
    });
    const pool =
      sourceDomain === "video"
        ? DAILY_QUOTES.filter((q) => q.domain === "video")
        : sourceDomain === "book"
          ? DAILY_QUOTES.filter((q) => q.domain === "book")
          : DAILY_QUOTES;
    const dayIndex = dayNumber % pool.length;
    const quote = pool[dayIndex] ?? DAILY_QUOTES[0]!;
    const localized = isZh ? quote.zh : quote.en;
    return {
      text: localized.text,
      source: matchingWork ? matchingWork.title : localized.source,
    };
  }, [catalog, dayNumber, isZh, sourceDomain]);

  return (
    <article
      data-plugin-widget={plugin.id}
      className="pv-glass-panel pv-glow-amber flex items-start gap-3.5 rounded-panel border px-5 py-4"
    >
      <span className="mt-0.5 grid size-9 shrink-0 place-items-center rounded-xl bg-[oklch(0.64_0.15_70/0.16)] text-[oklch(0.52_0.15_64)] dark:bg-[oklch(0.76_0.16_74/0.22)] dark:text-[oklch(0.85_0.14_76)]">
        <Quote className="size-4" aria-hidden />
      </span>
      <div className="grid min-w-0 gap-1">
        <p className="text-sm leading-relaxed font-mid text-fg">“{featured.text}”</p>
        <span className="truncate text-xs text-fg-muted">
          — {featured.source} · {t("overview.pluginWidgets.dailyPick")}
        </span>
      </div>
    </article>
  );
}

function GenericPluginWidgetCard({ plugin }: { plugin: Plugin }) {
  const { t } = useTranslation();
  return (
    <article
      data-plugin-widget={plugin.id}
      className="pv-glass-panel flex items-center justify-between gap-3 rounded-panel border px-5 py-4"
    >
      <div className="flex min-w-0 items-center gap-3">
        <span className="grid size-9 shrink-0 place-items-center rounded-xl bg-sunken text-icon">
          <Puzzle className="size-4" aria-hidden />
        </span>
        <div className="grid min-w-0 gap-0.5">
          <span className="truncate text-sm font-strong text-heading">{plugin.name}</span>
          <span className="truncate font-mono text-xs text-fg-muted">v{plugin.version}</span>
        </div>
      </div>
      <Link
        to="/apps/plugins"
        className="shrink-0 rounded-control px-2.5 py-1 text-xs font-mid text-link transition-colors hover:bg-hover"
      >
        {t("overview.pluginWidgets.manage")}
      </Link>
    </article>
  );
}

/**
 * Renders enabled installed widget plugins (`runtime === "widget"`) directly on Overview.
 */
export function PluginWidgetsRow({ className }: { className?: string }) {
  const isAdmin = useIsAdmin();
  const hasPlugins = useFeature("plugins");
  const { data } = usePlugins(isAdmin && hasPlugins);

  const widgets = useMemo(
    () =>
      (data?.plugins ?? []).filter(
        (plugin) =>
          plugin.enabled &&
          (plugin.runtime === "widget" || plugin.provides.includes("widget")),
      ),
    [data],
  );

  if (widgets.length === 0) return null;

  return (
    <section
      data-testid="overview-plugin-widgets"
      className={cn(
        "grid gap-3.5",
        widgets.length > 1 ? "md:grid-cols-2" : "grid-cols-1",
        className,
      )}
    >
      {widgets.map((plugin) => {
        if (plugin.id === "palveni.widget.clock_weather") {
          return <ClockStorageWidgetCard key={plugin.id} plugin={plugin} />;
        }
        if (plugin.id === "palveni.widget.media_quote") {
          return <MediaQuoteWidgetCard key={plugin.id} plugin={plugin} />;
        }
        return <GenericPluginWidgetCard key={plugin.id} plugin={plugin} />;
      })}
    </section>
  );
}
