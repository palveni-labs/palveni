/**
 * Connection status and current work, placed by the `status` slot: `inline` sits next to the
 * server name; `activity-window` is a resident window in the middle of the toolbar showing the
 * connection, the most important running work and how many things need attention.
 */
import { Dropdown, Label } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { AlertTriangle, ChevronDown, Info, Settings2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useEventStreamStatus } from "@/api/events";
import { useSystemInfo } from "@/api/queries/system";
import { MODULES } from "@/modules/registry";
import type { ActivitySummary } from "@/modules/types";
import { Lamp, Measure } from "@/ui/marks";
import { useInboxItems } from "./inbox-store";
import { shell } from "./shell-store";

export function useConnectionLabel(): { live: boolean; label: string } {
  const { t } = useTranslation();
  const status = useEventStreamStatus();
  const live = status === "open";
  return { live, label: live ? t("status.connected") : status === "connecting" ? t("status.connecting") : t("status.reconnecting") };
}

const CONTAINER_HOSTNAME = /^[0-9a-f]{12}$/i;

function displayServerName(raw: string | undefined, fallback: string): string {
  if (!raw) return "…";
  return CONTAINER_HOSTNAME.test(raw.trim()) ? fallback : raw;
}

/** Server name with a menu (one server today; the menu holds its details). */
export function ServerSwitch({ className, tone = "shell" }: { className?: string; tone?: "shell" | "plain" }) {
  const { t } = useTranslation();
  const { data: info } = useSystemInfo();
  const label = displayServerName(info?.name, t("app.name"));
  return (
    <Dropdown>
      <Dropdown.Trigger
        aria-label={t("status.serverMenu")}
        className={cn(
          "inline-flex h-8 min-w-0 items-center gap-1.5 rounded-control px-2.5 text-sm font-mid outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus",
          tone === "shell" ? "text-shell-fg hover:bg-shell-field" : "text-fg hover:bg-hover",
          className,
        )}
      >
        <span className="max-w-[10rem] truncate">{label}</span>
        <ChevronDown className="size-3.5 shrink-0 opacity-65" aria-hidden />
      </Dropdown.Trigger>
      <Dropdown.Popover placement="bottom start" className="min-w-64">
        <Dropdown.Menu aria-label={t("status.serverMenu")}>
          <Dropdown.Item id="info" textValue={info?.name ?? ""} isDisabled className="opacity-100">
            <Info className="size-4 text-fg-muted" aria-hidden />
            <div className="grid">
              <Label className="text-fg">{info?.name}</Label>
              <span className="text-xs text-fg-muted">{t("status.version", { version: info?.version ?? "" })}</span>
            </div>
          </Dropdown.Item>
          <Dropdown.Item id="settings" textValue={t("nav.settings")} href="/system/settings">
            <Settings2 className="size-4 text-fg-muted" aria-hidden />
            <Label>{t("status.serverSettings")}</Label>
          </Dropdown.Item>
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}

/** "● 已连接" next to the server name. */
export function InlineStatus({ className, tone = "shell" }: { className?: string; tone?: "shell" | "plain" }) {
  const { live, label } = useConnectionLabel();
  return (
    <span
      role="status"
      className={cn(
        "inline-flex items-center gap-1.5 text-xs whitespace-nowrap",
        tone === "shell" ? "text-shell-fg-muted" : "text-fg-muted",
        className,
      )}
    >
      <Lamp tone={live ? "ok" : "degraded"} />
      {label}
    </span>
  );
}

const useNoActivity = (): ActivitySummary | null => null;
/** The registry is fixed at build time, so this hook choice never changes between renders. */
const useModuleActivity = MODULES.find((module) => module.activity)?.activity ?? useNoActivity;

/** The resident activity window (toolbar centre). */
export function ActivityWindow({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { data: info } = useSystemInfo();
  const { live, label } = useConnectionLabel();
  const activity = useModuleActivity();
  const alerts = useInboxItems().filter((item) => item.kind === "alert").length;
  const p = activity?.progress ?? null;
  return (
    <div className={cn("relative flex h-9 min-w-0 items-stretch overflow-hidden rounded-tile border border-attention-line bg-attention-soft text-sm text-fg", className)}>
      <Link to="/system/settings" className="flex shrink-0 items-center gap-1.5 px-3 hover:bg-hover/60">
        <Lamp tone={live ? "ok" : "degraded"} />
        <span className="font-strong">{info?.name}</span>
        <span className="text-fg-muted">{label}</span>
      </Link>
      {activity ? (
        <Link to={activity.to} className="flex min-w-0 flex-1 items-center gap-2 border-s border-attention-line px-3 hover:bg-hover/60">
          <span className="shrink-0 text-fg-muted">{activity.label}</span>
          <span className="min-w-0 truncate">{activity.title}</span>
          {p !== null ? <Measure value={`${Math.round(p * 100)}%`} className="ms-auto shrink-0 font-strong" /> : null}
          {activity.more > 0 ? <span className="shrink-0 text-fg-muted">+{activity.more}</span> : null}
        </Link>
      ) : (
        <span className="flex flex-1 items-center border-s border-attention-line px-3 text-fg-muted">{t("status.idle")}</span>
      )}
      {alerts > 0 ? (
        <button
          type="button"
          onClick={() => shell.setInbox(true)}
          className="flex shrink-0 items-center gap-1 border-s border-attention-line px-2.5 text-degraded hover:bg-hover/60"
          aria-label={t("status.alerts", { count: alerts })}
        >
          <AlertTriangle className="size-3.5" aria-hidden />
          <span className="tabular-nums">{alerts}</span>
        </button>
      ) : null}
      {p !== null ? (
        <span aria-hidden className="absolute bottom-0 left-0 h-0.5 bg-active-mark transition-[width] duration-700" style={{ width: `${p * 100}%` }} />
      ) : null}
    </div>
  );
}
