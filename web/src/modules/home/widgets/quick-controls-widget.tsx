import { Button, Chip, Dropdown, Label, Slider, Switch } from "@heroui/react";
import { AirVent, Check, ChevronDown, Lamp as LampIcon, Lightbulb, Minus, Pause, Play, Plus, Tv, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { DevicePatch } from "@/blocks/types";
import { useActivateScene, useHome, useUpdateDevice } from "@/provisional/queries/home";
import type { HomeDevice } from "@/provisional/types";
import { useSlot } from "@/skins/scope";
import { Measure } from "@/ui/marks";
import { useRowInset } from "@/ui/section";
import { EmptyState, LoadingRows } from "@/ui/states";
import { Cover, useWorkColor } from "@/ui/work";
import { SCENE_ICONS } from "../scene-icons";

function Row({ icon: Icon, name, detail, control, className }: { icon: LucideIcon; name: string; detail?: ReactNode; control: ReactNode; className?: string }) {
  const inset = useRowInset();
  return (
    <li className={cn("flex min-h-14 items-center gap-3 py-2.5", inset, className)}>
      <Icon className="size-[1.15rem] shrink-0 text-icon" aria-hidden />
      <div className="grid min-w-0 flex-1">
        <span className="truncate text-base font-mid text-fg">{name}</span>
        {detail ? <span className="truncate text-sm text-fg-muted">{detail}</span> : null}
      </div>
      <div className="flex shrink-0 items-center gap-2.5">{control}</div>
    </li>
  );
}

function NowPlaying({ device, onPatch }: { device: HomeDevice; onPatch: (patch: DevicePatch) => void }) {
  const { t } = useTranslation();
  const progressSlot = useSlot("progress");
  const inset = useRowInset();
  const media = device.state.type === "media_player" ? device.state : null;
  const work = useWorkColor(media?.artworkId, media?.title);
  if (!media) return null;
  const state = media;
  const playing = state.status === "playing";
  const [position, duration] = (state.subtitle ?? "").split(" / ");
  const toggle = (
    <Button
      isIconOnly
      size="sm"
      variant={work ? "secondary" : "ghost"}
      className={cn(work && "button--on-field")}
      onPress={() => onPatch({ status: playing ? "paused" : "playing" })}
      aria-label={playing ? t("blocks.device.pause") : t("blocks.device.play")}
    >
      {playing ? <Pause aria-hidden /> : <Play aria-hidden />}
    </Button>
  );
  const fraction = Math.min(1, Math.max(0, state.progress ?? 0));
  if (work) {
    return (
      <li className="work-field grid grid-cols-[3.75rem_minmax(0,1fr)] gap-4 px-(--pv-pad-panel) py-[1.125rem]" style={work.style}>
        <Cover artworkId={state.artworkId} title={state.title ?? device.name} decorative className="h-[5.625rem] w-[3.75rem] rounded-[4px] shadow-art" />
        <div className="grid min-w-0 content-start">
          <span className="flex items-center gap-1.5 text-sm text-(--work-field-sub)">
            <Tv className="size-3.5" aria-hidden />
            {device.name} · {t(`blocks.device.media.${state.status}`)}
          </span>
          <div className="mt-0.5 mb-3 flex items-center gap-2.5">
            <span className="min-w-0 flex-1 truncate text-xl font-title">{state.title}</span>
            {toggle}
          </div>
          <div role="progressbar" aria-label={state.title ?? device.name} aria-valuenow={Math.round(fraction * 100)} aria-valuemin={0} aria-valuemax={100} className="h-1 overflow-hidden rounded-full bg-current/25">
            <div className="h-full bg-current" style={{ width: `${fraction * 100}%` }} />
          </div>
          <div className="mt-1.5 flex justify-between text-xs text-(--work-field-sub)">
            <Measure value={position} />
            <Measure value={duration} />
          </div>
        </div>
      </li>
    );
  }
  return (
    <li className={cn("relative grid gap-2 py-3", inset)}>
      <div className="flex items-center gap-3">
        <Tv className="size-[1.15rem] shrink-0 text-icon" aria-hidden />
        <div className="grid min-w-0 flex-1">
          <span className="truncate text-base font-mid text-fg">{device.name}</span>
          <span className="truncate text-sm text-fg-muted">
            {t(`blocks.device.media.${state.status}`)} {state.title}
          </span>
        </div>
        {toggle}
      </div>
      <div className="flex items-center gap-3 ps-[1.9rem]">
        {progressSlot === "separator" ? (
          <div aria-hidden className="absolute inset-x-0 bottom-0 h-px bg-line">
            <div className="relative h-full bg-active-mark" style={{ width: `${fraction * 100}%` }}>
              <span className="absolute -top-[3px] right-0 h-[7px] w-[2px] bg-active-mark" />
            </div>
          </div>
        ) : (
          <div role="progressbar" aria-label={state.title ?? device.name} aria-valuenow={Math.round(fraction * 100)} aria-valuemin={0} aria-valuemax={100} className="h-1 flex-1 overflow-hidden rounded-full bg-progress-rest">
            <div className="h-full rounded-full bg-progress-done" style={{ width: `${fraction * 100}%` }} />
          </div>
        )}
        <Measure value={state.subtitle} className={cn("text-xs text-fg-muted", progressSlot === "separator" && "ms-auto")} />
      </div>
    </li>
  );
}

/** PROVISIONAL: the active scene and the living room's devices (feature "home"). */
export default function QuickControlsWidget() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const { data: home, isPending } = useHome(true);
  const activate = useActivateScene();
  const update = useUpdateDevice();
  if (isPending) return <LoadingRows rows={4} className="px-5 pb-4" />;
  if (!home) return <EmptyState compact icon={Lightbulb} title={t("home.empty")} />;
  const patch = (id: string) => (next: DevicePatch) => update.mutate({ id, patch: next });
  const scene = home.scenes.find((entry) => entry.active);
  const SceneIcon = scene ? SCENE_ICONS[scene.icon] : Lightbulb;
  const living = home.devices.filter((device) => device.roomId === "living" && device.state.type !== "sensor");
  const climateLabel = (device: HomeDevice) =>
    device.state.type === "climate" ? t("home.climateDetail", { mode: t(`blocks.device.modes.${device.state.mode}`), current: device.state.current }) : "";

  return (
    <ul className="divide-y divide-line">
      <li className={cn("flex items-center gap-3 pt-1 pb-3", inset)}>
        <SceneIcon className="size-[1.15rem] shrink-0 text-icon" aria-hidden />
        <span className="truncate font-heading text-md font-strong whitespace-nowrap text-fg">{scene?.name ?? t("home.noScene")}</span>
        {scene ? (
          <Chip size="sm" variant="soft" className="shrink-0 bg-ok-soft text-ok">
            {t("home.sceneInUse")}
          </Chip>
        ) : null}
        <span className="flex-1" />
        <Dropdown>
          <Button size="sm" variant="secondary" className="shrink-0">
            {t("home.switchScene")}
            <ChevronDown aria-hidden />
          </Button>
          <Dropdown.Popover placement="bottom end">
            <Dropdown.Menu aria-label={t("home.switchScene")} onAction={(key) => activate.mutate(String(key))}>
              {home.scenes.map((entry) => {
                const Icon = SCENE_ICONS[entry.icon];
                return (
                  <Dropdown.Item key={entry.id} id={entry.id} textValue={entry.name}>
                    <Icon className="size-4 text-fg-muted" aria-hidden />
                    <Label className="flex-1">{entry.name}</Label>
                    {entry.active ? <Check className="size-4" aria-label={t("common.selected")} /> : null}
                  </Dropdown.Item>
                );
              })}
            </Dropdown.Menu>
          </Dropdown.Popover>
        </Dropdown>
      </li>
      {living.map((device) => {
        const state = device.state;
        if (state.type === "media_player") return <NowPlaying key={device.id} device={device} onPatch={patch(device.id)} />;
        if (state.type === "light") {
          const dimmable = state.colorTemp !== undefined;
          return (
            <Row
              key={device.id}
              icon={dimmable ? Lightbulb : LampIcon}
              name={device.name}
              detail={dimmable && state.on ? t("blocks.device.lightOn", { brightness: state.brightness }) : !state.on ? t("blocks.device.off") : undefined}
              control={
                <>
                  {dimmable ? (
                    <Slider
                      aria-label={t("blocks.device.brightnessOf", { name: device.name })}
                      value={state.brightness}
                      minValue={1}
                      maxValue={100}
                      isDisabled={!state.on}
                      onChangeEnd={(value) => patch(device.id)({ brightness: Array.isArray(value) ? (value[0] ?? 0) : value })}
                      className="w-24"
                    >
                      <Slider.Track>
                        <Slider.Fill />
                        <Slider.Thumb />
                      </Slider.Track>
                    </Slider>
                  ) : null}
                  <Switch isSelected={state.on} onChange={(on) => patch(device.id)({ on })} aria-label={t("blocks.device.toggle", { name: device.name })}>
                    <Switch.Content>
                      <Switch.Control>
                        <Switch.Thumb />
                      </Switch.Control>
                    </Switch.Content>
                  </Switch>
                </>
              }
            />
          );
        }
        if (state.type === "climate") {
          return (
            <Row
              key={device.id}
              icon={AirVent}
              name={device.name}
              detail={climateLabel(device)}
              control={
                <div className="flex items-center gap-0.5 rounded-control bg-sunken p-0.5">
                  <Button isIconOnly size="sm" variant="ghost" onPress={() => patch(device.id)({ target: state.target - 1 })} aria-label={t("blocks.device.cooler")}>
                    <Minus aria-hidden />
                  </Button>
                  <Measure value={`${state.target}°C`} className="min-w-12 text-center font-strong text-fg" />
                  <Button isIconOnly size="sm" variant="ghost" onPress={() => patch(device.id)({ target: state.target + 1 })} aria-label={t("blocks.device.warmer")}>
                    <Plus aria-hidden />
                  </Button>
                </div>
              }
            />
          );
        }
        if (state.type === "switch") {
          return (
            <Row
              key={device.id}
              icon={LampIcon}
              name={device.name}
              control={
                <Switch isSelected={state.on} onChange={(on) => patch(device.id)({ on })} aria-label={t("blocks.device.toggle", { name: device.name })}>
                  <Switch.Content>
                    <Switch.Control>
                      <Switch.Thumb />
                    </Switch.Control>
                  </Switch.Content>
                </Switch>
              }
            />
          );
        }
        return null;
      })}
    </ul>
  );
}
