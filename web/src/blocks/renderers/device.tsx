/**
 * Device cards (家居): one tile per device with its state in words and the controls its type
 * allows. Controls only report intents (patches); the host calls the API. A media player that
 * shows a cover takes the work's colour in skins whose `itemColor` slot is on.
 */
import { Button, Slider, Switch } from "@heroui/react";
import { AirVent, CloudOff, Lamp, Lightbulb, Minus, Pause, Play, Plus, Power, Radar, Sun, SunDim, Tv, type LucideIcon } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { Measure } from "@/ui/marks";
import { ProgressBar } from "@/ui/progress";
import { Cover, useWorkColor } from "@/ui/work";
import type { DeviceInfo, DevicePatch, DeviceState } from "../types";

type Of<T extends DeviceState["type"]> = Extract<DeviceState, { type: T }>;

const ICONS: Record<DeviceState["type"], LucideIcon> = {
  light: Lightbulb,
  switch: Power,
  climate: AirVent,
  sensor: Radar,
  media_player: Tv,
};

const TARGET_MIN = 16;
const TARGET_MAX = 30;

function isOn(state: DeviceState): boolean {
  switch (state.type) {
    case "light":
    case "switch":
    case "climate":
      return state.on;
    case "media_player":
      return state.status === "playing" || state.status === "paused";
    case "sensor":
      return true;
  }
}

/** A power switch with no visible label (the card's name labels it). */
function PowerSwitch({ label, isSelected, isDisabled, onChange }: { label: string; isSelected: boolean; isDisabled: boolean; onChange: (on: boolean) => void }) {
  return (
    <Switch aria-label={label} isSelected={isSelected} isDisabled={isDisabled} onChange={onChange} className="shrink-0">
      <Switch.Content>
        <Switch.Control>
          <Switch.Thumb />
        </Switch.Control>
      </Switch.Content>
    </Switch>
  );
}

function Offline() {
  const { t } = useTranslation();
  return (
    <span className="inline-flex items-center gap-1.5 font-mid text-fg-muted">
      <CloudOff className="size-3.5 shrink-0" aria-hidden />
      {t("blocks.device.offline")}
    </span>
  );
}

function StateLine({ state }: { state: DeviceState }) {
  const { t } = useTranslation();
  switch (state.type) {
    case "light":
      return <>{state.on ? t("blocks.device.lightOn", { brightness: state.brightness }) : t("blocks.device.off")}</>;
    case "switch":
      return state.on ? (
        <>
          {t("blocks.device.on")}
          {state.power !== undefined ? (
            <>
              <span aria-hidden> · </span>
              <Measure value={state.power} unit="W" />
            </>
          ) : null}
        </>
      ) : (
        <>{t("blocks.device.off")}</>
      );
    case "climate":
      return (
        <>
          {state.on
            ? t("blocks.device.climate", { mode: t(`blocks.device.modes.${state.mode}`), current: state.current })
            : t("blocks.device.climateOff", { current: state.current })}
        </>
      );
    case "media_player":
      return <>{[t(`blocks.device.media.${state.status}`), state.app].filter(Boolean).join(" · ")}</>;
    case "sensor":
      return <>{t("blocks.device.sensor")}</>;
  }
}

const ACTIVE_DEVICE_TONE: Record<DeviceState["type"], { glow: string; border: string; chip: string; sliderAccent?: string }> = {
  light: {
    glow: "pv-glow-amber",
    border: "border-[oklch(0.74_0.15_74/0.45)]",
    chip: "bg-[oklch(0.70_0.16_72/0.18)] text-[oklch(0.54_0.16_64)] dark:bg-[oklch(0.78_0.16_75/0.22)] dark:text-[oklch(0.86_0.15_78)] shadow-[0_0_16px_-2px_oklch(0.76_0.16_75/0.45)]",
    sliderAccent: "[--pv-selected:oklch(0.74_0.16_74)]",
  },
  climate: {
    glow: "pv-glow-sky",
    border: "border-[oklch(0.66_0.13_214/0.45)]",
    chip: "bg-[oklch(0.58_0.13_214/0.18)] text-[oklch(0.46_0.13_214)] dark:bg-[oklch(0.66_0.13_214/0.22)] dark:text-[oklch(0.82_0.12_210)] shadow-[0_0_16px_-2px_oklch(0.66_0.13_214/0.45)]",
  },
  media_player: {
    glow: "pv-glow-emerald",
    border: "border-[oklch(0.64_0.14_156/0.42)]",
    chip: "bg-[oklch(0.54_0.14_156/0.18)] text-[oklch(0.44_0.14_156)] dark:bg-[oklch(0.66_0.14_156/0.22)] dark:text-[oklch(0.82_0.13_156)]",
  },
  switch: {
    glow: "pv-glow-blue",
    border: "border-[oklch(0.60_0.16_250/0.42)]",
    chip: "bg-[oklch(0.54_0.16_250/0.18)] text-[oklch(0.48_0.16_250)] dark:bg-[oklch(0.58_0.16_250/0.24)] dark:text-[oklch(0.80_0.14_248)]",
  },
  sensor: {
    glow: "pv-glow-sky",
    border: "border-line/75",
    chip: "bg-[oklch(0.56_0.12_214/0.14)] text-[oklch(0.48_0.12_214)] dark:bg-[oklch(0.62_0.12_212/0.18)] dark:text-[oklch(0.78_0.11_210)]",
  },
};

/** Card frame: icon, name and state on top, the device's own control below. */
function Card({
  device,
  icon: Icon,
  control,
  children,
  className,
}: {
  device: DeviceInfo;
  icon: LucideIcon;
  control?: ReactNode;
  children?: ReactNode;
  className?: string;
}) {
  const on = isOn(device.state);
  const lit = device.online && on;
  const tone = ACTIVE_DEVICE_TONE[device.state.type];
  return (
    <article
      aria-label={device.name}
      data-on={on || undefined}
      className={cn(
        "pv-glass-panel flex min-h-[10.5rem] min-w-0 flex-col gap-4 rounded-panel border p-4.5 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-raised",
        !device.online
          ? "opacity-75"
          : lit
            ? cn(tone.glow, tone.border, tone.sliderAccent)
            : "",
        className,
      )}
    >
      <div className="grid gap-3">
        <div className="flex min-h-10 items-center justify-between gap-3">
          <span
            className={cn(
              "grid size-10 shrink-0 place-items-center rounded-xl transition-all duration-200",
              lit ? tone.chip : "bg-sunken/85 text-fg-subtle",
            )}
          >
            <Icon className="size-5" strokeWidth={1.85} aria-hidden />
          </span>
          {control}
        </div>
        <div className="grid min-w-0 gap-0.5">
          <h3 className={cn("truncate text-base font-strong", device.online ? "text-heading" : "text-fg-muted")} title={device.name}>
            {device.name}
          </h3>
          <p className="truncate text-sm text-fg-muted">
            {device.online ? <StateLine state={device.state} /> : <Offline />}
            {device.room ? <span> · {device.room}</span> : null}
          </p>
        </div>
      </div>
      {children ? <div className="mt-auto">{children}</div> : null}
    </article>
  );
}

/** Brightness: follows the thumb while dragging, reports only the committed value. */
function Brightness({ device, state, disabled, onPatch }: { device: DeviceInfo; state: Of<"light">; disabled: boolean; onPatch: (patch: DevicePatch) => void }) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<number | null>(null);
  const first = (value: number | number[]) => (Array.isArray(value) ? (value[0] ?? state.brightness) : value);
  return (
    <div className="flex items-center gap-3">
      <SunDim className="size-4 shrink-0 text-fg-subtle" aria-hidden />
      <Slider
        aria-label={t("blocks.device.brightnessOf", { name: device.name })}
        value={draft ?? state.brightness}
        minValue={1}
        maxValue={100}
        isDisabled={disabled || !state.on}
        onChange={(value) => setDraft(first(value))}
        onChangeEnd={(value) => {
          setDraft(null);
          onPatch({ brightness: first(value) });
        }}
        className="min-w-0 flex-1"
      >
        <Slider.Track>
          <Slider.Fill />
          <Slider.Thumb />
        </Slider.Track>
      </Slider>
      <Sun className="size-4 shrink-0 text-fg-subtle" aria-hidden />
    </div>
  );
}

function TargetStepper({ state, disabled, onPatch }: { state: Of<"climate">; disabled: boolean; onPatch: (patch: DevicePatch) => void }) {
  const { t } = useTranslation();
  const off = disabled || !state.on;
  return (
    <div className="flex items-center justify-between gap-1 rounded-xl border border-line/65 bg-sunken/70 p-1">
      <Button isIconOnly size="sm" variant="ghost" isDisabled={off || state.target <= TARGET_MIN} onPress={() => onPatch({ target: state.target - 1 })} aria-label={t("blocks.device.cooler")}>
        <Minus aria-hidden />
      </Button>
      <span className="grid min-w-0 text-center leading-tight">
        <span className="truncate text-[0.6875rem] text-fg-muted">{t("blocks.device.target")}</span>
        <Measure value={state.target} unit="°C" className={cn("font-heading text-lg font-title", off ? "text-fg-muted" : "text-heading")} />
      </span>
      <Button isIconOnly size="sm" variant="ghost" isDisabled={off || state.target >= TARGET_MAX} onPress={() => onPatch({ target: state.target + 1 })} aria-label={t("blocks.device.warmer")}>
        <Plus aria-hidden />
      </Button>
    </div>
  );
}

function SensorReadings({ state }: { state: Of<"sensor"> }) {
  const { t } = useTranslation();
  return (
    <dl className="grid grid-cols-2 gap-x-4 gap-y-2">
      {state.readings.map((reading) => (
        <div key={reading.kind} className="grid min-w-0 gap-0.5">
          <dt className="truncate text-xs text-fg-muted">{t(`blocks.device.readings.${reading.kind}`)}</dt>
          <dd className="text-md font-strong text-fg">
            {typeof reading.value === "boolean" ? (
              t(`blocks.device.readings.${reading.kind}Value.${reading.value ? "yes" : "no"}`)
            ) : (
              <Measure value={reading.value} unit={reading.unit} />
            )}
          </dd>
        </div>
      ))}
    </dl>
  );
}

function PlayToggle({ state, disabled, onField, onPatch }: { state: Of<"media_player">; disabled: boolean; onField: boolean; onPatch: (patch: DevicePatch) => void }) {
  const { t } = useTranslation();
  if (state.status !== "playing" && state.status !== "paused") return null;
  const playing = state.status === "playing";
  return (
    <Button
      isIconOnly
      size="sm"
      variant="secondary"
      className={cn("shrink-0", onField && "button--on-field")}
      isDisabled={disabled}
      onPress={() => onPatch({ status: playing ? "paused" : "playing" })}
      aria-label={playing ? t("blocks.device.pause") : t("blocks.device.play")}
    >
      {playing ? <Pause aria-hidden /> : <Play aria-hidden />}
    </Button>
  );
}

/** "1:12:08 / 2:04:30" → position and duration. */
function timesOf(subtitle: string | null): [string, string] | null {
  const [position, duration] = (subtitle ?? "").split(" / ");
  return position && duration ? [position, duration] : null;
}

function MediaCard({ device, state, disabled, onPatch }: { device: DeviceInfo; state: Of<"media_player">; disabled: boolean; onPatch: (patch: DevicePatch) => void }) {
  const { t } = useTranslation();
  const work = useWorkColor(device.online ? state.artworkId : null, state.title);
  const times = timesOf(state.subtitle);
  const fraction = state.progress === undefined ? null : Math.min(1, Math.max(0, state.progress));
  const progressLabel = t("blocks.device.playbackOf", { name: state.title ?? device.name });

  if (work && state.title) {
    return (
      <article aria-label={device.name} className="work-field col-span-full flex min-w-0 gap-4 rounded-tile p-4 sm:col-span-2" style={work.style}>
        <Cover artworkId={state.artworkId} title={state.title} decorative className="h-[5.75rem] w-[3.875rem] shrink-0 rounded-[4px] shadow-art" />
        <div className="grid min-w-0 flex-1 content-start gap-1">
          <p className="flex min-w-0 items-center gap-1.5 text-sm text-(--work-field-sub)">
            <Tv className="size-3.5 shrink-0" aria-hidden />
            <span className="truncate">
              {device.name}
              <span aria-hidden> · </span>
              <StateLine state={state} />
            </span>
          </p>
          <div className="flex items-center gap-3">
            <h3 className="min-w-0 flex-1 truncate font-heading text-xl font-title">{state.title}</h3>
            <PlayToggle state={state} disabled={disabled} onField onPatch={onPatch} />
          </div>
          {fraction !== null ? (
            <div className="mt-1.5 grid gap-1.5">
              <div role="progressbar" aria-label={progressLabel} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(fraction * 100)} className="h-1 overflow-hidden rounded-full bg-current/25">
                <div className="h-full rounded-full bg-current" style={{ width: `${fraction * 100}%` }} />
              </div>
              {times ? (
                <div className="flex justify-between text-xs text-(--work-field-sub)">
                  <Measure value={times[0]} />
                  <Measure value={times[1]} />
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      </article>
    );
  }

  return (
    <Card device={device} icon={Tv} className="col-span-full sm:col-span-2" control={<PlayToggle state={state} disabled={disabled} onField={false} onPatch={onPatch} />}>
      {state.title ? (
        <div className="grid gap-2">
          <div className="flex min-w-0 items-center gap-3">
            {state.artworkId ? <Cover artworkId={state.artworkId} title={state.title} decorative className="h-12 w-8 shrink-0 rounded-[3px] shadow-art" /> : null}
            <p className="min-w-0 flex-1 truncate text-md font-strong text-fg">{state.title}</p>
          </div>
          {fraction !== null ? (
            <div className="flex items-center gap-3">
              <ProgressBar value={fraction} label={progressLabel} tone="neutral" className="flex-1" />
              {times ? <Measure value={`${times[0]} / ${times[1]}`} className="text-xs text-fg-muted" /> : null}
            </div>
          ) : null}
        </div>
      ) : null}
    </Card>
  );
}

export function DeviceCard({ device, onPatch, readOnly = false }: { device: DeviceInfo; onPatch: (patch: DevicePatch) => void; readOnly?: boolean }) {
  const { t } = useTranslation();
  const { state } = device;
  const disabled = readOnly || !device.online;
  const power = (on: boolean) => (
    <PowerSwitch label={t("blocks.device.toggle", { name: device.name })} isSelected={on} isDisabled={disabled} onChange={(next) => onPatch({ on: next })} />
  );
  switch (state.type) {
    case "light":
      return (
        <Card device={device} icon={state.colorTemp !== undefined ? Lightbulb : Lamp} control={power(state.on)}>
          <Brightness device={device} state={state} disabled={disabled} onPatch={onPatch} />
        </Card>
      );
    case "switch":
      return <Card device={device} icon={ICONS.switch} control={power(state.on)} />;
    case "climate":
      return (
        <Card device={device} icon={ICONS.climate} control={power(state.on)}>
          <TargetStepper state={state} disabled={disabled} onPatch={onPatch} />
        </Card>
      );
    case "sensor":
      return (
        <Card device={device} icon={ICONS.sensor}>
          {device.online ? <SensorReadings state={state} /> : null}
        </Card>
      );
    case "media_player":
      return <MediaCard device={device} state={state} disabled={disabled} onPatch={onPatch} />;
  }
}
