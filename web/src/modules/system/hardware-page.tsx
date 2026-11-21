import { Button, Chip, Spinner, toast } from "@heroui/react";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useHardware, useRefreshHardware } from "@/api/queries/system";
import type { EncodeMethod, EncoderCapability, HardwareReport, ToolInfo } from "@/api/types";
import { RelativeTime } from "@/components/relative-time";
import { errorText } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import { PageFrame } from "@/ui/frame";
import { Measure, StatusLabel } from "@/ui/marks";
import { PageHeader, SummaryFacts, type Fact } from "@/ui/page";
import { Section, SectionRows, useRowInset, useSectionVariant } from "@/ui/section";
import { ErrorState, LoadingRows } from "@/ui/states";
import { ENCODER_STATUSES, ENCODER_TONE, encoderStatus } from "./encoder-status";

const METHOD_ORDER: EncodeMethod[] = ["software", "vaapi", "qsv", "nvenc", "amf", "rkmpp", "v4l2m2m", "videotoolbox"];
const HW_PREFERENCE: EncodeMethod[] = ["qsv", "nvenc", "videotoolbox", "amf", "rkmpp", "vaapi", "v4l2m2m"];
const CODEC_ORDER = ["h264", "hevc", "av1", "vp9", "vp8", "mpeg2video", "mjpeg"];
const CODEC_LABELS: Record<string, string> = {
  h264: "H.264",
  hevc: "HEVC",
  av1: "AV1",
  vp9: "VP9",
  vp8: "VP8",
  mpeg2video: "MPEG-2",
  mjpeg: "MJPEG",
};

function codecLabel(codec: string): string {
  return CODEC_LABELS[codec] ?? codec.toUpperCase();
}

function orderIndex<T>(order: readonly T[], value: T): number {
  const index = order.indexOf(value);
  return index === -1 ? order.length : index;
}

/* ---------------------------------------------------------------- summary */

function Summary({ report }: { report: HardwareReport }) {
  const { t, i18n } = useTranslation();
  const verifiedHw = report.encoders.filter((encoder) => encoder.verified && encoder.method !== "software");
  const methods = new Set(verifiedHw.map((encoder) => encoder.method));
  const best = HW_PREFERENCE.find((method) => methods.has(method));
  const facts: Fact[] = [
    { label: t("hardware.perfTier"), value: t(`hardware.tiers.${report.perfTier}.name`), note: t(`hardware.tiers.${report.perfTier}.summary`) },
    {
      label: t("hardware.cpu"),
      value: t("hardware.cores", { count: report.cpu.cores }),
      note: (
        <>
          <span>{report.cpu.model}</span>
          <span aria-hidden> · </span>
          {report.cpu.arch}
        </>
      ),
    },
    { label: t("hardware.memory"), value: <Measure value={formatBytes(report.memoryBytes, i18n.resolvedLanguage ?? "en")} /> },
    {
      label: t("hardware.hwEncoding"),
      value: best ? t(`hardware.methods.${best}`) : t("hardware.softwareOnly"),
      note: verifiedHw.length > 0 ? t("hardware.verifiedHw", { count: verifiedHw.length }) : t("hardware.noneVerified"),
      tone: best ? "default" : "degraded",
    },
  ];
  return <SummaryFacts facts={facts} />;
}

/* ---------------------------------------------------------------- tools */

function ToolRow({ name, tool }: { name: string; tool: ToolInfo }) {
  const { t } = useTranslation();
  const inset = useRowInset();
  return (
    <li className={cn("grid gap-x-6 gap-y-1.5 py-3 md:grid-cols-[7rem_8rem_minmax(0,1fr)]", inset)}>
      <span className="font-strong text-fg">{name}</span>
      <StatusLabel tone={tool.found ? "ok" : "failed"} className="text-sm">
        {tool.found ? t("hardware.toolFound") : t("hardware.toolMissing")}
      </StatusLabel>
      {tool.found ? (
        <dl className="grid min-w-0 gap-x-6 gap-y-1 text-sm sm:grid-cols-[auto_auto_minmax(0,1fr)]">
          <div className="flex min-w-0 gap-2">
            <dt className="shrink-0 text-fg-muted">{t("hardware.version")}</dt>
            <dd className="truncate text-fg" title={tool.version ?? undefined}>
              {tool.version ?? "—"}
            </dd>
          </div>
          <div className="flex min-w-0 gap-2">
            <dt className="shrink-0 text-fg-muted">{t("hardware.source")}</dt>
            <dd className="text-fg">{tool.source ? t(`hardware.sources.${tool.source}`) : "—"}</dd>
          </div>
          <div className="flex min-w-0 gap-2">
            <dt className="shrink-0 text-fg-muted">{t("hardware.path")}</dt>
            <dd className="min-w-0 font-mono break-all text-fg">{tool.path ?? "—"}</dd>
          </div>
        </dl>
      ) : (
        <p className="text-sm text-fg-muted">{t("hardware.toolMissingHint", { name })}</p>
      )}
    </li>
  );
}

function ToolsBody({ report }: { report: HardwareReport }) {
  const { t } = useTranslation();
  const inset = useRowInset();
  return (
    <>
      <SectionRows>
        <ToolRow name="ffmpeg" tool={report.ffmpeg} />
        <ToolRow name="ffprobe" tool={report.ffprobe} />
      </SectionRows>
      <div className={cn("grid gap-x-6 gap-y-2 border-t border-line py-3 md:grid-cols-[7rem_minmax(0,1fr)]", inset)}>
        <span className="text-sm text-fg-muted">{t("hardware.hwaccels")}</span>
        {report.hwaccels.length > 0 ? (
          <ul className="flex flex-wrap gap-1.5">
            {report.hwaccels.map((accel) => (
              <li key={accel}>
                <Chip size="sm" variant="soft" className="bg-sunken text-fg">
                  {accel}
                </Chip>
              </li>
            ))}
          </ul>
        ) : (
          <span className="text-sm text-fg-muted">{t("common.none")}</span>
        )}
      </div>
    </>
  );
}

/* ---------------------------------------------------------------- encoder matrix */

function EncoderCell({ encoder }: { encoder: EncoderCapability }) {
  const { t } = useTranslation();
  const status = encoderStatus(encoder);
  return (
    <div data-status={status} className="grid min-w-0 gap-0.5" title={encoder.error ?? undefined}>
      <StatusLabel tone={ENCODER_TONE[status]} className="text-sm">
        {t(`hardware.statusShort.${status}`)}
      </StatusLabel>
      <span className="truncate text-xs text-fg-muted">{encoder.id}</span>
    </div>
  );
}

function Legend() {
  const { t } = useTranslation();
  const inset = useRowInset();
  return (
    <ul className={cn("flex flex-wrap gap-x-5 gap-y-1.5 pb-3 text-sm", inset)}>
      {ENCODER_STATUSES.map((status) => (
        <li key={status} className="flex items-center gap-1.5">
          <StatusLabel tone={ENCODER_TONE[status]}>{t(`hardware.statusShort.${status}`)}</StatusLabel>
          <span className="text-fg-muted">{t(`hardware.explain.${status}`)}</span>
        </li>
      ))}
    </ul>
  );
}

function EncoderMatrix({ encoders }: { encoders: EncoderCapability[] }) {
  const { t } = useTranslation();
  const inset = useRowInset();
  const variant = useSectionVariant();
  const methods = [...new Set(encoders.map((encoder) => encoder.method))].sort((a, b) => orderIndex(METHOD_ORDER, a) - orderIndex(METHOD_ORDER, b));
  const codecs = [...new Set(encoders.map((encoder) => encoder.codec))].sort(
    (a, b) => orderIndex(CODEC_ORDER, a) - orderIndex(CODEC_ORDER, b) || a.localeCompare(b),
  );
  if (encoders.length === 0) return <p className={cn("pb-4 text-sm text-fg-muted", inset)}>{t("hardware.noEncoders")}</p>;
  const edge = variant === "panel" ? "first:ps-(--pv-pad-panel) last:pe-(--pv-pad-panel)" : variant === "plain" ? "first:ps-3 last:pe-3" : "first:ps-0 last:pe-0";
  return (
    <>
      <Legend />
      <div className="overflow-x-auto">
        <table className="w-full min-w-[40rem] table-fixed border-collapse text-sm">
          <caption className="sr-only">{t("hardware.matrixCaption")}</caption>
          <thead>
            <tr className="border-y border-line text-xs text-fg-muted">
              <th scope="col" className={cn("w-40 py-2 pe-3 text-start font-mid", edge)}>
                {t("hardware.method")}
              </th>
              {codecs.map((codec) => (
                <th key={codec} scope="col" className={cn("px-3 py-2 text-start font-mid", edge)}>
                  {codecLabel(codec)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {methods.map((method, index) => {
              const rowEncoders = encoders.filter((encoder) => encoder.method === method);
              return (
                <tr key={method} className={cn("align-top", variant === "plain" ? index % 2 === 1 && "bg-sunken" : "border-b border-line last:border-b-0")}>
                  <th scope="row" className={cn("py-3 pe-3 text-start font-strong text-fg", edge)}>
                    {t(`hardware.methods.${method}`)}
                  </th>
                  {codecs.map((codec) => {
                    const cell = rowEncoders.filter((encoder) => encoder.codec === codec);
                    return (
                      <td key={codec} className={cn("px-3 py-3", edge)}>
                        {cell.length === 0 ? (
                          <>
                            <span aria-hidden className="text-fg-subtle">
                              —
                            </span>
                            <span className="sr-only">{t("common.none")}</span>
                          </>
                        ) : (
                          <div className="grid gap-2.5">
                            {cell.map((encoder) => (
                              <EncoderCell key={encoder.id} encoder={encoder} />
                            ))}
                          </div>
                        )}
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </>
  );
}

function FailureRows({ failed }: { failed: EncoderCapability[] }) {
  const { t } = useTranslation();
  const inset = useRowInset();
  return (
    <>
      <p className={cn("pb-3 text-sm text-fg-muted", inset)}>{t("hardware.failuresDetail")}</p>
      <SectionRows>
        {failed.map((encoder) => (
          <li key={encoder.id} className={cn("grid gap-x-6 gap-y-1 py-3 sm:grid-cols-[13rem_minmax(0,1fr)]", inset)}>
            <span className="min-w-0">
              <span className="block truncate font-strong text-fg">{encoder.id}</span>
              <span className="block text-sm text-fg-muted">
                {t(`hardware.methods.${encoder.method}`)} · {codecLabel(encoder.codec)}
              </span>
            </span>
            <p className="flex min-w-0 items-start gap-2 text-sm break-words text-fg">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-degraded" aria-hidden />
              {encoder.error ?? t("hardware.noErrorSummary")}
            </p>
          </li>
        ))}
      </SectionRows>
    </>
  );
}

/* ---------------------------------------------------------------- page */

export function HardwarePage() {
  const { t } = useTranslation();
  const query = useHardware();
  const report = query.data;
  const refresh = useRefreshHardware();
  const failed = report?.encoders.filter((encoder) => encoderStatus(encoder) === "listed") ?? [];
  const verified = report?.encoders.filter((encoder) => encoder.verified).length ?? 0;
  const listed = report?.encoders.filter((encoder) => encoder.listed).length ?? 0;

  return (
    <PageFrame>
      <PageHeader
        title={t("nav.hardware")}
        summary={report ? <RelativeTime value={report.probedAt} format={(time) => t("hardware.probedAt", { time })} /> : undefined}
        actions={
          <Button
            variant="outline"
            size="sm"
            isPending={refresh.isPending}
            isDisabled={query.isPending}
            onPress={() =>
              refresh.mutate(undefined, {
                onSuccess: () => toast.success(t("hardware.refreshed")),
                onError: (error) => toast.danger(errorText(t, error)),
              })
            }
          >
            {refresh.isPending ? <Spinner size="sm" color="current" /> : <RefreshCw aria-hidden />}
            {refresh.isPending ? t("hardware.refreshing") : t("hardware.refresh")}
          </Button>
        }
      />
      {query.isPending ? (
        <LoadingRows rows={5} />
      ) : !report ? (
        <ErrorState error={query.error} onRetry={() => void query.refetch()} />
      ) : (
        <>
          <Summary report={report} />
          <Section title={t("hardware.tools")}>
            <ToolsBody report={report} />
          </Section>
          <Section
            title={t("hardware.matrix")}
            actions={
              <span data-num className="text-sm text-fg-muted tabular-nums">
                {t("hardware.matrixCounts", { verified, listed, total: report.encoders.length })}
              </span>
            }
          >
            <EncoderMatrix encoders={report.encoders} />
          </Section>
          {failed.length > 0 ? (
            <Section title={t("hardware.failures", { count: failed.length })}>
              <FailureRows failed={failed} />
            </Section>
          ) : null}
        </>
      )}
    </PageFrame>
  );
}
