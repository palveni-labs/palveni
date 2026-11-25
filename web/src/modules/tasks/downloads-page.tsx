import { Button, Drawer, Input, Label, Spinner, TextField, toast } from "@heroui/react";
import { Check, Download, Plus, RefreshCw, Rss, Server, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useSystemInfo } from "@/api/queries/system";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import { useFeature } from "@/modules/access";
import {
  useCheckSubscription,
  useCreateDownloadTask,
  useCreateSubscription,
  useDeleteDownloaderClient,
  useDeleteSubscription,
  useDownloadsOverview,
  useSaveDownloaderClient,
} from "@/provisional/queries/downloads";
import type {
  DownloaderKind,
  FeedMatchItem,
  SubscriptionMediaKind,
} from "@/provisional/types";
import { PageFrame } from "@/ui/frame";
import { PageHeader, SummaryFacts } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

const SELECT_CLASS = "pv-select w-full";

const DEFAULT_CLIENT_URL: Record<DownloaderKind, string> = {
  qbittorrent: "http://192.168.1.20:8080",
  transmission: "http://192.168.1.20:9091",
  aria2: "http://192.168.1.20:6800",
};

function ClientModalDrawer({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation();
  const saveClient = useSaveDownloaderClient();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<DownloaderKind>("qbittorrent");
  const [baseUrl, setBaseUrl] = useState(DEFAULT_CLIENT_URL.qbittorrent);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [savePath, setSavePath] = useState("");

  const onKindChange = (nextKind: DownloaderKind) => {
    setKind(nextKind);
    setBaseUrl(DEFAULT_CLIENT_URL[nextKind]);
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (!name.trim() || !baseUrl.trim()) return;
    saveClient.mutate(
      {
        name: name.trim(),
        kind,
        baseUrl: baseUrl.trim(),
        username: username.trim() || undefined,
        password: password.trim() || undefined,
        savePath: savePath.trim() || undefined,
        enabled: true,
      },
      {
        onSuccess: (saved) => {
          toast.success(t("downloadsApp.clientSaved", { name: saved.name }));
          setName("");
          setPassword("");
          onOpenChange(false);
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  return (
    <Drawer.Backdrop isOpen={open} onOpenChange={onOpenChange}>
      <Drawer.Content placement="right" className="w-full sm:max-w-md">
        <Drawer.Dialog aria-label={t("downloadsApp.configureClient")} className="flex h-full flex-col">
          <Drawer.CloseTrigger aria-label={t("a11y.close")} className="absolute top-3 end-3 z-10" />
          <Drawer.Header className="border-b border-line px-6 pt-5 pb-4">
            <Drawer.Heading className="font-heading text-lg font-title text-heading">
              {t("downloadsApp.configureClient")}
            </Drawer.Heading>
          </Drawer.Header>
          <Drawer.Body className="overflow-y-auto px-6 py-5">
            <form onSubmit={onSubmit} className="grid gap-3.5">
              <div className="grid gap-1.5">
                <label htmlFor="dl-client-kind" className="text-sm font-mid text-fg">
                  {t("downloadsApp.clientKind")}
                </label>
                <select
                  id="dl-client-kind"
                  value={kind}
                  onChange={(event) => onKindChange(event.target.value as DownloaderKind)}
                  className={SELECT_CLASS}
                >
                  <option value="qbittorrent">qBittorrent</option>
                  <option value="transmission">Transmission</option>
                  <option value="aria2">aria2</option>
                </select>
              </div>

              <TextField value={name} onChange={setName} isRequired>
                <Label>{t("downloadsApp.clientName")}</Label>
                <Input placeholder="NAS qBittorrent" />
              </TextField>

              <TextField value={baseUrl} onChange={setBaseUrl} isRequired>
                <Label>{t("downloadsApp.clientUrl")}</Label>
                <Input placeholder={DEFAULT_CLIENT_URL[kind]} />
              </TextField>

              {kind !== "aria2" ? (
                <TextField value={username} onChange={setUsername}>
                  <Label>{t("downloadsApp.clientUsername")}</Label>
                  <Input placeholder="admin" />
                </TextField>
              ) : null}

              <TextField value={password} onChange={setPassword}>
                <Label>{t("downloadsApp.clientPassword")}</Label>
                <Input type="password" autoComplete="off" placeholder="••••••••" />
              </TextField>

              <TextField value={savePath} onChange={setSavePath}>
                <Label>{t("downloadsApp.clientSavePath")}</Label>
                <Input placeholder="/downloads/media" />
              </TextField>

              <div className="flex justify-end gap-2 pt-2">
                <Button variant="ghost" size="sm" onPress={() => onOpenChange(false)}>
                  {t("common.cancel")}
                </Button>
                <Button
                  type="submit"
                  size="sm"
                  variant="primary"
                  isDisabled={!name.trim() || !baseUrl.trim() || saveClient.isPending}
                >
                  {saveClient.isPending ? <Spinner size="sm" color="current" /> : null}
                  {t("downloadsApp.saveClient")}
                </Button>
              </div>
            </form>
          </Drawer.Body>
        </Drawer.Dialog>
      </Drawer.Content>
    </Drawer.Backdrop>
  );
}

export function DownloadsPage() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "zh-CN";
  const isAdmin = useIsAdmin();
  const hasFeature = useFeature("downloads");
  const { data: sysInfo } = useSystemInfo();
  const overviewQuery = useDownloadsOverview(hasFeature);
  const deleteClient = useDeleteDownloaderClient();
  const createTask = useCreateDownloadTask();
  const createSub = useCreateSubscription();
  const deleteSub = useDeleteSubscription();
  const checkSub = useCheckSubscription();

  const [clientModalOpen, setClientModalOpen] = useState(false);
  const [taskUri, setTaskUri] = useState("");
  const [selectedClientId, setSelectedClientId] = useState("");

  const [subTitle, setSubTitle] = useState("");
  const [subKind, setSubKind] = useState<SubscriptionMediaKind>("series");
  const [subQuality, setSubQuality] = useState("");
  const [subFeedUrl, setSubFeedUrl] = useState("");
  const [lastMatches, setLastMatches] = useState<{ subId: string; matches: FeedMatchItem[] } | null>(null);
  const [submittedLinks, setSubmittedLinks] = useState<Record<string, boolean>>({});

  const inset = useRowInset();

  if (!hasFeature) {
    return (
      <PageFrame>
        <EmptyState
          icon={Download}
          title={t("feature.unavailableTitle")}
          detail={t("feature.unavailableDetail", {
            version: sysInfo?.version ?? "0.1.0",
            feature: t("downloadsApp.title"),
          })}
        />
      </PageFrame>
    );
  }

  if (overviewQuery.isError) {
    return (
      <PageFrame>
        <ErrorState error={overviewQuery.error} onRetry={() => void overviewQuery.refetch()} />
      </PageFrame>
    );
  }

  const data = overviewQuery.data;
  const clients = data?.clients ?? [];
  const enabledClients = clients.filter((c) => c.enabled);
  const transfers = data?.transfers ?? [];
  const subscriptions = data?.subscriptions ?? [];

  const sendFeedMatchToDownloader = (link: string) => {
    createTask.mutate(
      {
        clientId: selectedClientId || enabledClients[0]?.id || undefined,
        uri: link,
      },
      {
        onSuccess: (res) => {
          setSubmittedLinks((prev) => ({ ...prev, [link]: true }));
          toast.success(t("downloadsApp.taskSubmitted", { name: res.clientName }));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const onSubmitTask = (event: FormEvent) => {
    event.preventDefault();
    const uri = taskUri.trim();
    if (!uri) return;
    createTask.mutate(
      {
        clientId: selectedClientId || undefined,
        uri,
      },
      {
        onSuccess: (res) => {
          setTaskUri("");
          toast.success(t("downloadsApp.taskSubmitted", { name: res.clientName }));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const onSubmitSub = (event: FormEvent) => {
    event.preventDefault();
    if (!subTitle.trim()) return;
    createSub.mutate(
      {
        title: subTitle.trim(),
        mediaKind: subKind,
        qualityRule: subQuality.trim() || undefined,
        feedUrl: subFeedUrl.trim() || undefined,
        enabled: true,
      },
      {
        onSuccess: (created) => {
          setSubTitle("");
          setSubQuality("");
          setSubFeedUrl("");
          toast.success(t("downloadsApp.subscriptionAdded", { title: created.title }));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  return (
    <PageFrame className="grid gap-6">
      <PageHeader
        title={t("downloadsApp.title")}
        summary={t("downloadsApp.summary")}
        actions={
          isAdmin ? (
            <Button size="sm" variant="primary" onPress={() => setClientModalOpen(true)}>
              <Plus className="size-4" aria-hidden />
              {t("downloadsApp.configureClient")}
            </Button>
          ) : null
        }
      />

      {overviewQuery.isPending ? (
        <LoadingRows rows={4} />
      ) : (
        <>
          <SummaryFacts
            facts={[
              { label: t("downloadsApp.clientsSection"), value: clients.length },
              { label: t("downloadsApp.transfersSection"), value: transfers.length },
              { label: t("downloadsApp.subscriptionsSection"), value: subscriptions.length },
            ]}
          />

          {isAdmin && clients.length > 0 ? (
            <Section title={t("downloadsApp.addTask")} bodyClassName="px-(--pv-pad-panel) pb-4">
              <form onSubmit={onSubmitTask} className="flex flex-wrap items-center gap-2">
                {clients.length > 1 ? (
                  <select
                    aria-label={t("downloadsApp.clientsSection")}
                    value={selectedClientId}
                    onChange={(event) => setSelectedClientId(event.target.value)}
                    className={cn(SELECT_CLASS, "sm:w-48")}
                  >
                    <option value="">{clients[0]?.name}</option>
                    {clients.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.name}
                      </option>
                    ))}
                  </select>
                ) : null}
                <Input
                  aria-label={t("downloadsApp.addTask")}
                  value={taskUri}
                  onChange={(event) => setTaskUri(event.target.value)}
                  placeholder={t("downloadsApp.uriPlaceholder")}
                  className="min-w-0 flex-1"
                />
                <Button
                  type="submit"
                  size="sm"
                  variant="primary"
                  isDisabled={!taskUri.trim() || createTask.isPending}
                >
                  {createTask.isPending ? <Spinner size="sm" color="current" /> : <Download className="size-4" aria-hidden />}
                  {t("downloadsApp.submitTask")}
                </Button>
              </form>
            </Section>
          ) : null}

          <Section title={t("downloadsApp.clientsSection")} count={clients.length}>
            {clients.length === 0 ? (
              <div className="px-(--pv-pad-panel) pb-5 text-sm text-fg-muted">
                {t("downloadsApp.clientsEmpty")}
              </div>
            ) : (
              <SectionRows>
                {clients.map((client) => (
                  <li key={client.id} className={cn("flex flex-wrap items-center justify-between gap-3 py-3", inset)}>
                    <div className="flex min-w-0 items-center gap-3">
                      <Server className="size-4 shrink-0 text-fg-muted" aria-hidden />
                      <div className="grid min-w-0 gap-0.5">
                        <div className="flex items-center gap-2">
                          <span className="font-mid text-fg">{client.name}</span>
                          <span className="rounded-control bg-sunken px-2 py-0.5 font-mono text-xs text-fg-muted">
                            {client.kind}
                          </span>
                          <span
                            className={cn(
                              "text-xs font-mid",
                              client.status === "online" ? "text-fg" : "text-fg-muted",
                            )}
                          >
                            {t(`downloadsApp.status.${client.status}`)}
                          </span>
                        </div>
                        <span className="truncate font-mono text-xs text-fg-muted">{client.baseUrl}</span>
                      </div>
                    </div>
                    {isAdmin ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        isIconOnly
                        aria-label={t("downloadsApp.deleteClient")}
                        onPress={() =>
                          deleteClient.mutate(client.id, {
                            onSuccess: () => toast.success(t("downloadsApp.clientDeleted")),
                            onError: (error) => toast.danger(errorText(t, error)),
                          })
                        }
                      >
                        <Trash2 className="size-4" aria-hidden />
                      </Button>
                    ) : null}
                  </li>
                ))}
              </SectionRows>
            )}
          </Section>

          <Section title={t("downloadsApp.transfersSection")} count={transfers.length}>
            {transfers.length === 0 ? (
              <div className="px-(--pv-pad-panel) pb-5 text-sm text-fg-muted">
                {t("downloadsApp.transfersEmpty")}
              </div>
            ) : (
              <SectionRows>
                {transfers.map((tr) => {
                  const pct = Math.round(tr.progress * 100);
                  return (
                    <li key={`${tr.clientId}-${tr.id}`} className={cn("grid gap-1.5 py-3", inset)}>
                      <div className="flex flex-wrap items-baseline justify-between gap-2">
                        <span className="min-w-0 truncate font-mid text-fg">{tr.name}</span>
                        <span className="font-mono text-xs text-fg-muted tabular-nums">
                          {pct}% · {formatBytes(tr.sizeBytes, locale)}
                          {tr.downloadSpeedBps > 0 ? ` · ${formatBytes(tr.downloadSpeedBps, locale)}/s` : ""}
                        </span>
                      </div>
                      <div className="flex items-center gap-3">
                        <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-sunken">
                          <div className="h-full bg-fg transition-all" style={{ width: `${pct}%` }} />
                        </div>
                        <span className="text-xs text-fg-muted">
                          {tr.clientName} · {t(`downloadsApp.transferState.${tr.state}`)}
                        </span>
                      </div>
                    </li>
                  );
                })}
              </SectionRows>
            )}
          </Section>

          <Section title={t("downloadsApp.subscriptionsSection")} count={subscriptions.length}>
            <div className="grid gap-4 px-(--pv-pad-panel) pb-4">
              <form onSubmit={onSubmitSub} className="grid gap-2.5 sm:grid-cols-4">
                <Input
                  aria-label={t("downloadsApp.subTitle")}
                  value={subTitle}
                  onChange={(event) => setSubTitle(event.target.value)}
                  placeholder={t("downloadsApp.subTitlePlaceholder")}
                />
                <select
                  aria-label={t("downloadsApp.subKind")}
                  value={subKind}
                  onChange={(event) => setSubKind(event.target.value as SubscriptionMediaKind)}
                  className={SELECT_CLASS}
                >
                  <option value="series">{t("downloadsApp.subKinds.series")}</option>
                  <option value="movie">{t("downloadsApp.subKinds.movie")}</option>
                  <option value="music">{t("downloadsApp.subKinds.music")}</option>
                  <option value="book">{t("downloadsApp.subKinds.book")}</option>
                </select>
                <Input
                  aria-label={t("downloadsApp.subQuality")}
                  value={subQuality}
                  onChange={(event) => setSubQuality(event.target.value)}
                  placeholder={t("downloadsApp.subQualityPlaceholder")}
                />
                <div className="flex gap-2">
                  <Input
                    aria-label={t("downloadsApp.subFeedUrl")}
                    value={subFeedUrl}
                    onChange={(event) => setSubFeedUrl(event.target.value)}
                    placeholder="https://example.com/rss.xml"
                    className="min-w-0 flex-1"
                  />
                  <Button
                    type="submit"
                    size="sm"
                    variant="secondary"
                    isDisabled={!subTitle.trim() || createSub.isPending}
                  >
                    <Plus className="size-4" aria-hidden />
                    {t("downloadsApp.addSubscription")}
                  </Button>
                </div>
              </form>

              {subscriptions.length === 0 ? (
                <p className="text-sm text-fg-muted">{t("downloadsApp.subscriptionsEmpty")}</p>
              ) : (
                <ul className="divide-y divide-line rounded-tile border border-line bg-surface">
                  {subscriptions.map((sub) => (
                    <li key={sub.id} className="grid gap-2 px-3.5 py-3">
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <div className="flex min-w-0 items-center gap-2.5">
                          <Rss className="size-4 shrink-0 text-fg-muted" aria-hidden />
                          <div className="grid min-w-0">
                            <span className="font-mid text-fg">
                              {sub.title}
                              <span className="ms-2 text-xs font-normal text-fg-muted">
                                {t(`downloadsApp.subKinds.${sub.mediaKind}`)}
                                {sub.qualityRule ? ` · ${sub.qualityRule}` : ""}
                              </span>
                            </span>
                            {sub.feedUrl ? (
                              <span className="truncate font-mono text-xs text-fg-muted">{sub.feedUrl}</span>
                            ) : null}
                          </div>
                        </div>
                        <div className="flex items-center gap-1.5">
                          {sub.feedUrl ? (
                            <Button
                              size="sm"
                              variant="outline"
                              isDisabled={checkSub.isPending}
                              onPress={() =>
                                checkSub.mutate(sub.id, {
                                  onSuccess: (res) => {
                                    setLastMatches({ subId: sub.id, matches: res.matches });
                                    toast.success(
                                      t("downloadsApp.checkedResult", {
                                        total: res.totalFeedItems,
                                        matched: res.matches.length,
                                      }),
                                    );
                                  },
                                  onError: (error) => toast.danger(errorText(t, error)),
                                })
                              }
                            >
                              <RefreshCw className="size-3.5" aria-hidden />
                              {t("downloadsApp.checkNow")}
                            </Button>
                          ) : null}
                          <Button
                            size="sm"
                            variant="ghost"
                            isIconOnly
                            aria-label={t("downloadsApp.deleteSubscription")}
                            onPress={() =>
                              deleteSub.mutate(sub.id, {
                                onError: (error) => toast.danger(errorText(t, error)),
                              })
                            }
                          >
                            <Trash2 className="size-4" aria-hidden />
                          </Button>
                        </div>
                      </div>
                      {lastMatches?.subId === sub.id && lastMatches.matches.length > 0 ? (
                        <ul className="grid gap-1.5 rounded-control bg-sunken p-2.5 text-xs">
                          {lastMatches.matches.map((item, idx) => {
                            const link = item.link?.trim() ?? "";
                            const alreadySent = Boolean(link && submittedLinks[link]);
                            return (
                              <li
                                key={`${idx}-${item.title}`}
                                className="flex flex-wrap items-center justify-between gap-2 rounded-control px-1.5 py-1 hover:bg-surface/60"
                              >
                                <div className="grid min-w-0 flex-1 gap-0.5">
                                  <span className="truncate font-mono text-fg">{item.title}</span>
                                  {item.pubDate ? (
                                    <span className="truncate text-[11px] text-fg-muted">{item.pubDate}</span>
                                  ) : null}
                                </div>
                                {isAdmin && link && enabledClients.length > 0 ? (
                                  <Button
                                    size="sm"
                                    variant={alreadySent ? "ghost" : "primary"}
                                    isDisabled={alreadySent || createTask.isPending}
                                    onPress={() => sendFeedMatchToDownloader(link)}
                                  >
                                    {alreadySent ? (
                                      <Check className="size-3.5" aria-hidden />
                                    ) : (
                                      <Download className="size-3.5" aria-hidden />
                                    )}
                                    {alreadySent ? t("downloadsApp.sentBadge") : t("downloadsApp.submitTask")}
                                  </Button>
                                ) : null}
                              </li>
                            );
                          })}
                        </ul>
                      ) : null}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </Section>
        </>
      )}

      <ClientModalDrawer open={clientModalOpen} onOpenChange={setClientModalOpen} />
    </PageFrame>
  );
}
