import { CheckCircle2, MemoryStick, MonitorSmartphone, Zap, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { useHardware } from "@/api/queries/system";
import { formatBytes } from "@/lib/format";
import { useFeature } from "@/modules/access";
import { useMetrics } from "@/provisional/queries/catalog";
import { useStorage } from "@/provisional/queries/storage";
import { useSlot } from "@/skins/scope";
import { StatusLabel } from "@/ui/marks";
import { ReadingMeter } from "@/ui/progress";
import { useRowInset } from "@/ui/section";
import { LoadingRows } from "@/ui/states";

const RING_R = 24;
const RING_C = 2 * Math.PI * RING_R;

function TelemetryRing({
  label,
  fraction,
  detail,
  strokeColor,
}: {
  label: string;
  fraction: number;
  detail?: string;
  strokeColor: string;
}) {
  const clamped = Math.min(1, Math.max(0, fraction));
  const pct = Math.round(clamped * 100);
  const offset = RING_C * (1 - clamped);
  return (
    <div className="flex flex-col items-center gap-1.5 rounded-xl border border-line/65 bg-sunken/55 px-2 py-3 text-center">
      <div className="relative grid size-15 place-items-center">
        <svg viewBox="0 0 60 60" className="size-full -rotate-90" aria-hidden>
          <circle cx="30" cy="30" r={RING_R} fill="none" stroke="var(--pv-progress-rest)" strokeWidth="5.5" />
          <circle
            cx="30"
            cy="30"
            r={RING_R}
            fill="none"
            stroke={strokeColor}
            strokeWidth="5.5"
            strokeLinecap="round"
            strokeDasharray={RING_C}
            strokeDashoffset={offset}
            className="transition-[stroke-dashoffset] duration-500 ease-out"
          />
        </svg>
        <span data-num className="absolute font-heading text-sm font-title text-heading tabular-nums">
          {pct}%
        </span>
      </div>
      <div className="grid min-w-0 gap-0.5">
        <span className="truncate text-xs font-strong text-fg">{label}</span>
        {detail ? (
          <span data-num className="truncate text-[0.6875rem] text-fg-muted tabular-nums">
            {detail}
          </span>
        ) : null}
      </div>
    </div>
  );
}

function Reading({ icon: Icon, label, value, meter, note }: { icon: LucideIcon; label: string; value: ReactNode; meter?: { value: number; label: string; warn?: boolean }; note?: ReactNode }) {
  const progress = useSlot("progress");
  const inset = useRowInset();
  const separator = progress === "separator";
  return (
    <li className={cn("relative grid min-h-11 grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3 py-2.5", inset)}>
      <span className="flex min-w-0 items-center gap-2.5 text-sm text-fg-muted">
        <span className="grid size-7 shrink-0 place-items-center rounded-lg bg-sunken/80 text-icon">
          <Icon className="size-3.5" aria-hidden />
        </span>
        <span className="truncate">{label}</span>
      </span>
      <span className="text-end">
        <span className="text-sm font-strong text-fg">{value}</span>
        {note ? <span className="block text-xs text-fg-muted">{note}</span> : null}
      </span>
      {separator && meter ? <ReadingMeter value={meter.value} label={meter.label} warn={meter.warn} /> : null}
    </li>
  );
}

/** 服务器: load, memory, storage, transcoding and connected devices. */
export default function ServerWidget() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const isAdmin = useIsAdmin();
  const hasMetrics = useFeature("metrics");
  const hasStorage = useFeature("storage");
  const hasHardware = useFeature("hardware") && isAdmin;
  const metrics = useMetrics(hasMetrics);
  const storage = useStorage(hasStorage);
  const hardware = useHardware({ enabled: hasHardware });
  const inset = useRowInset();
  const pending = (hasMetrics && metrics.isPending) || (hasHardware && hardware.isPending);
  if (pending) return <LoadingRows rows={4} className="px-5 pb-4" />;

  const m = metrics.data;
  const volume = storage.data?.volumes[0];
  const hwMethods = new Set(hardware.data?.encoders.filter((encoder) => encoder.verified && encoder.method !== "software").map((encoder) => encoder.method) ?? []);
  const hw = (["qsv", "nvenc", "videotoolbox", "amf", "rkmpp", "vaapi", "v4l2m2m"] as const).find((method) => hwMethods.has(method));
  const gb = (bytes: number) => (bytes / 1024 ** 3).toFixed(1).replace(/\.0$/, "");
  const tb = (bytes: number) => (bytes / 1024 ** 4).toFixed(1).replace(/\.0$/, "");
  const storageFrac = volume && volume.totalBytes > 0 ? volume.usedBytes / volume.totalBytes : null;

  const hasRings = Boolean(m || storageFrac !== null);

  return (
    <div className="grid gap-1 pb-3">
      {hasRings ? (
        <div
          className={cn(
            "grid gap-2.5 pb-2",
            inset,
            m && storageFrac !== null ? "grid-cols-3" : m ? "grid-cols-2" : "grid-cols-1",
          )}
        >
          {m ? (
            <TelemetryRing
              label={t("server.cpu")}
              fraction={m.cpu}
              strokeColor="oklch(0.60 0.16 248)"
            />
          ) : null}
          {m ? (
            <TelemetryRing
              label={t("server.memory")}
              fraction={m.memoryTotalBytes > 0 ? m.memoryUsedBytes / m.memoryTotalBytes : 0}
              detail={`${gb(m.memoryUsedBytes)}/${gb(m.memoryTotalBytes)}G`}
              strokeColor="oklch(0.64 0.13 206)"
            />
          ) : null}
          {volume && storageFrac !== null ? (
            <TelemetryRing
              label={t("server.storage")}
              fraction={storageFrac}
              detail={`${tb(volume.usedBytes)}/${tb(volume.totalBytes)}T`}
              strokeColor={storageFrac > 0.9 ? "var(--pv-degraded)" : "oklch(0.66 0.14 156)"}
            />
          ) : null}
        </div>
      ) : null}
      <ul className="divide-y divide-line/60">
        {!m && hardware.data ? (
          <Reading icon={MemoryStick} label={t("server.memory")} value={formatBytes(hardware.data.memoryBytes, locale)} />
        ) : null}
        {hasHardware ? (
          <Reading
            icon={Zap}
            label={t("server.transcoding")}
            value={
              hw ? (
                <StatusLabel tone="ok" className="font-strong">
                  {t("server.hwAvailable", { method: t(`hardware.methods.${hw}`) })}
                </StatusLabel>
              ) : (
                <span className="font-mid text-fg-muted">{t("hardware.softwareOnly")}</span>
              )
            }
          />
        ) : null}
        {m ? (
          <Reading icon={MonitorSmartphone} label={t("server.devices")} value={t("server.devicesOnline", { count: m.devicesOnline })} />
        ) : null}
        {!m && !hasHardware && !volume ? (
          <li className={cn("flex items-center gap-2 py-4 text-sm text-fg-muted", inset)}>
            <CheckCircle2 className="size-4 text-ok" aria-hidden />
            {t("server.running")}
          </li>
        ) : null}
      </ul>
    </div>
  );
}
