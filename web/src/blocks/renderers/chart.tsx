/**
 * Charts from data: a bar chart for magnitudes per category (one hue) and a donut for parts of
 * a whole (the skin's five categorical colours, a sixth series folds into "其他"). Every value is
 * written next to its mark, so colour is never the only key.
 */
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { formatBytes, formatNumber, formatPercent } from "@/lib/format";
import { Measure } from "@/ui/marks";
import type { ChartBlock } from "../types";
import { BlockTitle } from "./basic";

const STROKES = ["stroke-chart-1", "stroke-chart-2", "stroke-chart-3", "stroke-chart-4", "stroke-chart-5"] as const;
const SWATCHES = ["bg-chart-1", "bg-chart-2", "bg-chart-3", "bg-chart-4", "bg-chart-5"] as const;

function useValueFormat(unit: ChartBlock["unit"]) {
  const { i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  return {
    value: (value: number) => (unit === "bytes" ? formatBytes(value, locale) : formatNumber(value, locale)),
    share: (fraction: number) => formatPercent(fraction, locale),
  };
}

/** Magnitude per category: thin horizontal bars in one hue, each value written at its end. */
function BarChart({ block, label }: { block: ChartBlock; label: string }) {
  const format = useValueFormat(block.unit);
  const [hover, setHover] = useState<number | null>(null);
  const max = Math.max(1, ...block.series.map((entry) => entry.value));
  return (
    <ul aria-label={label} className="grid gap-2.5">
      {block.series.map((entry, index) => (
        <li
          key={entry.label}
          className="grid grid-cols-[minmax(4.5rem,9rem)_minmax(0,1fr)_auto] items-center gap-3 text-sm"
          onPointerEnter={() => setHover(index)}
          onPointerLeave={() => setHover(null)}
        >
          <span className="truncate text-fg-muted" title={entry.label}>
            {entry.label}
          </span>
          <span aria-hidden className="relative h-2 overflow-hidden rounded-full bg-progress-rest">
            <span
              className={cn("absolute inset-y-0 left-0 rounded-full bg-chart-1 transition-opacity duration-150", hover !== null && hover !== index && "opacity-40")}
              style={{ width: `${Math.max(1.5, (entry.value / max) * 100)}%` }}
            />
          </span>
          <Measure value={format.value(entry.value)} className="text-end font-mid text-fg" />
        </li>
      ))}
    </ul>
  );
}

/** Part-to-whole: slices separated by small gaps, the total in the middle, a legend with values. */
function DonutChart({ block, label }: { block: ChartBlock; label: string }) {
  const { t } = useTranslation();
  const format = useValueFormat(block.unit);
  const [hover, setHover] = useState<number | null>(null);
  const series =
    block.series.length > STROKES.length
      ? [
          ...block.series.slice(0, STROKES.length - 1),
          { label: t("blocks.chartOther"), value: block.series.slice(STROKES.length - 1).reduce((sum, entry) => sum + entry.value, 0) },
        ]
      : block.series;
  const total = series.reduce((sum, entry) => sum + entry.value, 0);
  const radius = 40;
  const circumference = 2 * Math.PI * radius;
  const lengths = series.map((entry) => (total > 0 ? (entry.value / total) * circumference : 0));
  const offsets = lengths.map((_, index) => lengths.slice(0, index).reduce((sum, length) => sum + length, 0));
  const drawn = series.filter((entry) => entry.value > 0).length;
  return (
    <div className="flex flex-wrap items-center gap-x-6 gap-y-4">
      <div className="relative size-32 shrink-0">
        <svg viewBox="0 0 100 100" className="size-full" role="img" aria-label={label}>
          <circle cx="50" cy="50" r={radius} fill="none" className="stroke-progress-rest" strokeWidth="13" />
          <g transform="rotate(-90 50 50)">
            {series.map((entry, index) => {
              const length = lengths[index] ?? 0;
              if (length <= 0) return null;
              const gap = drawn > 1 ? Math.min(1.8, length / 2) : 0;
              return (
                <circle
                  key={entry.label}
                  cx="50"
                  cy="50"
                  r={radius}
                  fill="none"
                  className={cn(STROKES[index], "transition-[stroke-width] duration-150")}
                  strokeWidth={hover === index ? 16 : 13}
                  strokeDasharray={`${Math.max(0, length - gap)} ${circumference}`}
                  strokeDashoffset={-(offsets[index] ?? 0)}
                  onPointerEnter={() => setHover(index)}
                  onPointerLeave={() => setHover(null)}
                >
                  <title>{`${entry.label}: ${format.value(entry.value)}`}</title>
                </circle>
              );
            })}
          </g>
        </svg>
        <div className="pointer-events-none absolute inset-0 grid place-content-center text-center">
          <Measure value={format.value(total)} className="text-md leading-tight font-strong text-fg" />
          <span className="text-xs text-fg-muted">{t("blocks.chartTotal")}</span>
        </div>
      </div>
      <ul className="grid min-w-[12rem] flex-1 gap-0.5 text-sm">
        {series.map((entry, index) => (
          <li
            key={entry.label}
            className={cn("flex items-center gap-2.5 rounded-control px-1.5 py-1 transition-colors", hover === index && "bg-hover")}
            onPointerEnter={() => setHover(index)}
            onPointerLeave={() => setHover(null)}
          >
            <span aria-hidden className={cn("size-2.5 shrink-0 rounded-[2px]", SWATCHES[index])} />
            <span className="min-w-0 flex-1 truncate text-fg">{entry.label}</span>
            <Measure value={format.value(entry.value)} className="font-mid text-fg" />
            <span className="w-10 text-end text-fg-muted tabular-nums">{total > 0 ? format.share(entry.value / total) : "—"}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function ChartBlockView({ block }: { block: ChartBlock }) {
  const { t } = useTranslation();
  const label = block.title ?? t("blocks.chart");
  return (
    <figure className="grid min-w-0 gap-3">
      {block.title ? (
        <figcaption>
          <BlockTitle>{block.title}</BlockTitle>
        </figcaption>
      ) : null}
      {block.kind === "donut" ? <DonutChart block={block} label={label} /> : <BarChart block={block} label={label} />}
    </figure>
  );
}
