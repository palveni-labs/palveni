/**
 * 家居 (PROVISIONAL, feature "home"): scenes, every device by room as device blocks, and the
 * "影音联动" rules that let playback drive the house.
 */
import { Button, Spinner, Switch, toast, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Check, CornerDownRight, House, Play, Settings2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { cn } from "@/lib/cn";
import { BlockRenderer } from "@/blocks/block-renderer";
import type { Block } from "@/blocks/types";
import { RelativeTime } from "@/components/relative-time";
import { errorText } from "@/lib/errors";
import { useActivateScene, useHome, useSetRule } from "@/provisional/queries/home";
import type { AutomationRule, HomeDevice, Scene } from "@/provisional/types";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { HomeManageDrawer } from "./manage-drawer";
import { SCENE_ICONS } from "./scene-icons";
import { useDeviceAction } from "./use-device-action";

const ALL_ROOMS = "all";

function isOn(device: HomeDevice): boolean {
  const state = device.state;
  if (state.type === "sensor") return false;
  if (state.type === "media_player") return state.status === "playing";
  return state.on;
}

/* ---------------------------------------------------------------- scenes */

const SCENE_STYLE: Record<
  Scene["icon"],
  { glow: string; idleChip: string; activeChip: string; activeBorder: string; activeText: string }
> = {
  movie: {
    glow: "pv-glow-amber",
    idleChip: "bg-[oklch(0.68_0.15_70/0.14)] text-[oklch(0.55_0.15_64)] dark:bg-[oklch(0.76_0.16_74/0.18)] dark:text-[oklch(0.84_0.14_76)]",
    activeChip: "bg-[oklch(0.62_0.15_68)] text-white shadow-[0_6px_16px_-4px_oklch(0.68_0.15_68/0.55)]",
    activeBorder: "data-[selected=true]:border-[oklch(0.72_0.15_70/0.65)]",
    activeText: "text-[oklch(0.52_0.15_64)] dark:text-[oklch(0.84_0.14_76)]",
  },
  home: {
    glow: "pv-glow-blue",
    idleChip: "bg-[oklch(0.54_0.16_250/0.14)] text-[oklch(0.48_0.16_250)] dark:bg-[oklch(0.56_0.16_250/0.20)] dark:text-[oklch(0.78_0.14_248)]",
    activeChip: "pv-nav-pill-active text-white",
    activeBorder: "data-[selected=true]:border-[oklch(0.62_0.16_248/0.7)]",
    activeText: "text-active",
  },
  night: {
    glow: "pv-glow-sky",
    idleChip: "bg-[oklch(0.56_0.13_214/0.14)] text-[oklch(0.46_0.13_214)] dark:bg-[oklch(0.62_0.13_212/0.18)] dark:text-[oklch(0.80_0.12_210)]",
    activeChip: "bg-[oklch(0.54_0.13_214)] text-white shadow-[0_6px_16px_-4px_oklch(0.56_0.13_214/0.55)]",
    activeBorder: "data-[selected=true]:border-[oklch(0.66_0.13_212/0.65)]",
    activeText: "text-[oklch(0.45_0.13_214)] dark:text-[oklch(0.80_0.12_210)]",
  },
  morning: {
    glow: "pv-glow-orange",
    idleChip: "bg-[oklch(0.66_0.16_48/0.14)] text-[oklch(0.52_0.16_45)] dark:bg-[oklch(0.72_0.16_50/0.18)] dark:text-[oklch(0.82_0.14_54)]",
    activeChip: "bg-[oklch(0.58_0.16_46)] text-white shadow-[0_6px_16px_-4px_oklch(0.64_0.16_46/0.55)]",
    activeBorder: "data-[selected=true]:border-[oklch(0.70_0.16_48/0.65)]",
    activeText: "text-[oklch(0.50_0.16_45)] dark:text-[oklch(0.82_0.14_54)]",
  },
  away: {
    glow: "pv-glow-emerald",
    idleChip: "bg-[oklch(0.54_0.13_156/0.14)] text-[oklch(0.44_0.13_156)] dark:bg-[oklch(0.66_0.14_156/0.18)] dark:text-[oklch(0.80_0.13_156)]",
    activeChip: "bg-[oklch(0.50_0.14_156)] text-white shadow-[0_6px_16px_-4px_oklch(0.54_0.14_156/0.55)]",
    activeBorder: "data-[selected=true]:border-[oklch(0.66_0.14_156/0.65)]",
    activeText: "text-ok",
  },
};

function Scenes({ scenes }: { scenes: Scene[] }) {
  const { t } = useTranslation();
  const activate = useActivateScene();
  const active = scenes.find((scene) => scene.active);
  const run = (id: string) => {
    const scene = scenes.find((entry) => entry.id === id);
    if (!scene || scene.active) return;
    activate.mutate(id, {
      onSuccess: () => toast.success(t("home.sceneActivated", { name: scene.name })),
      onError: (error) => toast.danger(errorText(t, error)),
    });
  };
  return (
    <Section title={t("home.scenes")} unframed>
      <ToggleButtonGroup
        aria-label={t("home.scenes")}
        selectionMode="single"
        isDetached
        selectedKeys={active ? [active.id] : []}
        onSelectionChange={(keys) => {
          const [id] = [...keys];
          if (id !== undefined) run(String(id));
        }}
        isDisabled={activate.isPending}
        className="grid w-full grid-cols-[repeat(auto-fill,minmax(min(100%,clamp(9.75rem,14cqi,12.25rem)),1fr))] gap-3"
      >
        {scenes.map((scene) => {
          const Icon = SCENE_ICONS[scene.icon];
          const tone = SCENE_STYLE[scene.icon];
          const pending = activate.isPending && activate.variables === scene.id;
          return (
            <ToggleButton
              key={scene.id}
              id={scene.id}
              className={cn(
                "pv-glass-panel h-auto min-h-[4.5rem] w-full justify-start gap-3.5 rounded-panel border px-4 py-3.5 text-start whitespace-normal transition-all duration-200 hover:-translate-y-0.5",
                "[--toggle-button-bg-hover:color-mix(in_oklab,var(--pv-surface)_86%,transparent)] [--toggle-button-bg-pressed:color-mix(in_oklab,var(--pv-surface)_86%,transparent)] [--toggle-button-bg:color-mix(in_oklab,var(--pv-surface)_74%,transparent)] [--toggle-button-fg:var(--pv-fg)]",
                "[--toggle-button-bg-selected-hover:color-mix(in_oklab,var(--pv-active-soft)_92%,transparent)] [--toggle-button-bg-selected-pressed:color-mix(in_oklab,var(--pv-active-soft)_92%,transparent)] [--toggle-button-bg-selected:color-mix(in_oklab,var(--pv-active-soft)_88%,transparent)] [--toggle-button-fg-selected:var(--pv-fg)]",
                tone.activeBorder,
                scene.active && tone.glow,
                "data-[selected=true]:shadow-raised",
              )}
            >
              <span
                className={cn(
                  "grid size-10 shrink-0 place-items-center rounded-xl transition-all duration-200",
                  scene.active ? tone.activeChip : tone.idleChip,
                )}
              >
                {pending ? <Spinner size="sm" color="current" /> : <Icon className="size-4.5" strokeWidth={1.9} aria-hidden />}
              </span>
              <span className="grid min-w-0 gap-0.5">
                <span className="truncate text-base font-strong text-heading">{scene.name}</span>
                {scene.active ? (
                  <span className={cn("flex items-center gap-1 text-xs font-strong", tone.activeText)}>
                    <Check className="size-3.5" strokeWidth={2.5} aria-hidden />
                    {t("home.sceneActive")}
                  </span>
                ) : null}
              </span>
            </ToggleButton>
          );
        })}
      </ToggleButtonGroup>
    </Section>
  );
}

/* ---------------------------------------------------------------- automation rules */

function RuleRow({ rule }: { rule: AutomationRule }) {
  const { t } = useTranslation();
  const inset = useRowInset();
  const setRule = useSetRule();
  const toggle = (enabled: boolean) =>
    setRule.mutate({ id: rule.id, enabled }, { onError: (error) => toast.danger(errorText(t, error)) });
  return (
    <li className={cn("flex items-start gap-3 py-3.5 transition-colors hover:bg-hover/40", inset)}>
      <span
        className={cn(
          "mt-0.5 grid size-8 shrink-0 place-items-center rounded-xl transition-colors",
          rule.enabled
            ? "bg-[oklch(0.54_0.16_250/0.16)] text-[oklch(0.48_0.16_250)] dark:bg-[oklch(0.56_0.16_250/0.22)] dark:text-[oklch(0.78_0.14_248)]"
            : "bg-sunken text-fg-subtle",
        )}
      >
        <Play className="size-3.5" aria-hidden />
      </span>
      <div className="grid min-w-0 flex-1 gap-1">
        <p className={cn("text-sm leading-snug", rule.enabled ? "font-strong text-heading" : "font-mid text-fg-muted")}>
          {rule.trigger}
        </p>
        <p className="flex items-start gap-1.5 text-xs text-fg-muted">
          <CornerDownRight className="mt-0.5 size-3 shrink-0 text-fg-subtle" aria-hidden />
          <span className="min-w-0">{rule.action}</span>
        </p>
        <p className="text-[0.75rem] text-fg-subtle">
          {!rule.enabled ? (
            t("home.ruleOff")
          ) : rule.lastRunAt ? (
            <RelativeTime value={rule.lastRunAt} format={(time) => t("home.lastRun", { time })} />
          ) : (
            t("home.neverRun")
          )}
        </p>
      </div>
      <Switch
        aria-label={t("home.ruleToggle", { trigger: rule.trigger })}
        isSelected={rule.enabled}
        isDisabled={setRule.isPending}
        onChange={toggle}
        className="mt-1 shrink-0"
      >
        <Switch.Content>
          <Switch.Control>
            <Switch.Thumb />
          </Switch.Control>
        </Switch.Content>
      </Switch>
    </li>
  );
}

function Rules({ rules }: { rules: AutomationRule[] }) {
  const { t } = useTranslation();
  const enabled = rules.filter((rule) => rule.enabled).length;
  return (
    <Section title={t("home.rules")} count={enabled}>
      {rules.length === 0 ? (
        <EmptyState compact icon={Play} title={t("home.rulesEmpty")} detail={t("home.rulesEmptyDetail")} />
      ) : (
        <SectionRows>
          {rules.map((rule) => (
            <RuleRow key={rule.id} rule={rule} />
          ))}
        </SectionRows>
      )}
    </Section>
  );
}

/* ---------------------------------------------------------------- page */

export function HomePage() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const [manageOpen, setManageOpen] = useState(false);
  const { data: home, isPending, isError, error, refetch } = useHome(true);
  const onDevice = useDeviceAction();
  const navigate = useNavigate({ from: "/home/" });
  const room = useSearch({ from: "/_app/home/" }).room ?? ALL_ROOMS;
  const setRoom = (next: string) => void navigate({ search: { room: next === ALL_ROOMS ? undefined : next }, replace: true });

  if (isPending || isError || !home) {
    return (
      <PageFrame>
        <PageHeader title={t("modules.home.title")} />
        {isError ? <ErrorState error={error} onRetry={() => void refetch()} /> : <LoadingRows rows={6} label={t("home.loading")} />}
      </PageFrame>
    );
  }

  const rooms = home.rooms.filter((entry) => home.devices.some((device) => device.roomId === entry.id));
  // An unknown room (a stale link) shows every room.
  const picked = rooms.some((entry) => entry.id === room) ? room : ALL_ROOMS;
  const shown = picked === ALL_ROOMS ? rooms : rooms.filter((entry) => entry.id === picked);
  const countIn = (id: string) => home.devices.filter((device) => device.roomId === id).length;
  const offline = home.devices.filter((device) => !device.online).length;
  const summary = [
    t("home.summary", { rooms: home.rooms.length, devices: home.devices.length, on: home.devices.filter(isOn).length }),
    offline > 0 ? t("home.summaryOffline", { count: offline }) : null,
  ]
    .filter(Boolean)
    .join(" · ");

  const manageAction = isAdmin ? (
    <Button size="sm" variant="secondary" onPress={() => setManageOpen(true)}>
      <Settings2 className="size-4" aria-hidden />
      {t("home.manage.open")}
    </Button>
  ) : null;

  return (
    <PageFrame>
      <PageHeader title={t("modules.home.title")} summary={home.devices.length > 0 ? summary : undefined} actions={manageAction} />
      {home.devices.length === 0 ? (
        <EmptyState icon={House} title={t("home.empty")} detail={t("home.emptyDetail")} />
      ) : (
        <div className="grid items-start gap-(--pv-gap-section) xl:grid-cols-[minmax(0,1fr)_var(--pv-aside-w)]">
          <div className="grid min-w-0 gap-(--pv-gap-section)">
            {home.scenes.length > 0 ? <Scenes scenes={home.scenes} /> : null}
            <div className="grid min-w-0 gap-4">
              {rooms.length > 1 ? (
                <div className="no-scrollbar -mx-4 overflow-x-auto px-4 md:mx-0 md:px-0">
                  <ToggleButtonGroup
                    aria-label={t("home.roomFilter")}
                    selectionMode="single"
                    disallowEmptySelection
                    selectedKeys={[picked]}
                    onSelectionChange={(keys) => setRoom(String([...keys][0] ?? ALL_ROOMS))}
                    className="pv-segmented"
                  >
                    {[{ id: ALL_ROOMS, name: t("home.allRooms") }, ...rooms].map((entry) => (
                      <ToggleButton key={entry.id} id={entry.id} className="shrink-0 whitespace-nowrap">
                        {entry.name}
                        <span className="text-fg-muted tabular-nums">{entry.id === ALL_ROOMS ? home.devices.length : countIn(entry.id)}</span>
                      </ToggleButton>
                    ))}
                  </ToggleButtonGroup>
                </div>
              ) : null}
              {shown.map((entry) => {
                const blocks: Block[] = home.devices
                  .filter((device) => device.roomId === entry.id)
                  .map((device) => ({ type: "device", device: { ...device, room: undefined } }));
                return (
                  <Section key={entry.id} title={entry.name} count={blocks.length} unframed>
                    <BlockRenderer blocks={blocks} onAction={onDevice} />
                  </Section>
                );
              })}
            </div>
          </div>
          <Rules rules={home.rules} />
        </div>
      )}
      {isAdmin ? (
        <HomeManageDrawer open={manageOpen} onOpenChange={setManageOpen} scenes={home.scenes} devices={home.devices} />
      ) : null}
    </PageFrame>
  );
}
