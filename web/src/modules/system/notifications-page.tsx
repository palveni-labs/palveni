import { Button, Chip, Input, Switch, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Bell,
  CalendarClock,
  CheckCheck,
  Copy,
  Play,
  Plus,
  Send,
  Trash2,
  Webhook,
} from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { useSecureSurface } from "@/skins/secure";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import {
  notificationKeys,
  notificationsApi,
  type CreatedInboundWebhook,
  type InboundWebhook,
  type InboundWebhookSourceKind,
  type NotificationChannel,
  type NotificationChannelKind,
  type RoutineActionKind,
  type RoutineScheduleKind,
  type ScheduledRoutine,
} from "./notifications-api";

const CHANNEL_KINDS: NotificationChannelKind[] = [
  "webhook",
  "telegram",
  "bark",
  "ntfy",
  "wecom",
  "feishu",
  "discord",
];

const SCHEDULE_KINDS: RoutineScheduleKind[] = ["hourly", "daily", "weekly", "on_event"];
const WEBHOOK_SOURCES: InboundWebhookSourceKind[] = [
  "sonarr",
  "radarr",
  "moviepilot",
  "qbittorrent",
  "custom",
];

const selectClass =
  "w-full rounded-control border border-line bg-surface px-3 py-2 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)";

async function copyText(value: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(value);
      return true;
    }
  } catch {
    // Fall back when clipboard API is unavailable in insecure LAN contexts.
  }
  return false;
}

function defaultScheduleExpr(kind: RoutineScheduleKind): string {
  switch (kind) {
    case "hourly":
      return "00";
    case "daily":
      return "03:00";
    case "weekly":
      return "MON 03:00";
    case "on_event":
      return "webhook.triggered";
  }
}

function InboxSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const queryClient = useQueryClient();
  const inbox = useQuery({
    queryKey: notificationKeys.inbox(),
    queryFn: ({ signal }) => notificationsApi.list(50, signal),
  });
  const markAll = useMutation({
    mutationFn: () => notificationsApi.markAllRead(),
    onSuccess: (res) => {
      toast.success(t("notificationsCenter.markedAllRead", { count: res.markedRead }));
      void queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
    },
  });

  const unread = inbox.data?.unreadCount ?? 0;
  const items = inbox.data?.notifications ?? [];

  return (
    <Section
      title={t("notificationsCenter.inboxTitle")}
      count={unread > 0 ? unread : items.length}
      actions={
        unread > 0 ? (
          <Button
            size="sm"
            variant="outline"
            isPending={markAll.isPending}
            onPress={() => markAll.mutate()}
          >
            <CheckCheck aria-hidden />
            {t("notificationsCenter.markAllRead")}
          </Button>
        ) : undefined
      }
    >
      {inbox.isPending ? (
        <LoadingRows rows={3} />
      ) : inbox.isError ? (
        <ErrorState compact error={inbox.error} onRetry={() => void inbox.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState icon={Bell} title={t("notificationsCenter.emptyInbox")} />
      ) : (
        <SectionRows>
          {items.map((item) => {
            const isUnread = item.status === "in_app" && !item.readAt;
            return (
              <li
                key={item.id}
                className={cn(
                  "flex flex-wrap items-start justify-between gap-3 py-3",
                  inset,
                )}
              >
                <div className="grid min-w-0 flex-1 gap-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className={cn("text-base text-fg", isUnread && "font-strong")}>
                      {item.title}
                    </span>
                    {isUnread && (
                      <Chip size="sm" color="accent" variant="soft">
                        {t("notificationsCenter.unreadTag")}
                      </Chip>
                    )}
                    <Chip
                      size="sm"
                      color={item.status === "failed" ? "danger" : "default"}
                      variant="soft"
                    >
                      {t(`notificationsCenter.status.${item.status}`)}
                    </Chip>
                    <span className="font-mono text-xs text-fg-muted">{item.eventType}</span>
                  </div>
                  <p className="text-sm text-fg-muted">{item.body}</p>
                  {item.error && (
                    <p role="alert" className="text-xs text-failed">
                      {item.error}
                    </p>
                  )}
                </div>
                <time
                  dateTime={item.createdAt}
                  className="shrink-0 text-xs text-fg-muted tabular-nums"
                >
                  {new Date(item.createdAt).toLocaleString()}
                </time>
              </li>
            );
          })}
        </SectionRows>
      )}
    </Section>
  );
}

function ChannelsSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<NotificationChannel | null>(null);

  const [name, setName] = useState("");
  const [kind, setKind] = useState<NotificationChannelKind>("webhook");
  const [url, setUrl] = useState("");
  const [token, setToken] = useState("");
  const [target, setTarget] = useState("");
  const [eventsText, setEventsText] = useState("*");
  const [quietHours, setQuietHours] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const channelsQuery = useQuery({
    queryKey: notificationKeys.channels(),
    queryFn: ({ signal }) => notificationsApi.listChannels(signal),
  });

  const resetForm = () => {
    setName("");
    setKind("webhook");
    setUrl("");
    setToken("");
    setTarget("");
    setEventsText("*");
    setQuietHours("");
    setFormError(null);
  };

  const createMutation = useMutation({
    mutationFn: notificationsApi.createChannel,
    onSuccess: () => {
      resetForm();
      setAdding(false);
      void queryClient.invalidateQueries({ queryKey: notificationKeys.channels() });
    },
    onError: (err) => setFormError(errorText(t, err)),
  });

  const toggleMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      notificationsApi.updateChannel(id, { enabled }),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: notificationKeys.channels() }),
  });

  const testMutation = useMutation({
    mutationFn: (id: string) => notificationsApi.testChannel(id),
    onSuccess: (res) => {
      void queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
      if (res.delivery.status === "sent") {
        toast.success(t("notificationsCenter.testSucceeded"));
      } else {
        toast.danger(
          t("notificationsCenter.testFailed", {
            reason: res.delivery.error ?? res.delivery.status,
          }),
        );
      }
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => notificationsApi.deleteChannel(id),
    onSuccess: () => {
      setConfirmDelete(null);
      void queryClient.invalidateQueries({ queryKey: notificationKeys.channels() });
    },
  });

  function submitChannel(event: FormEvent) {
    event.preventDefault();
    setFormError(null);
    const events = eventsText
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
    createMutation.mutate({
      name: name.trim(),
      kind,
      config: {
        url: url.trim() || null,
        token: token.trim() || null,
        target: target.trim() || null,
      },
      events: events.length > 0 ? events : ["*"],
      quietHours: quietHours.trim() || null,
      enabled: true,
    });
  }

  const channels = channelsQuery.data?.channels ?? [];

  return (
    <Section
      title={t("notificationsCenter.channelsTitle")}
      count={channels.length}
      actions={
        !adding ? (
          <Button size="sm" variant="outline" onPress={() => setAdding(true)}>
            <Plus aria-hidden />
            {t("notificationsCenter.addChannel")}
          </Button>
        ) : undefined
      }
    >
      {adding && (
        <form
          {...secure}
          onSubmit={submitChannel}
          className={cn("grid gap-3 border-b border-line py-4", inset)}
        >
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.channelName")}
              <Input
                required
                aria-label={t("notificationsCenter.channelName")}
                placeholder={t("notificationsCenter.channelNamePlaceholder")}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.channelKind")}
              <select
                aria-label={t("notificationsCenter.channelKind")}
                className={selectClass}
                value={kind}
                onChange={(e) => setKind(e.target.value as NotificationChannelKind)}
              >
                {CHANNEL_KINDS.map((k) => (
                  <option key={k} value={k}>
                    {t(`notificationsCenter.kinds.${k}`)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          {kind === "telegram" ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="grid gap-1 text-sm text-fg">
                {t("notificationsCenter.botToken")}
                <Input
                  required
                  type="password"
                  aria-label={t("notificationsCenter.botToken")}
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                />
              </label>
              <label className="grid gap-1 text-sm text-fg">
                {t("notificationsCenter.chatId")}
                <Input
                  required
                  aria-label={t("notificationsCenter.chatId")}
                  value={target}
                  onChange={(e) => setTarget(e.target.value)}
                />
              </label>
            </div>
          ) : (
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="grid gap-1 text-sm text-fg">
                {t("notificationsCenter.endpointUrl")}
                <Input
                  required
                  aria-label={t("notificationsCenter.endpointUrl")}
                  placeholder={
                    kind === "ntfy"
                      ? "https://ntfy.sh"
                      : kind === "bark"
                        ? "https://api.day.app"
                        : "https://"
                  }
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                />
              </label>
              {kind === "ntfy" ? (
                <label className="grid gap-1 text-sm text-fg">
                  {t("notificationsCenter.topic")}
                  <Input
                    aria-label={t("notificationsCenter.topic")}
                    value={target}
                    onChange={(e) => setTarget(e.target.value)}
                  />
                </label>
              ) : (
                <label className="grid gap-1 text-sm text-fg">
                  {t(
                    kind === "bark"
                      ? "notificationsCenter.deviceKey"
                      : "notificationsCenter.secretToken",
                  )}
                  <Input
                    type="password"
                    aria-label={t(
                      kind === "bark"
                        ? "notificationsCenter.deviceKey"
                        : "notificationsCenter.secretToken",
                    )}
                    value={token}
                    onChange={(e) => setToken(e.target.value)}
                  />
                </label>
              )}
            </div>
          )}

          <div className="grid gap-3 sm:grid-cols-2">
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.eventsLabel")}
              <Input
                aria-label={t("notificationsCenter.eventsLabel")}
                value={eventsText}
                onChange={(e) => setEventsText(e.target.value)}
              />
            </label>
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.quietHours")}
              <Input
                aria-label={t("notificationsCenter.quietHours")}
                placeholder={t("notificationsCenter.quietHoursPlaceholder")}
                value={quietHours}
                onChange={(e) => setQuietHours(e.target.value)}
              />
            </label>
          </div>

          {formError && (
            <p role="alert" className="text-sm text-failed">
              {formError}
            </p>
          )}
          <div className="flex flex-wrap justify-end gap-2">
            <Button
              size="sm"
              variant="ghost"
              onPress={() => {
                resetForm();
                setAdding(false);
              }}
            >
              {t("common.cancel")}
            </Button>
            <Button type="submit" size="sm" isPending={createMutation.isPending}>
              {t("common.create")}
            </Button>
          </div>
        </form>
      )}

      {channelsQuery.isPending ? (
        <LoadingRows rows={2} />
      ) : channelsQuery.isError ? (
        <ErrorState compact error={channelsQuery.error} onRetry={() => void channelsQuery.refetch()} />
      ) : channels.length === 0 ? (
        <EmptyState icon={Send} title={t("notificationsCenter.emptyChannels")} />
      ) : (
        <SectionRows>
          {channels.map((ch) => (
            <li key={ch.id} className={cn("grid gap-2 py-3.5", inset)}>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="grid min-w-0 flex-1 gap-0.5">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-strong text-fg">{ch.name}</span>
                    <Chip size="sm" variant="soft">
                      {t(`notificationsCenter.kinds.${ch.kind}`)}
                    </Chip>
                    {ch.quietHours && (
                      <Chip size="sm" variant="soft">
                        {t("notificationsCenter.quietBadge", { range: ch.quietHours })}
                      </Chip>
                    )}
                  </div>
                  <div className="flex flex-wrap items-center gap-2 text-xs text-fg-muted">
                    {ch.targetHint && <span className="font-mono">{ch.targetHint}</span>}
                    <span>{ch.events.join(", ")}</span>
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Switch
                    size="sm"
                    aria-label={t("notificationsCenter.toggleChannel", { name: ch.name })}
                    isSelected={ch.enabled}
                    onChange={(enabled) => toggleMutation.mutate({ id: ch.id, enabled })}
                  >
                    <Switch.Control>
                      <Switch.Thumb />
                    </Switch.Control>
                  </Switch>
                  <Button
                    size="sm"
                    variant="outline"
                    isPending={testMutation.isPending && testMutation.variables === ch.id}
                    onPress={() => testMutation.mutate(ch.id)}
                  >
                    <Send aria-hidden />
                    {t("notificationsCenter.testSend")}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    isIconOnly
                    aria-label={t("notificationsCenter.deleteChannel") + " " + ch.name}
                    onPress={() => setConfirmDelete(ch)}
                  >
                    <Trash2 aria-hidden />
                  </Button>
                </div>
              </div>
              {confirmDelete?.id === ch.id && (
                <div {...secure} className="flex flex-wrap items-center gap-2 pt-1 text-sm">
                  <span>{t("notificationsCenter.confirmDeleteChannel", { name: ch.name })}</span>
                  <Button
                    size="sm"
                    variant="danger"
                    isPending={deleteMutation.isPending}
                    onPress={() => deleteMutation.mutate(ch.id)}
                  >
                    {t("common.delete")}
                  </Button>
                  <Button size="sm" variant="ghost" onPress={() => setConfirmDelete(null)}>
                    {t("common.cancel")}
                  </Button>
                </div>
              )}
            </li>
          ))}
        </SectionRows>
      )}
    </Section>
  );
}

function RoutinesSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const secure = useSecureSurface();
  const isAdmin = useIsAdmin();
  const queryClient = useQueryClient();
  const libraries = useLibraries();

  const [adding, setAdding] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<ScheduledRoutine | null>(null);
  const [name, setName] = useState("");
  const [scheduleKind, setScheduleKind] = useState<RoutineScheduleKind>("daily");
  const [scheduleExpr, setScheduleExpr] = useState("03:00");
  const [actionKind, setActionKind] = useState<RoutineActionKind>(
    isAdmin ? "scan_all" : "home_scene",
  );
  const [actionPayload, setActionPayload] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const routinesQuery = useQuery({
    queryKey: notificationKeys.routines(),
    queryFn: ({ signal }) => notificationsApi.listRoutines(signal),
  });

  const availableActions: RoutineActionKind[] = isAdmin
    ? ["scan_all", "scan_library", "home_scene", "assistant_prompt"]
    : ["home_scene", "assistant_prompt"];

  const resetForm = () => {
    setName("");
    setScheduleKind("daily");
    setScheduleExpr("03:00");
    setActionKind(isAdmin ? "scan_all" : "home_scene");
    setActionPayload("");
    setFormError(null);
  };

  const createMutation = useMutation({
    mutationFn: notificationsApi.createRoutine,
    onSuccess: () => {
      resetForm();
      setAdding(false);
      void queryClient.invalidateQueries({ queryKey: notificationKeys.routines() });
    },
    onError: (err) => setFormError(errorText(t, err)),
  });

  const toggleMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      notificationsApi.updateRoutine(id, { enabled }),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: notificationKeys.routines() }),
  });

  const runMutation = useMutation({
    mutationFn: (id: string) => notificationsApi.runRoutine(id),
    onSuccess: (res) => {
      toast.success(t("notificationsCenter.ranSucceeded", { detail: res.detail }));
      void queryClient.invalidateQueries({ queryKey: notificationKeys.routines() });
      void queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => notificationsApi.deleteRoutine(id),
    onSuccess: () => {
      setConfirmDelete(null);
      void queryClient.invalidateQueries({ queryKey: notificationKeys.routines() });
    },
  });

  function submitRoutine(event: FormEvent) {
    event.preventDefault();
    setFormError(null);
    const fallbackLib = libraries.data?.libraries[0]?.id ?? "";
    const resolvedPayload =
      actionKind === "scan_all"
        ? ""
        : actionKind === "scan_library"
          ? actionPayload || fallbackLib
          : actionKind === "home_scene"
            ? actionPayload.trim() || "movie"
            : actionPayload.trim();
    createMutation.mutate({
      name: name.trim(),
      scheduleKind,
      scheduleExpr: scheduleExpr.trim(),
      actionKind,
      actionPayload: resolvedPayload,
      enabled: true,
    });
  }

  const routines = routinesQuery.data?.routines ?? [];
  const libs = libraries.data?.libraries ?? [];

  return (
    <Section
      title={t("notificationsCenter.routinesTitle")}
      count={routines.length}
      actions={
        !adding ? (
          <Button size="sm" variant="outline" onPress={() => setAdding(true)}>
            <Plus aria-hidden />
            {t("notificationsCenter.addRoutine")}
          </Button>
        ) : undefined
      }
    >
      {adding && (
        <form
          {...secure}
          onSubmit={submitRoutine}
          className={cn("grid gap-3 border-b border-line py-4", inset)}
        >
          <div className="grid gap-3 sm:grid-cols-3">
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.routineName")}
              <Input
                required
                aria-label={t("notificationsCenter.routineName")}
                placeholder={t("notificationsCenter.routineNamePlaceholder")}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.scheduleKind")}
              <select
                aria-label={t("notificationsCenter.scheduleKind")}
                className={selectClass}
                value={scheduleKind}
                onChange={(e) => {
                  const next = e.target.value as RoutineScheduleKind;
                  setScheduleKind(next);
                  setScheduleExpr(defaultScheduleExpr(next));
                }}
              >
                {SCHEDULE_KINDS.map((sk) => (
                  <option key={sk} value={sk}>
                    {t(`notificationsCenter.scheduleKinds.${sk}`)}
                  </option>
                ))}
              </select>
            </label>
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.scheduleExpr")}
              <Input
                required
                aria-label={t("notificationsCenter.scheduleExpr")}
                value={scheduleExpr}
                onChange={(e) => setScheduleExpr(e.target.value)}
              />
            </label>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.actionKind")}
              <select
                aria-label={t("notificationsCenter.actionKind")}
                className={selectClass}
                value={actionKind}
                onChange={(e) => {
                  const next = e.target.value as RoutineActionKind;
                  setActionKind(next);
                  setActionPayload(
                    next === "scan_library"
                      ? (libs[0]?.id ?? "")
                      : next === "home_scene"
                        ? "movie"
                        : "",
                  );
                }}
              >
                {availableActions.map((ak) => (
                  <option key={ak} value={ak}>
                    {t(`notificationsCenter.actionKinds.${ak}`)}
                  </option>
                ))}
              </select>
            </label>

            {actionKind === "scan_library" && (
              <label className="grid gap-1 text-sm text-fg">
                {t("notificationsCenter.targetLibrary")}
                <select
                  aria-label={t("notificationsCenter.targetLibrary")}
                  className={selectClass}
                  value={actionPayload || libs[0]?.id || ""}
                  onChange={(e) => setActionPayload(e.target.value)}
                >
                  {libs.map((lib) => (
                    <option key={lib.id} value={lib.id}>
                      {lib.name}
                    </option>
                  ))}
                </select>
              </label>
            )}

            {actionKind === "home_scene" && (
              <label className="grid gap-1 text-sm text-fg">
                {t("notificationsCenter.sceneId")}
                <Input
                  required
                  aria-label={t("notificationsCenter.sceneId")}
                  placeholder="movie"
                  value={actionPayload}
                  onChange={(e) => setActionPayload(e.target.value)}
                />
              </label>
            )}

            {actionKind === "assistant_prompt" && (
              <label className="grid gap-1 text-sm text-fg">
                {t("notificationsCenter.assistantPrompt")}
                <Input
                  required
                  aria-label={t("notificationsCenter.assistantPrompt")}
                  value={actionPayload}
                  onChange={(e) => setActionPayload(e.target.value)}
                />
              </label>
            )}
          </div>

          {formError && (
            <p role="alert" className="text-sm text-failed">
              {formError}
            </p>
          )}
          <div className="flex flex-wrap justify-end gap-2">
            <Button
              size="sm"
              variant="ghost"
              onPress={() => {
                resetForm();
                setAdding(false);
              }}
            >
              {t("common.cancel")}
            </Button>
            <Button type="submit" size="sm" isPending={createMutation.isPending}>
              {t("common.create")}
            </Button>
          </div>
        </form>
      )}

      {routinesQuery.isPending ? (
        <LoadingRows rows={2} />
      ) : routinesQuery.isError ? (
        <ErrorState compact error={routinesQuery.error} onRetry={() => void routinesQuery.refetch()} />
      ) : routines.length === 0 ? (
        <EmptyState icon={CalendarClock} title={t("notificationsCenter.emptyRoutines")} />
      ) : (
        <SectionRows>
          {routines.map((rt) => (
            <li key={rt.id} className={cn("grid gap-2 py-3.5", inset)}>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="grid min-w-0 flex-1 gap-0.5">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-strong text-fg">{rt.name}</span>
                    <Chip size="sm" variant="soft">
                      {t(`notificationsCenter.scheduleKinds.${rt.scheduleKind}`)} · {rt.scheduleExpr}
                    </Chip>
                    <Chip size="sm" variant="soft">
                      {t(`notificationsCenter.actionKinds.${rt.actionKind}`)}
                    </Chip>
                  </div>
                  <div className="flex flex-wrap items-center gap-3 text-xs text-fg-muted">
                    <span>
                      {rt.lastRunAt
                        ? t("notificationsCenter.lastRun", {
                            time: new Date(rt.lastRunAt).toLocaleString(),
                          })
                        : t("notificationsCenter.neverRun")}
                    </span>
                    {rt.lastStatus && (
                      <span
                        className={cn(
                          "font-mono",
                          rt.lastStatus.startsWith("failed") && "text-failed",
                        )}
                      >
                        {rt.lastStatus}
                      </span>
                    )}
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Switch
                    size="sm"
                    aria-label={t("notificationsCenter.toggleRoutine", { name: rt.name })}
                    isSelected={rt.enabled}
                    onChange={(enabled) => toggleMutation.mutate({ id: rt.id, enabled })}
                  >
                    <Switch.Control>
                      <Switch.Thumb />
                    </Switch.Control>
                  </Switch>
                  <Button
                    size="sm"
                    variant="outline"
                    isPending={runMutation.isPending && runMutation.variables === rt.id}
                    onPress={() => runMutation.mutate(rt.id)}
                  >
                    <Play aria-hidden />
                    {t("notificationsCenter.runNow")}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    isIconOnly
                    aria-label={t("notificationsCenter.deleteRoutine") + " " + rt.name}
                    onPress={() => setConfirmDelete(rt)}
                  >
                    <Trash2 aria-hidden />
                  </Button>
                </div>
              </div>
              {confirmDelete?.id === rt.id && (
                <div {...secure} className="flex flex-wrap items-center gap-2 pt-1 text-sm">
                  <span>{t("notificationsCenter.confirmDeleteRoutine", { name: rt.name })}</span>
                  <Button
                    size="sm"
                    variant="danger"
                    isPending={deleteMutation.isPending}
                    onPress={() => deleteMutation.mutate(rt.id)}
                  >
                    {t("common.delete")}
                  </Button>
                  <Button size="sm" variant="ghost" onPress={() => setConfirmDelete(null)}>
                    {t("common.cancel")}
                  </Button>
                </div>
              )}
            </li>
          ))}
        </SectionRows>
      )}
    </Section>
  );
}

function InboundWebhooksSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();
  const libraries = useLibraries();

  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");
  const [sourceKind, setSourceKind] = useState<InboundWebhookSourceKind>("sonarr");
  const [libraryId, setLibraryId] = useState("");
  const [createdOnce, setCreatedOnce] = useState<CreatedInboundWebhook | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<InboundWebhook | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const webhooksQuery = useQuery({
    queryKey: notificationKeys.webhooks(),
    queryFn: ({ signal }) => notificationsApi.listInboundWebhooks(signal),
  });

  const createMutation = useMutation({
    mutationFn: notificationsApi.createInboundWebhook,
    onSuccess: (created) => {
      setCreatedOnce(created);
      setName("");
      setLibraryId("");
      setAdding(false);
      setFormError(null);
      void queryClient.invalidateQueries({ queryKey: notificationKeys.webhooks() });
    },
    onError: (err) => setFormError(errorText(t, err)),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => notificationsApi.deleteInboundWebhook(id),
    onSuccess: () => {
      setConfirmDelete(null);
      void queryClient.invalidateQueries({ queryKey: notificationKeys.webhooks() });
    },
  });

  function submitWebhook(event: FormEvent) {
    event.preventDefault();
    setFormError(null);
    createMutation.mutate({
      name: name.trim(),
      sourceKind,
      libraryId: libraryId || null,
      enabled: true,
    });
  }

  const webhooks = webhooksQuery.data?.webhooks ?? [];
  const libs = libraries.data?.libraries ?? [];
  const oneTimeUrl = createdOnce
    ? new URL(createdOnce.triggerPath, window.location.origin).href
    : "";

  return (
    <Section
      title={t("notificationsCenter.webhooksTitle")}
      count={webhooks.length}
      actions={
        !adding ? (
          <Button size="sm" variant="outline" onPress={() => setAdding(true)}>
            <Plus aria-hidden />
            {t("notificationsCenter.addWebhook")}
          </Button>
        ) : undefined
      }
    >
      {createdOnce && (
        <div
          {...secure}
          className={cn("grid gap-2 border-b border-line bg-sunken py-3.5", inset)}
        >
          <span className="text-sm font-strong text-fg">
            {t("notificationsCenter.triggerUrlLabel")}
          </span>
          <div className="flex flex-wrap items-center gap-2">
            <Input
              readOnly
              className="min-w-0 flex-1 font-mono"
              aria-label={t("notificationsCenter.triggerUrlLabel")}
              value={oneTimeUrl}
              onFocus={(e) => e.currentTarget.select()}
            />
            <Button
              size="sm"
              variant="outline"
              onPress={() => {
                void copyText(oneTimeUrl).then(() =>
                  toast.success(t("notificationsCenter.copied")),
                );
              }}
            >
              <Copy aria-hidden />
              {t("notificationsCenter.copyUrl")}
            </Button>
            <Button size="sm" variant="ghost" onPress={() => setCreatedOnce(null)}>
              {t("common.done")}
            </Button>
          </div>
        </div>
      )}

      {adding && (
        <form
          {...secure}
          onSubmit={submitWebhook}
          className={cn("grid gap-3 border-b border-line py-4", inset)}
        >
          <div className="grid gap-3 sm:grid-cols-3">
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.webhookName")}
              <Input
                required
                aria-label={t("notificationsCenter.webhookName")}
                placeholder={t("notificationsCenter.webhookNamePlaceholder")}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.sourceKind")}
              <select
                aria-label={t("notificationsCenter.sourceKind")}
                className={selectClass}
                value={sourceKind}
                onChange={(e) => setSourceKind(e.target.value as InboundWebhookSourceKind)}
              >
                {WEBHOOK_SOURCES.map((src) => (
                  <option key={src} value={src}>
                    {t(`notificationsCenter.sources.${src}`)}
                  </option>
                ))}
              </select>
            </label>
            <label className="grid gap-1 text-sm text-fg">
              {t("notificationsCenter.boundLibrary")}
              <select
                aria-label={t("notificationsCenter.boundLibrary")}
                className={selectClass}
                value={libraryId}
                onChange={(e) => setLibraryId(e.target.value)}
              >
                <option value="">{t("notificationsCenter.allMediaLibraries")}</option>
                {libs.map((lib) => (
                  <option key={lib.id} value={lib.id}>
                    {lib.name}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {formError && (
            <p role="alert" className="text-sm text-failed">
              {formError}
            </p>
          )}
          <div className="flex flex-wrap justify-end gap-2">
            <Button
              size="sm"
              variant="ghost"
              onPress={() => {
                setAdding(false);
                setFormError(null);
              }}
            >
              {t("common.cancel")}
            </Button>
            <Button type="submit" size="sm" isPending={createMutation.isPending}>
              {t("common.create")}
            </Button>
          </div>
        </form>
      )}

      {webhooksQuery.isPending ? (
        <LoadingRows rows={2} />
      ) : webhooksQuery.isError ? (
        <ErrorState compact error={webhooksQuery.error} onRetry={() => void webhooksQuery.refetch()} />
      ) : webhooks.length === 0 ? (
        <EmptyState icon={Webhook} title={t("notificationsCenter.emptyWebhooks")} />
      ) : (
        <SectionRows>
          {webhooks.map((wh) => {
            const boundLib = libs.find((l) => l.id === wh.libraryId);
            return (
              <li key={wh.id} className={cn("grid gap-2 py-3.5", inset)}>
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div className="grid min-w-0 flex-1 gap-0.5">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-strong text-fg">{wh.name}</span>
                      <Chip size="sm" variant="soft">
                        {t(`notificationsCenter.sources.${wh.sourceKind}`)}
                      </Chip>
                      <span className="text-xs text-fg-muted">
                        {boundLib ? boundLib.name : t("notificationsCenter.allMediaLibraries")}
                      </span>
                    </div>
                    <span className="text-xs text-fg-muted">
                      {wh.lastTriggeredAt
                        ? t("notificationsCenter.lastTriggered", {
                            time: new Date(wh.lastTriggeredAt).toLocaleString(),
                          })
                        : t("notificationsCenter.neverTriggered")}
                    </span>
                  </div>
                  <Button
                    size="sm"
                    variant="ghost"
                    isIconOnly
                    aria-label={t("notificationsCenter.deleteWebhook") + " " + wh.name}
                    onPress={() => setConfirmDelete(wh)}
                  >
                    <Trash2 aria-hidden />
                  </Button>
                </div>
                {confirmDelete?.id === wh.id && (
                  <div {...secure} className="flex flex-wrap items-center gap-2 pt-1 text-sm">
                    <span>{t("notificationsCenter.confirmDeleteWebhook", { name: wh.name })}</span>
                    <Button
                      size="sm"
                      variant="danger"
                      isPending={deleteMutation.isPending}
                      onPress={() => deleteMutation.mutate(wh.id)}
                    >
                      {t("common.delete")}
                    </Button>
                    <Button size="sm" variant="ghost" onPress={() => setConfirmDelete(null)}>
                      {t("common.cancel")}
                    </Button>
                  </div>
                )}
              </li>
            );
          })}
        </SectionRows>
      )}
    </Section>
  );
}

export function NotificationsPage() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const hasRoutines = useFeature("routines");

  return (
    <PageFrame>
      <PageHeader title={t("notificationsCenter.title")} />
      <InboxSection />
      <ChannelsSection />
      {hasRoutines && <RoutinesSection />}
      {isAdmin && <InboundWebhooksSection />}
    </PageFrame>
  );
}
