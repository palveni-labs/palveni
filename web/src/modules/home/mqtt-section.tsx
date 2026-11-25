/**
 * MQTT 3.1.1 broker & cover-color ambient light bridge configuration
 * mounted inside `HomeManageDrawer`.
 */
import { Button, Input, Label, Spinner, Switch, TextField, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Radio, Send, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import { queryKeys } from "@/api/query-keys";
import { RelativeTime } from "@/components/relative-time";
import { errorText } from "@/lib/errors";
import type { HomeDevice } from "@/provisional/types";

export interface MqttBridgeSummary {
  id: string;
  name: string;
  host: string;
  port: number;
  username?: string | null;
  passwordConfigured: boolean;
  topicPrefix: string;
  ambientLightDeviceId?: string | null;
  publishPlayback: boolean;
  enabled: boolean;
  lastPublishedAt?: string | null;
  createdAt: string;
}

export interface MqttBridgeList {
  bridges: MqttBridgeSummary[];
}

export interface SaveMqttBridgeRequest {
  id?: string;
  name: string;
  host: string;
  port: number;
  username?: string;
  password?: string;
  topicPrefix: string;
  ambientLightDeviceId?: string;
  publishPlayback: boolean;
  enabled: boolean;
}

export interface MqttTestResponse {
  ok: boolean;
  topic: string;
  error?: string | null;
}

const SELECT_CLASS = "pv-select w-full";

export function MqttSection({
  open,
  devices,
}: {
  open: boolean;
  devices: HomeDevice[];
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const lights = devices.filter((device) => device.state.type === "light");
  const lightNameById = new Map(lights.map((light) => [light.id, light.name]));

  const [name, setName] = useState("Zigbee2MQTT");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("1883");
  const [topicPrefix, setTopicPrefix] = useState("palveni");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [ambientLightDeviceId, setAmbientLightDeviceId] = useState("");
  const [publishPlayback, setPublishPlayback] = useState(true);

  const bridgesQuery = useQuery({
    queryKey: queryKeys.mqtt.bridges,
    queryFn: ({ signal }) => request<MqttBridgeList>("/home/mqtt", { signal }),
    enabled: open,
    staleTime: 30_000,
  });

  const saveMutation = useMutation({
    mutationFn: (body: SaveMqttBridgeRequest) =>
      request<MqttBridgeSummary>("/home/mqtt", { method: "POST", body }),
    onSuccess: (saved) => {
      queryClient.setQueryData<MqttBridgeList>(queryKeys.mqtt.bridges, (prev) => {
        const list = prev?.bridges ?? [];
        const exists = list.some((entry) => entry.id === saved.id);
        return {
          bridges: exists ? list.map((entry) => (entry.id === saved.id ? saved : entry)) : [...list, saved],
        };
      });
      setHost("");
      setUsername("");
      setPassword("");
      toast.success(t("home.manage.mqttSaved", { name: saved.name }));
    },
    onError: (error) => toast.danger(errorText(t, error)),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => request<void>(`/home/mqtt/${encodeURIComponent(id)}`, { method: "DELETE" }),
    onSuccess: (_res, id) => {
      queryClient.setQueryData<MqttBridgeList>(queryKeys.mqtt.bridges, (prev) =>
        prev ? { bridges: prev.bridges.filter((entry) => entry.id !== id) } : prev,
      );
      toast.success(t("home.manage.mqttDeleted"));
    },
    onError: (error) => toast.danger(errorText(t, error)),
  });

  const testMutation = useMutation({
    mutationFn: (id: string) =>
      request<MqttTestResponse>(`/home/mqtt/${encodeURIComponent(id)}/test`, { method: "POST" }),
    onSuccess: (res) => {
      if (res.ok) {
        toast.success(t("home.manage.mqttTestOk", { topic: res.topic }));
        void queryClient.invalidateQueries({ queryKey: queryKeys.mqtt.bridges });
      } else {
        toast.danger(t("home.manage.mqttTestFailed", { topic: res.topic, error: res.error ?? "Unknown error" }));
      }
    },
    onError: (error) => toast.danger(errorText(t, error)),
  });

  const parsedPort = Number.parseInt(port, 10);
  const validPort = Number.isInteger(parsedPort) && parsedPort >= 1 && parsedPort <= 65535;
  const canSubmit = Boolean(name.trim() && host.trim() && topicPrefix.trim() && validPort && !saveMutation.isPending);

  const onSave = (event: FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;
    saveMutation.mutate({
      name: name.trim(),
      host: host.trim(),
      port: parsedPort,
      topicPrefix: topicPrefix.trim(),
      username: username.trim() || undefined,
      password: password.trim() || undefined,
      ambientLightDeviceId: ambientLightDeviceId || undefined,
      publishPlayback,
      enabled: true,
    });
  };

  const bridges = bridgesQuery.data?.bridges ?? [];

  return (
    <section className="grid gap-3 border-t border-line pt-5">
      <div className="flex items-center gap-2">
        <Radio className="size-4 text-fg-muted" aria-hidden />
        <h2 className="text-sm font-strong text-heading">{t("home.manage.mqttSection")}</h2>
      </div>

      {bridges.length === 0 ? (
        <p className="text-sm text-fg-muted">{t("home.manage.mqttEmpty")}</p>
      ) : (
        <ul className="divide-y divide-line rounded-tile border border-line bg-surface">
          {bridges.map((bridge) => {
            const testing = testMutation.isPending && testMutation.variables === bridge.id;
            const lightName = bridge.ambientLightDeviceId
              ? (lightNameById.get(bridge.ambientLightDeviceId) ?? bridge.ambientLightDeviceId)
              : null;
            return (
              <li key={bridge.id} className="grid gap-2 px-3.5 py-2.5">
                <div className="flex items-center gap-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-mid text-fg">{bridge.name}</p>
                    <p className="truncate font-mono text-xs text-fg-muted">
                      {bridge.host}:{bridge.port} · {bridge.topicPrefix}
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    isDisabled={testing}
                    onPress={() => testMutation.mutate(bridge.id)}
                  >
                    {testing ? <Spinner size="sm" color="current" /> : <Send className="size-3.5" aria-hidden />}
                    {t("home.manage.mqttTest")}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    isIconOnly
                    aria-label={t("home.manage.mqttDelete")}
                    isDisabled={deleteMutation.isPending}
                    onPress={() => deleteMutation.mutate(bridge.id)}
                  >
                    <Trash2 className="size-4" aria-hidden />
                  </Button>
                </div>
                {lightName || bridge.lastPublishedAt ? (
                  <div className="flex flex-wrap items-center gap-2 text-xs text-fg-muted">
                    {lightName ? (
                      <span className="rounded-chip bg-sunken px-2 py-0.5 text-[0.7rem] text-fg">
                        {t("home.manage.mqttAmbientBadge", { name: lightName })}
                      </span>
                    ) : null}
                    {bridge.lastPublishedAt ? (
                      <RelativeTime
                        value={bridge.lastPublishedAt}
                        format={(time) => t("home.manage.mqttLastPublished", { time })}
                      />
                    ) : null}
                  </div>
                ) : null}
              </li>
            );
          })}
        </ul>
      )}

      <form onSubmit={onSave} className="grid gap-2.5 pt-1">
        <TextField value={name} onChange={setName} isDisabled={saveMutation.isPending}>
          <Label>{t("home.manage.mqttName")}</Label>
          <Input placeholder="Zigbee2MQTT" />
        </TextField>
        <div className="grid grid-cols-[minmax(0,1fr)_5.5rem] gap-2.5">
          <TextField value={host} onChange={setHost} isDisabled={saveMutation.isPending}>
            <Label>{t("home.manage.mqttHost")}</Label>
            <Input placeholder="192.168.1.50" />
          </TextField>
          <TextField value={port} onChange={setPort} isDisabled={saveMutation.isPending}>
            <Label>{t("home.manage.mqttPort")}</Label>
            <Input inputMode="numeric" placeholder="1883" />
          </TextField>
        </div>
        <TextField value={topicPrefix} onChange={setTopicPrefix} isDisabled={saveMutation.isPending}>
          <Label>{t("home.manage.mqttTopicPrefix")}</Label>
          <Input placeholder="palveni" />
        </TextField>
        <div className="grid grid-cols-2 gap-2.5">
          <TextField value={username} onChange={setUsername} isDisabled={saveMutation.isPending}>
            <Label>{t("home.manage.mqttUsername")}</Label>
            <Input autoComplete="off" />
          </TextField>
          <TextField value={password} onChange={setPassword} isDisabled={saveMutation.isPending}>
            <Label>{t("home.manage.mqttPassword")}</Label>
            <Input type="password" autoComplete="off" />
          </TextField>
        </div>
        <div className="grid gap-1.5">
          <label htmlFor="home-mqtt-ambient-light" className="text-sm font-mid text-fg">
            {t("home.manage.mqttAmbientLight")}
          </label>
          <select
            id="home-mqtt-ambient-light"
            value={ambientLightDeviceId}
            disabled={saveMutation.isPending}
            onChange={(event) => setAmbientLightDeviceId(event.target.value)}
            className={SELECT_CLASS}
          >
            <option value="">{t("home.manage.mqttNoAmbientLight")}</option>
            {lights.map((light) => (
              <option key={light.id} value={light.id}>
                {light.room ? `${light.room} · ${light.name}` : light.name}
              </option>
            ))}
          </select>
        </div>
        <div className="flex items-center justify-between gap-3 py-1">
          <span className="text-sm text-fg">{t("home.manage.mqttPublishPlayback")}</span>
          <Switch
            aria-label={t("home.manage.mqttPublishPlayback")}
            isSelected={publishPlayback}
            isDisabled={saveMutation.isPending}
            onChange={setPublishPlayback}
          >
            <Switch.Content>
              <Switch.Control>
                <Switch.Thumb />
              </Switch.Control>
            </Switch.Content>
          </Switch>
        </div>
        <div className="flex justify-end pt-1">
          <Button type="submit" size="sm" variant="primary" isDisabled={!canSubmit}>
            {saveMutation.isPending ? <Spinner size="sm" color="current" /> : null}
            {t("home.manage.mqttSave")}
          </Button>
        </div>
      </form>
    </section>
  );
}
