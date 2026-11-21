import { CheckCircle2, type Cpu, MemoryStick, Zap } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { useHardware } from "@/api/queries/system";
import { formatBytes } from "@/lib/format";
import { useFeature } from "@/modules/access";
import { useSlot } from "@/skins/scope";
import { StatusLabel } from "@/ui/marks";
import { ReadingMeter } from "@/ui/progress";
import { useRowInset } from "@/ui/section";
import { LoadingRows } from "@/ui/states";

function Reading({ icon: Icon, label, value, meter, note }: { icon: typeof Cpu; label: string; value: ReactNode; meter?: { value: number; label: string; warn?: boolean }; note?: ReactNode }) {
  const progress = useSlot("progress");
  const measure = useSlot("measure");
  const inset = useRowInset();
  const separator = progress === "separator";
  return (
    <li className={cn("relative grid min-h-11 grid-cols-[5.5rem_minmax(0,1fr)_auto] items-center gap-x-3 py-2.5", inset, separator && "grid-cols-[minmax(0,1fr)_auto]")}>
      <span className="flex items-center gap-2.5 text-sm text-fg-muted">
        {measure === "mono" ? <Icon className="size-4 text-icon" aria-hidden /> : null}
        {label}
      </span>
      {!separator ? <span>{meter ? <ReadingMeter value={meter.value} label={meter.label} warn={meter.warn} /> : null}</span> : null}
      <span className="text-end">
        <span className="text-base font-strong text-fg">{value}</span>
        {note ? <span className="block text-xs text-fg-muted">{note}</span> : null}
      </span>
      {separator && meter ? <ReadingMeter value={meter.value} label={meter.label} warn={meter.warn} /> : null}
    </li>
  );
}

/** Hardware summary for the current server. */
export default function ServerWidget() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const isAdmin = useIsAdmin();
  const hasHardware = useFeature("hardware") && isAdmin;
  const hardware = useHardware({ enabled: hasHardware });
  const progress = useSlot("progress");
  const pending = (hasHardware && hardware.isPending);
  if (pending) return <LoadingRows rows={4} className="px-5 pb-4" />;

  const hwMethods = new Set(hardware.data?.encoders.filter((encoder) => encoder.verified && encoder.method !== "software").map((encoder) => encoder.method) ?? []);
  const hw = (["qsv", "nvenc", "videotoolbox", "amf", "rkmpp", "vaapi", "v4l2m2m"] as const).find((method) => hwMethods.has(method));

  return (
    <ul className={cn("pb-3", progress === "separator" && "[&>li]:pb-3.5")}>
      {hardware.data ? (
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
      {!hasHardware ? (
        <li className="flex items-center gap-2 px-5 py-4 text-sm text-fg-muted">
          <CheckCircle2 className="size-4 text-ok" aria-hidden />
          {t("server.running")}
        </li>
      ) : null}
    </ul>
  );
}
