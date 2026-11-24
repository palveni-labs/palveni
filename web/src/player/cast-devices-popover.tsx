/**
 * "Devices & Cast" popover listing active playback sessions across all of
 * the caller's devices with remote Play/Pause, Prev/Next, and Volume controls.
 */
import { Button, Popover, Spinner, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Cast,
  Laptop,
  Monitor,
  Pause,
  Play,
  RefreshCw,
  SkipBack,
  SkipForward,
  Smartphone,
  Tv,
  Volume2,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { queryKeys } from "@/api/query-keys";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import {
  getLocalPlaybackSessionId,
  listPlaybackSessions,
  sendPlaybackCommand,
  type PlaybackClientKind,
  type PlaybackCommandRequest,
  type PlaybackSession,
} from "./cast-sync";

function ClientIcon({ kind }: { kind: PlaybackClientKind }) {
  switch (kind) {
    case "ios":
    case "android":
      return <Smartphone className="size-4 shrink-0 text-fg-muted" aria-hidden />;
    case "tvos":
      return <Tv className="size-4 shrink-0 text-fg-muted" aria-hidden />;
    case "macos":
    case "desktop":
      return <Laptop className="size-4 shrink-0 text-fg-muted" aria-hidden />;
    default:
      return <Monitor className="size-4 shrink-0 text-fg-muted" aria-hidden />;
  }
}

function SessionDeviceRow({
  session,
  isSelf,
  onCommand,
  busy,
}: {
  session: PlaybackSession;
  isSelf: boolean;
  onCommand: (session: PlaybackSession, cmd: PlaybackCommandRequest) => void;
  busy: boolean;
}) {
  const { t } = useTranslation();
  const [draftVolume, setDraftVolume] = useState(session.volume);
  const [prevServerVolume, setPrevServerVolume] = useState(session.volume);

  if (session.volume !== prevServerVolume) {
    setPrevServerVolume(session.volume);
    setDraftVolume(session.volume);
  }

  const isPlaying = session.state === "playing";
  const stateLabel = t(`cast.states.${session.state}`);

  return (
    <li className="grid gap-2.5 rounded-tile border border-line bg-surface p-3">
      <div className="flex items-start justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <ClientIcon kind={session.clientKind} />
          <div className="min-w-0">
            <div className="flex items-center gap-1.5">
              <span className="truncate text-sm font-strong text-fg">{session.deviceName}</span>
              {isSelf ? (
                <span className="shrink-0 rounded-chip bg-selected/15 px-1.5 py-0.5 text-[0.65rem] font-strong text-fg">
                  {t("cast.thisDevice")}
                </span>
              ) : null}
            </div>
            <p className="truncate text-xs text-fg-muted">
              {session.title ? (
                <>
                  {session.title}
                  {session.subtitle ? ` · ${session.subtitle}` : ""}
                </>
              ) : (
                t("cast.idleTrack")
              )}
            </p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {session.ambientHex ? (
            <span
              aria-hidden
              title={session.ambientHex}
              className="size-2.5 rounded-full ring-1 ring-line"
              style={{ backgroundColor: session.ambientHex }}
            />
          ) : null}
          <span
            className={cn(
              "rounded-chip px-1.5 py-0.5 text-[0.68rem] font-mid",
              isPlaying ? "bg-action text-action-fg" : "bg-sunken text-fg-muted",
            )}
          >
            {stateLabel}
          </span>
        </div>
      </div>

      <div className="flex items-center justify-between gap-2 border-t border-line/70 pt-2">
        <div className="flex items-center gap-1">
          <button
            type="button"
            disabled={busy}
            aria-label={`${session.deviceName} ${t("cast.controls.previous")}`}
            onClick={() => onCommand(session, { command: "prev" })}
            className="grid size-7 place-items-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus) disabled:opacity-50"
          >
            <SkipBack className="size-3.5 fill-current" aria-hidden />
          </button>
          <button
            type="button"
            disabled={busy}
            aria-label={`${session.deviceName} ${isPlaying ? t("cast.controls.pause") : t("cast.controls.play")}`}
            onClick={() => onCommand(session, { command: isPlaying ? "pause" : "play" })}
            className="grid size-7 place-items-center rounded-full bg-action text-action-fg transition-colors hover:bg-action-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus) disabled:opacity-50"
          >
            {isPlaying ? (
              <Pause className="size-3.5 fill-current" aria-hidden />
            ) : (
              <Play className="ms-0.5 size-3.5 fill-current" aria-hidden />
            )}
          </button>
          <button
            type="button"
            disabled={busy}
            aria-label={`${session.deviceName} ${t("cast.controls.next")}`}
            onClick={() => onCommand(session, { command: "next" })}
            className="grid size-7 place-items-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus) disabled:opacity-50"
          >
            <SkipForward className="size-3.5 fill-current" aria-hidden />
          </button>
        </div>

        <div className="flex min-w-0 flex-1 items-center justify-end gap-1.5">
          <Volume2 className="size-3.5 shrink-0 text-fg-muted" aria-hidden />
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={draftVolume}
            disabled={busy}
            aria-label={`${session.deviceName} ${t("cast.controls.volume")}`}
            onChange={(event) => setDraftVolume(Number(event.target.value))}
            onMouseUp={() => onCommand(session, { command: "volume", volume: draftVolume })}
            onTouchEnd={() => onCommand(session, { command: "volume", volume: draftVolume })}
            onKeyUp={() => onCommand(session, { command: "volume", volume: draftVolume })}
            className="h-1.5 w-20 cursor-pointer accent-(--pv-action)"
          />
          <span data-num className="w-7 text-end text-[0.7rem] text-fg-muted tabular-nums">
            {draftVolume}%
          </span>
        </div>
      </div>
    </li>
  );
}

export function CastDevicesPopover({
  tone = "player",
  placement = "top end",
  forceEnabled,
}: {
  tone?: "shell" | "plain" | "player";
  placement?: "top end" | "bottom end";
  forceEnabled?: boolean;
}) {
  const { t } = useTranslation();
  const featureCast = useFeature("cast");
  const enabled = forceEnabled ?? featureCast;
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();
  const localSessionId = getLocalPlaybackSessionId();

  const sessionsQuery = useQuery({
    queryKey: queryKeys.cast.sessions,
    queryFn: ({ signal }) => listPlaybackSessions(signal),
    enabled: enabled && open,
    refetchInterval: open ? 15_000 : false,
    staleTime: 5_000,
  });

  const commandMutation = useMutation({
    mutationFn: ({ id, body }: { id: string; body: PlaybackCommandRequest }) => sendPlaybackCommand(id, body),
    onSuccess: (_ack, vars) => {
      const target = sessionsQuery.data?.sessions.find((s) => s.id === vars.id);
      if (target) {
        toast.success(t("cast.controls.commandSent", { device: target.deviceName }));
      }
      void queryClient.invalidateQueries({ queryKey: queryKeys.cast.sessions });
    },
    onError: (error) => toast.danger(errorText(t, error)),
  });

  if (!enabled) return null;

  const sessions = sessionsQuery.data?.sessions ?? [];
  const activeCount = sessions.length;

  const triggerClass =
    tone === "shell"
      ? "relative grid size-8 place-items-center rounded-control text-shell-fg-muted transition-colors hover:bg-shell-field hover:text-shell-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
      : tone === "plain"
        ? "relative grid size-8 place-items-center rounded-control text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        : "relative inline-flex h-8 items-center gap-1 rounded-control px-2 text-xs text-fg-muted transition-colors hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)";

  return (
    <Popover isOpen={open} onOpenChange={setOpen}>
      <Popover.Trigger aria-label={t("cast.open")} className={triggerClass}>
        <Cast className="size-4" aria-hidden />
        {tone === "player" && activeCount > 0 ? (
          <span data-num className="tabular-nums">
            {activeCount}
          </span>
        ) : null}
      </Popover.Trigger>
      <Popover.Content placement={placement} className="w-80 max-w-[calc(100vw-1.5rem)] p-3">
        <Popover.Dialog aria-label={t("cast.title")} className="grid gap-3 outline-none">
          <div className="flex items-center justify-between gap-2">
            <h2 className="text-sm font-strong text-heading">{t("cast.title")}</h2>
            <Button
              size="sm"
              variant="ghost"
              isIconOnly
              aria-label={t("cast.refresh")}
              isDisabled={sessionsQuery.isFetching}
              onPress={() => void sessionsQuery.refetch()}
            >
              {sessionsQuery.isFetching ? (
                <Spinner size="sm" color="current" />
              ) : (
                <RefreshCw className="size-3.5" aria-hidden />
              )}
            </Button>
          </div>

          {sessionsQuery.isPending ? (
            <div className="flex items-center justify-center py-6">
              <Spinner size="sm" />
            </div>
          ) : sessions.length === 0 ? (
            <div className="grid gap-1 rounded-tile border border-dashed border-line bg-sunken/60 px-3 py-4 text-center">
              <p className="text-xs font-mid text-fg">{t("cast.empty")}</p>
              <p className="text-[0.72rem] text-fg-muted">{t("cast.emptyDetail")}</p>
            </div>
          ) : (
            <ul className="grid max-h-80 gap-2 overflow-y-auto">
              {sessions.map((session) => (
                <SessionDeviceRow
                  key={session.id}
                  session={session}
                  isSelf={session.id === localSessionId}
                  busy={commandMutation.isPending}
                  onCommand={(target, cmd) => commandMutation.mutate({ id: target.id, body: cmd })}
                />
              ))}
            </ul>
          )}
        </Popover.Dialog>
      </Popover.Content>
    </Popover>
  );
}
