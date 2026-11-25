import { Button, Drawer, Input, Label, Spinner, TextField, toast } from "@heroui/react";
import { Plus, RefreshCw, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { errorText } from "@/lib/errors";
import {
  useCreateHomeDevice,
  useCreateHomeRule,
  useDeleteHomeBridge,
  useHomeBridges,
  useSaveHomeBridge,
  useSyncHomeBridge,
} from "@/provisional/queries/home";
import type { HomeDevice, HomeDeviceKind, Scene } from "@/provisional/types";
import { MqttSection } from "./mqtt-section";

const SELECT_CLASS = "pv-select w-full";

const DEVICE_KINDS: HomeDeviceKind[] = ["light", "switch", "climate", "sensor", "media_player"];

export function HomeManageDrawer({
  open,
  onOpenChange,
  scenes,
  devices = [],
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  scenes: Scene[];
  devices?: HomeDevice[];
}) {
  const { t } = useTranslation();
  const bridgesQuery = useHomeBridges(open);
  const saveBridge = useSaveHomeBridge();
  const syncBridge = useSyncHomeBridge();
  const deleteBridge = useDeleteHomeBridge();
  const createDevice = useCreateHomeDevice();
  const createRule = useCreateHomeRule();

  const [bridgeName, setBridgeName] = useState("Home Assistant");
  const [bridgeUrl, setBridgeUrl] = useState("");
  const [bridgeToken, setBridgeToken] = useState("");

  const [deviceRoom, setDeviceRoom] = useState("客厅");
  const [deviceName, setDeviceName] = useState("");
  const [deviceKind, setDeviceKind] = useState<HomeDeviceKind>("light");

  const [ruleEvent, setRuleEvent] = useState("video.play");
  const [ruleTrigger, setRuleTrigger] = useState("");
  const [ruleAction, setRuleAction] = useState("");
  const [ruleSceneId, setRuleSceneId] = useState("movie");

  const onSaveBridge = (event: FormEvent) => {
    event.preventDefault();
    if (!bridgeName.trim() || !bridgeUrl.trim() || !bridgeToken.trim()) return;
    saveBridge.mutate(
      {
        name: bridgeName.trim(),
        baseUrl: bridgeUrl.trim(),
        token: bridgeToken.trim(),
        enabled: true,
      },
      {
        onSuccess: (saved) => {
          setBridgeToken("");
          toast.success(t("home.manage.bridgeSaved", { name: saved.name }));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const onAddDevice = (event: FormEvent) => {
    event.preventDefault();
    if (!deviceRoom.trim() || !deviceName.trim()) return;
    createDevice.mutate(
      {
        roomName: deviceRoom.trim(),
        name: deviceName.trim(),
        kind: deviceKind,
      },
      {
        onSuccess: (dev) => {
          setDeviceName("");
          toast.success(t("home.manage.deviceAdded", { name: dev.name }));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const onAddRule = (event: FormEvent) => {
    event.preventDefault();
    if (!ruleTrigger.trim() || !ruleAction.trim()) return;
    createRule.mutate(
      {
        eventKind: ruleEvent,
        trigger: ruleTrigger.trim(),
        action: ruleAction.trim(),
        sceneId: ruleSceneId || undefined,
      },
      {
        onSuccess: () => {
          setRuleTrigger("");
          setRuleAction("");
          toast.success(t("home.manage.ruleAdded"));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const bridges = bridgesQuery.data?.bridges ?? [];

  return (
    <Drawer.Backdrop isOpen={open} onOpenChange={onOpenChange}>
      <Drawer.Content placement="right" className="w-full sm:max-w-md">
        <Drawer.Dialog aria-label={t("home.manage.title")} className="flex h-full flex-col">
          <Drawer.CloseTrigger aria-label={t("a11y.close")} className="absolute top-3 end-3 z-10" />
          <Drawer.Header className="border-b border-line px-6 pt-5 pb-4">
            <Drawer.Heading className="font-heading text-lg font-title text-heading">
              {t("home.manage.title")}
            </Drawer.Heading>
          </Drawer.Header>
          <Drawer.Body className="grid content-start gap-6 overflow-y-auto px-6 py-5">
            <section className="grid gap-3">
              <h2 className="text-sm font-strong text-heading">{t("home.manage.bridgesSection")}</h2>
              {bridges.length === 0 ? (
                <p className="text-sm text-fg-muted">{t("home.manage.bridgesEmpty")}</p>
              ) : (
                <ul className="divide-y divide-line rounded-tile border border-line bg-surface">
                  {bridges.map((entry) => {
                    const syncing = syncBridge.isPending && syncBridge.variables === entry.id;
                    return (
                      <li key={entry.id} className="flex items-center gap-2 px-3.5 py-2.5">
                        <div className="min-w-0 flex-1">
                          <p className="truncate text-sm font-mid text-fg">{entry.name}</p>
                          <p className="truncate font-mono text-xs text-fg-muted">{entry.baseUrl}</p>
                        </div>
                        <Button
                          size="sm"
                          variant="secondary"
                          isDisabled={syncing}
                          onPress={() =>
                            syncBridge.mutate(entry.id, {
                              onSuccess: (res) =>
                                toast.success(
                                  t("home.manage.bridgeSynced", {
                                    devices: res.syncedDevices,
                                    scenes: res.syncedScenes,
                                  }),
                                ),
                              onError: (error) => toast.danger(errorText(t, error)),
                            })
                          }
                        >
                          {syncing ? <Spinner size="sm" color="current" /> : <RefreshCw className="size-3.5" aria-hidden />}
                          {t("home.manage.syncBridge")}
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          isIconOnly
                          aria-label={t("home.manage.deleteBridge")}
                          isDisabled={deleteBridge.isPending}
                          onPress={() =>
                            deleteBridge.mutate(entry.id, {
                              onError: (error) => toast.danger(errorText(t, error)),
                            })
                          }
                        >
                          <Trash2 className="size-4" aria-hidden />
                        </Button>
                      </li>
                    );
                  })}
                </ul>
              )}

              <form onSubmit={onSaveBridge} className="grid gap-2.5 pt-1">
                <TextField value={bridgeName} onChange={setBridgeName} isDisabled={saveBridge.isPending}>
                  <Label>{t("home.manage.bridgeName")}</Label>
                  <Input placeholder="Home Assistant" />
                </TextField>
                <TextField value={bridgeUrl} onChange={setBridgeUrl} isDisabled={saveBridge.isPending}>
                  <Label>{t("home.manage.bridgeUrl")}</Label>
                  <Input placeholder="http://192.168.1.10:8123" />
                </TextField>
                <TextField value={bridgeToken} onChange={setBridgeToken} isDisabled={saveBridge.isPending}>
                  <Label>{t("home.manage.bridgeToken")}</Label>
                  <Input type="password" autoComplete="off" placeholder="eyJ..." />
                </TextField>
                <div className="flex justify-end pt-1">
                  <Button
                    type="submit"
                    size="sm"
                    variant="primary"
                    isDisabled={!bridgeName.trim() || !bridgeUrl.trim() || !bridgeToken.trim() || saveBridge.isPending}
                  >
                    {saveBridge.isPending ? <Spinner size="sm" color="current" /> : null}
                    {t("home.manage.saveBridge")}
                  </Button>
                </div>
              </form>
            </section>

            <MqttSection open={open} devices={devices} />

            <section className="grid gap-3 border-t border-line pt-5">
              <h2 className="text-sm font-strong text-heading">{t("home.manage.deviceSection")}</h2>
              <form onSubmit={onAddDevice} className="grid gap-2.5">
                <div className="grid grid-cols-2 gap-2.5">
                  <TextField value={deviceRoom} onChange={setDeviceRoom} isDisabled={createDevice.isPending}>
                    <Label>{t("home.manage.deviceRoom")}</Label>
                    <Input placeholder="客厅" />
                  </TextField>
                  <div className="grid gap-1.5">
                    <label htmlFor="home-device-kind" className="text-sm font-mid text-fg">
                      {t("home.manage.deviceKind")}
                    </label>
                    <select
                      id="home-device-kind"
                      value={deviceKind}
                      disabled={createDevice.isPending}
                      onChange={(event) => setDeviceKind(event.target.value as HomeDeviceKind)}
                      className={SELECT_CLASS}
                    >
                      {DEVICE_KINDS.map((kind) => (
                        <option key={kind} value={kind}>
                          {t(`home.manage.kinds.${kind}`)}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
                <TextField value={deviceName} onChange={setDeviceName} isDisabled={createDevice.isPending}>
                  <Label>{t("home.manage.deviceName")}</Label>
                  <Input placeholder="落地灯" />
                </TextField>
                <div className="flex justify-end pt-1">
                  <Button
                    type="submit"
                    size="sm"
                    variant="secondary"
                    isDisabled={!deviceRoom.trim() || !deviceName.trim() || createDevice.isPending}
                  >
                    <Plus className="size-4" aria-hidden />
                    {t("home.manage.addDevice")}
                  </Button>
                </div>
              </form>
            </section>

            <section className="grid gap-3 border-t border-line pt-5">
              <h2 className="text-sm font-strong text-heading">{t("home.manage.ruleSection")}</h2>
              <form onSubmit={onAddRule} className="grid gap-2.5">
                <div className="grid gap-1.5">
                  <label htmlFor="home-rule-event" className="text-sm font-mid text-fg">
                    {t("home.manage.ruleEvent")}
                  </label>
                  <select
                    id="home-rule-event"
                    value={ruleEvent}
                    disabled={createRule.isPending}
                    onChange={(event) => setRuleEvent(event.target.value)}
                    className={SELECT_CLASS}
                  >
                    <option value="video.play">{t("home.manage.events.videoPlay")}</option>
                    <option value="video.pause">{t("home.manage.events.videoPause")}</option>
                    <option value="video.stop">{t("home.manage.events.videoStop")}</option>
                    <option value="music.play">{t("home.manage.events.musicPlay")}</option>
                    <option value="music.pause">{t("home.manage.events.musicPause")}</option>
                    <option value="night.play">{t("home.manage.events.nightPlay")}</option>
                  </select>
                </div>
                <TextField value={ruleTrigger} onChange={setRuleTrigger} isDisabled={createRule.isPending}>
                  <Label>{t("home.manage.ruleTrigger")}</Label>
                  <Input placeholder="开始播放电影时调暗客厅灯光" />
                </TextField>
                <TextField value={ruleAction} onChange={setRuleAction} isDisabled={createRule.isPending}>
                  <Label>{t("home.manage.ruleAction")}</Label>
                  <Input placeholder="主灯关闭，灯带调至 15%" />
                </TextField>
                {scenes.length > 0 ? (
                  <div className="grid gap-1.5">
                    <label htmlFor="home-rule-scene" className="text-sm font-mid text-fg">
                      {t("home.manage.ruleScene")}
                    </label>
                    <select
                      id="home-rule-scene"
                      value={ruleSceneId}
                      disabled={createRule.isPending}
                      onChange={(event) => setRuleSceneId(event.target.value)}
                      className={SELECT_CLASS}
                    >
                      <option value="">{t("home.manage.noSceneBind")}</option>
                      {scenes.map((scene) => (
                        <option key={scene.id} value={scene.id}>
                          {scene.name}
                        </option>
                      ))}
                    </select>
                  </div>
                ) : null}
                <div className="flex justify-end pt-1">
                  <Button
                    type="submit"
                    size="sm"
                    variant="secondary"
                    isDisabled={!ruleTrigger.trim() || !ruleAction.trim() || createRule.isPending}
                  >
                    <Plus className="size-4" aria-hidden />
                    {t("home.manage.addRule")}
                  </Button>
                </div>
              </form>
            </section>
          </Drawer.Body>
        </Drawer.Dialog>
      </Drawer.Content>
    </Drawer.Backdrop>
  );
}
