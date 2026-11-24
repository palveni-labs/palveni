import { Button, Chip, Input, Spinner, toast } from "@heroui/react";
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Activity,
  Archive,
  Check,
  Copy,
  Download,
  FileUp,
  History,
  KeyRound,
  RefreshCw,
  RotateCcw,
  ScrollText,
  Server,
  ShieldCheck,
  Trash2,
  Upload,
} from "lucide-react";
import { useRef, useState, type ChangeEvent, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import { queryKeys } from "@/api/query-keys";
import type {
  BackupSummary,
  DoctorStatus,
  ExternalImportResult,
  ExternalImportSource,
  ServerLogLevel,
} from "@/api/types";
import { RelativeTime } from "@/components/relative-time";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import { useSecureSurface } from "@/skins/secure";
import { PageFrame } from "@/ui/frame";
import { StatusLabel, type StatusTone } from "@/ui/marks";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

const IMPORT_SOURCES: ExternalImportSource[] = ["auto", "jellyfin", "emby", "navidrome", "m3u"];
const AUDIT_FILTERS = ["", "auth.", "user.", "library.", "system.", "metadata."] as const;
const LOG_LEVEL_FILTERS = ["all", "info", "warn", "error"] as const;
const METADATA_LANGUAGES = ["zh-CN", "zh-TW", "en-US", "ja-JP"] as const;

function doctorTone(status: DoctorStatus): StatusTone {
  switch (status) {
    case "healthy":
      return "ok";
    case "degraded":
      return "degraded";
    case "unhealthy":
      return "failed";
  }
}

function logLevelChipClass(level: ServerLogLevel): string {
  switch (level) {
    case "error":
      return "bg-sunken text-failed ring-1 ring-failed/30";
    case "warn":
      return "bg-sunken text-degraded ring-1 ring-degraded/30";
    case "info":
      return "bg-sunken text-fg-muted ring-1 ring-line";
  }
}

function SystemDoctorSection() {
  const { t } = useTranslation();
  const inset = useRowInset();

  const doctorQuery = useQuery({
    queryKey: queryKeys.maintenance.doctor,
    queryFn: ({ signal }) => api.observability.doctor(signal),
  });

  const report = doctorQuery.data;

  return (
    <Section
      title={t("maintenance.doctor.title")}
      count={report?.checks.length}
      actions={
        <div className="flex flex-wrap items-center gap-3">
          {report ? (
            <StatusLabel tone={doctorTone(report.overallStatus)} className="text-sm">
              {t(`maintenance.doctor.status.${report.overallStatus}`)}
            </StatusLabel>
          ) : null}
          <Button
            size="sm"
            variant="outline"
            isPending={doctorQuery.isFetching}
            onPress={() => void doctorQuery.refetch()}
          >
            {doctorQuery.isFetching ? (
              <Spinner size="sm" color="current" />
            ) : (
              <RefreshCw className="size-3.5" aria-hidden />
            )}
            {t("maintenance.doctor.refresh")}
          </Button>
        </div>
      }
    >
      {doctorQuery.isPending ? (
        <LoadingRows rows={3} className={cn("pb-4", inset)} />
      ) : doctorQuery.isError ? (
        <ErrorState compact error={doctorQuery.error} onRetry={() => void doctorQuery.refetch()} />
      ) : report ? (
        <>
          <div className={cn("flex items-center gap-2 pb-2.5 text-xs text-fg-muted", inset)}>
            <Activity className="size-3.5" aria-hidden />
            <span>{t("maintenance.doctor.checkedAt")}</span>
            <RelativeTime value={report.generatedAt} />
          </div>
          <SectionRows>
            {report.checks.map((check) => (
              <li
                key={check.id}
                data-doctor-check={check.id}
                className={cn("grid gap-1 py-3", inset)}
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-strong text-fg">{check.name}</span>
                  <StatusLabel tone={doctorTone(check.status)} lamp className="text-xs">
                    {t(`maintenance.doctor.status.${check.status}`)}
                  </StatusLabel>
                </div>
                <p className="text-sm text-fg">{check.summary}</p>
                <p className="font-mono text-xs text-fg-muted">{check.detail}</p>
                {check.remediation ? (
                  <p className="text-xs text-degraded">{check.remediation}</p>
                ) : null}
              </li>
            ))}
          </SectionRows>
        </>
      ) : null}
    </Section>
  );
}

function LiveLogsSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const [level, setLevel] = useState<(typeof LOG_LEVEL_FILTERS)[number]>("all");

  const logsQuery = useQuery({
    queryKey: queryKeys.maintenance.logs(level),
    queryFn: ({ signal }) => api.observability.logs({ level, limit: 100 }, signal),
  });

  const entries = logsQuery.data?.entries ?? [];

  return (
    <Section
      title={t("maintenance.logs.title")}
      count={entries.length > 0 ? entries.length : undefined}
      actions={
        <div className="flex flex-wrap items-center gap-2">
          <div role="group" aria-label={t("maintenance.logs.levelLabel")} className="inline-flex rounded-control border border-line bg-sunken p-0.5">
            {LOG_LEVEL_FILTERS.map((lv) => (
              <button
                key={lv}
                type="button"
                aria-pressed={level === lv}
                onClick={() => setLevel(lv)}
                className={cn(
                  "rounded-control px-2.5 py-1 text-xs font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
                  level === lv
                    ? "bg-surface text-heading font-strong shadow-xs"
                    : "text-fg-muted hover:text-fg",
                )}
              >
                {t(`maintenance.logs.levels.${lv}`)}
              </button>
            ))}
          </div>
          <Button
            size="sm"
            variant="outline"
            isPending={logsQuery.isFetching}
            onPress={() => void logsQuery.refetch()}
          >
            {logsQuery.isFetching ? (
              <Spinner size="sm" color="current" />
            ) : (
              <RefreshCw className="size-3.5" aria-hidden />
            )}
            {t("maintenance.logs.refresh")}
          </Button>
        </div>
      }
    >
      {logsQuery.isPending ? (
        <LoadingRows rows={3} className={cn("pb-4", inset)} />
      ) : logsQuery.isError ? (
        <ErrorState compact error={logsQuery.error} onRetry={() => void logsQuery.refetch()} />
      ) : entries.length === 0 ? (
        <EmptyState compact icon={ScrollText} title={t("maintenance.logs.empty")} />
      ) : (
        <div className="max-h-80 overflow-y-auto border-t border-line">
          <ul className="divide-y divide-line">
            {entries.map((entry) => (
              <li
                key={entry.id}
                data-log-level={entry.level}
                className={cn("grid gap-1 py-2.5 text-xs sm:grid-cols-[auto_auto_auto_1fr] sm:items-baseline sm:gap-3", inset)}
              >
                <span className="whitespace-nowrap text-fg-muted">
                  <RelativeTime value={entry.timestamp} />
                </span>
                <Chip size="sm" variant="soft" className={cn("uppercase", logLevelChipClass(entry.level))}>
                  {entry.level}
                </Chip>
                <code className="rounded-control bg-sunken px-1.5 py-0.5 font-mono text-[11px] text-fg-muted">
                  {entry.target}
                </code>
                <span className="min-w-0 break-all font-mono text-fg">{entry.message}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </Section>
  );
}

function MetadataConfigSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const queryClient = useQueryClient();

  const configQuery = useQuery({
    queryKey: queryKeys.maintenance.metadataConfig,
    queryFn: ({ signal }) => api.metadata.config(signal),
  });

  const [tmdbApiKey, setTmdbApiKey] = useState("");
  const [bangumiToken, setBangumiToken] = useState("");
  const [langOverride, setLangOverride] = useState<string | null>(null);
  const [autoPosterOverride, setAutoPosterOverride] = useState<boolean | null>(null);

  const cfg = configQuery.data;
  const preferredLanguage = langOverride ?? cfg?.preferredLanguage ?? "zh-CN";
  const autoFetchPosters = autoPosterOverride ?? cfg?.autoFetchPosters ?? true;

  const saveConfig = useMutation({
    mutationFn: (payload: {
      tmdbApiKey?: string;
      clearTmdbKey?: boolean;
      bangumiToken?: string;
      clearBangumiToken?: boolean;
      preferredLanguage?: string;
      autoFetchPosters?: boolean;
    }) => api.metadata.updateConfig(payload),
    onSuccess: (updated) => {
      setTmdbApiKey("");
      setBangumiToken("");
      setLangOverride(null);
      setAutoPosterOverride(null);
      queryClient.setQueryData(queryKeys.maintenance.metadataConfig, updated);
      toast.success(t("maintenance.metadataConfig.savedToast"));
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.logsAll });
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    saveConfig.mutate({
      tmdbApiKey: tmdbApiKey.trim() || undefined,
      bangumiToken: bangumiToken.trim() || undefined,
      preferredLanguage,
      autoFetchPosters,
    });
  };

  return (
    <Section title={t("maintenance.metadataConfig.title")}>
      {configQuery.isPending ? (
        <LoadingRows rows={2} className={cn("pb-4", inset)} />
      ) : configQuery.isError ? (
        <ErrorState compact error={configQuery.error} onRetry={() => void configQuery.refetch()} />
      ) : (
        <form onSubmit={handleSubmit} className={cn("grid gap-4 pb-4", inset)}>
          <div className="grid gap-3 md:grid-cols-2">
            <div className="grid gap-1.5">
              <div className="flex items-center justify-between gap-2">
                <label htmlFor="tmdb-key-input" className="text-sm font-mid text-fg">
                  {t("maintenance.metadataConfig.tmdbKey")}
                </label>
                <div className="flex items-center gap-1.5">
                  <Chip
                    size="sm"
                    variant="soft"
                    className={cfg?.tmdbConfigured ? "bg-ok-soft text-ok" : "bg-sunken text-fg-muted"}
                  >
                    {t(
                      cfg?.tmdbConfigured
                        ? "maintenance.metadataConfig.configured"
                        : "maintenance.metadataConfig.notConfigured",
                    )}
                  </Chip>
                  {cfg?.tmdbConfigured ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={saveConfig.isPending}
                      onPress={() => saveConfig.mutate({ clearTmdbKey: true })}
                    >
                      {t("maintenance.metadataConfig.clearKey")}
                    </Button>
                  ) : null}
                </div>
              </div>
              <Input
                id="tmdb-key-input"
                type="password"
                autoComplete="off"
                placeholder={t("maintenance.metadataConfig.tmdbPlaceholder")}
                value={tmdbApiKey}
                onChange={(e) => setTmdbApiKey(e.target.value)}
              />
            </div>

            <div className="grid gap-1.5">
              <div className="flex items-center justify-between gap-2">
                <label htmlFor="bangumi-token-input" className="text-sm font-mid text-fg">
                  {t("maintenance.metadataConfig.bangumiToken")}
                </label>
                <div className="flex items-center gap-1.5">
                  <Chip
                    size="sm"
                    variant="soft"
                    className={cfg?.bangumiConfigured ? "bg-ok-soft text-ok" : "bg-sunken text-fg-muted"}
                  >
                    {t(
                      cfg?.bangumiConfigured
                        ? "maintenance.metadataConfig.configured"
                        : "maintenance.metadataConfig.notConfigured",
                    )}
                  </Chip>
                  {cfg?.bangumiConfigured ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={saveConfig.isPending}
                      onPress={() => saveConfig.mutate({ clearBangumiToken: true })}
                    >
                      {t("maintenance.metadataConfig.clearKey")}
                    </Button>
                  ) : null}
                </div>
              </div>
              <Input
                id="bangumi-token-input"
                type="password"
                autoComplete="off"
                placeholder={t("maintenance.metadataConfig.bangumiPlaceholder")}
                value={bangumiToken}
                onChange={(e) => setBangumiToken(e.target.value)}
              />
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-wrap items-center gap-4">
              <label className="flex items-center gap-2 text-sm text-fg">
                <span>{t("maintenance.metadataConfig.preferredLanguage")}</span>
                <select
                  aria-label={t("maintenance.metadataConfig.preferredLanguage")}
                  value={preferredLanguage}
                  onChange={(e) => setLangOverride(e.target.value)}
                  className="rounded-control border border-line bg-surface px-2.5 py-1.5 text-sm text-fg"
                >
                  {METADATA_LANGUAGES.map((lang) => (
                    <option key={lang} value={lang}>
                      {lang}
                    </option>
                  ))}
                </select>
              </label>

              <label className="inline-flex cursor-pointer items-center gap-2 text-sm text-fg">
                <input
                  type="checkbox"
                  checked={autoFetchPosters}
                  onChange={(e) => setAutoPosterOverride(e.target.checked)}
                  className="size-4 rounded border-line"
                />
                <span>{t("maintenance.metadataConfig.autoFetchPosters")}</span>
              </label>
            </div>

            <Button type="submit" size="sm" variant="primary" isPending={saveConfig.isPending}>
              {saveConfig.isPending ? (
                <Spinner size="sm" color="current" />
              ) : (
                <KeyRound className="size-3.5" aria-hidden />
              )}
              {t("maintenance.metadataConfig.save")}
            </Button>
          </div>
        </form>
      )}
    </Section>
  );
}

function ProtocolEndpointsSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const origin = typeof window !== "undefined" ? window.location.origin : "http://localhost:8080";
  const endpoints = [
    {
      id: "webdav",
      label: t("maintenance.endpoints.webdavLabel"),
      hint: t("maintenance.endpoints.webdavHint"),
      url: `${origin}/webdav/`,
    },
    {
      id: "prometheus",
      label: t("maintenance.endpoints.prometheusLabel"),
      hint: t("maintenance.endpoints.prometheusHint"),
      url: `${origin}/api/v1/system/prometheus`,
    },
  ];

  const handleCopy = async (id: string, url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      setCopiedId(id);
      toast.success(t("maintenance.endpoints.copiedToast"));
      setTimeout(() => setCopiedId((prev) => (prev === id ? null : prev)), 2000);
    } catch {
      toast.success(t("maintenance.endpoints.copiedToast"));
    }
  };

  return (
    <Section title={t("maintenance.endpoints.title")}>
      <SectionRows>
        {endpoints.map((ep) => (
          <li key={ep.id} className={cn("grid gap-2 py-3.5", inset)}>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2 text-sm font-strong text-fg">
                <Server className="size-4 text-fg-muted" aria-hidden />
                <span>{ep.label}</span>
              </div>
              <Button
                size="sm"
                variant="outline"
                onPress={() => void handleCopy(ep.id, ep.url)}
              >
                {copiedId === ep.id ? (
                  <Check className="size-3.5 text-ok" aria-hidden />
                ) : (
                  <Copy className="size-3.5" aria-hidden />
                )}
                {t("maintenance.endpoints.copy")}
              </Button>
            </div>
            <code className="block overflow-x-auto rounded-control border border-line bg-sunken px-3 py-2 font-mono text-xs text-fg">
              {ep.url}
            </code>
            <p className="text-xs text-fg-muted">{ep.hint}</p>
          </li>
        ))}
      </SectionRows>
    </Section>
  );
}

function BackupsSection() {
  const { t, i18n } = useTranslation();
  const inset = useRowInset();
  const queryClient = useQueryClient();
  const [label, setLabel] = useState("");
  const [confirmRestoreId, setConfirmRestoreId] = useState<string | null>(null);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);

  const backupsQuery = useQuery({
    queryKey: queryKeys.maintenance.backups,
    queryFn: ({ signal }) => api.maintenance.backups(signal),
  });

  const createBackup = useMutation({
    mutationFn: () => api.maintenance.createBackup({ name: label.trim() || undefined }),
    onSuccess: (created) => {
      setLabel("");
      toast.success(t("maintenance.backups.created", { name: created.name }));
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.backups });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const restoreBackup = useMutation({
    mutationFn: (id: string) => api.maintenance.restoreBackup(id),
    onSuccess: (result) => {
      setConfirmRestoreId(null);
      toast.success(
        t("maintenance.backups.restored", {
          libraries: result.librariesRestored,
          states: result.itemStatesRestored,
        }),
      );
      void queryClient.invalidateQueries({ queryKey: queryKeys.libraries.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.system.settings });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const deleteBackup = useMutation({
    mutationFn: (id: string) => api.maintenance.deleteBackup(id),
    onSuccess: () => {
      toast.success(t("maintenance.backups.deleted"));
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.backups });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const handleDownload = async (backup: BackupSummary) => {
    setDownloadingId(backup.id);
    try {
      const blob = await api.maintenance.downloadBackup(backup.id);
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `palveni-backup-${backup.id}.json`;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
    } catch (err) {
      toast.danger(errorText(t, err));
    } finally {
      setDownloadingId(null);
    }
  };

  const backups = backupsQuery.data?.backups ?? [];

  return (
    <Section title={t("maintenance.backups.title")} count={backups.length}>
      <div className={cn("flex flex-wrap items-center gap-2.5 pb-3.5", inset)}>
        <Input
          aria-label={t("maintenance.backups.namePlaceholder")}
          placeholder={t("maintenance.backups.namePlaceholder")}
          maxLength={128}
          value={label}
          onChange={(event) => setLabel(event.target.value)}
          className="min-w-56 flex-1"
        />
        <Button
          variant="primary"
          size="sm"
          isPending={createBackup.isPending}
          onPress={() => createBackup.mutate()}
        >
          {createBackup.isPending ? <Spinner size="sm" color="current" /> : <Archive aria-hidden />}
          {t("maintenance.backups.create")}
        </Button>
      </div>

      {backupsQuery.isPending ? (
        <LoadingRows rows={2} className={cn("pb-4", inset)} />
      ) : backupsQuery.isError ? (
        <ErrorState compact error={backupsQuery.error} onRetry={() => void backupsQuery.refetch()} />
      ) : backups.length === 0 ? (
        <EmptyState
          compact
          icon={Archive}
          title={t("maintenance.backups.empty")}
          detail={t("maintenance.backups.emptyDetail")}
        />
      ) : (
        <SectionRows>
          {backups.map((backup) => (
            <li
              key={backup.id}
              data-backup-row={backup.id}
              className={cn("flex flex-wrap items-center justify-between gap-4 py-3.5", inset)}
            >
              <div className="grid min-w-0 gap-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-strong text-fg">{backup.name}</span>
                  <code className="rounded-control bg-sunken px-1.5 py-0.5 font-mono text-xs text-fg-muted">
                    {backup.sha256.slice(0, 12)}
                  </code>
                </div>
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-fg-muted">
                  <span data-num className="tabular-nums">
                    {formatBytes(backup.sizeBytes, i18n.resolvedLanguage ?? "en")}
                  </span>
                  <span aria-hidden>·</span>
                  <span>
                    {t("maintenance.backups.counts", {
                      libraries: backup.counts.libraries,
                      playlists: backup.counts.playlists,
                      states: backup.counts.itemStates,
                      bookmarks: backup.counts.bookmarks,
                    })}
                  </span>
                  <span aria-hidden>·</span>
                  <RelativeTime value={backup.createdAt} />
                  {backup.creatorUsername ? (
                    <>
                      <span aria-hidden>·</span>
                      <span>{backup.creatorUsername}</span>
                    </>
                  ) : null}
                </div>
              </div>

              <div className="flex flex-wrap items-center gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  isPending={downloadingId === backup.id}
                  onPress={() => void handleDownload(backup)}
                >
                  <Download aria-hidden />
                  {t("maintenance.backups.download")}
                </Button>

                {confirmRestoreId === backup.id ? (
                  <>
                    <Button
                      size="sm"
                      variant="outline"
                      isDisabled={restoreBackup.isPending}
                      onPress={() => setConfirmRestoreId(null)}
                    >
                      {t("common.cancel")}
                    </Button>
                    <Button
                      size="sm"
                      variant="primary"
                      isPending={restoreBackup.isPending}
                      onPress={() => restoreBackup.mutate(backup.id)}
                    >
                      <RotateCcw aria-hidden />
                      {t("maintenance.backups.confirmRestore")}
                    </Button>
                  </>
                ) : (
                  <Button
                    size="sm"
                    variant="outline"
                    onPress={() => setConfirmRestoreId(backup.id)}
                  >
                    <RotateCcw aria-hidden />
                    {t("maintenance.backups.restore")}
                  </Button>
                )}

                <Button
                  size="sm"
                  variant="outline"
                  isDisabled={deleteBackup.isPending}
                  aria-label={`${t("common.delete")} ${backup.name}`}
                  onPress={() => deleteBackup.mutate(backup.id)}
                >
                  <Trash2 aria-hidden />
                  {t("common.delete")}
                </Button>
              </div>
            </li>
          ))}
        </SectionRows>
      )}
    </Section>
  );
}

function ImporterSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const queryClient = useQueryClient();
  const fileRef = useRef<HTMLInputElement | null>(null);

  const [source, setSource] = useState<ExternalImportSource>("auto");
  const [playlistName, setPlaylistName] = useState("");
  const [content, setContent] = useState("");
  const [result, setResult] = useState<ExternalImportResult | null>(null);

  const runImport = useMutation({
    mutationFn: (dryRun: boolean) =>
      api.maintenance.importExternal({
        source,
        content,
        playlistName: playlistName.trim() || undefined,
        dryRun,
      }),
    onSuccess: (res) => {
      setResult(res);
      if (!res.dryRun) {
        toast.success(t("maintenance.importer.imported", { matched: res.matchedItems }));
        void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
      }
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const handleFileChange = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    const text = await file.text();
    setContent(text);
    setResult(null);
    if (!playlistName && /\.(m3u8?)$/i.test(file.name)) {
      setPlaylistName(file.name.replace(/\.(m3u8?)$/i, ""));
    }
    event.target.value = "";
  };

  return (
    <Section title={t("maintenance.importer.title")}>
      <div className={cn("grid gap-3.5 pb-4", inset)}>
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-sm text-fg">
            <span>{t("maintenance.importer.source")}</span>
            <select
              aria-label={t("maintenance.importer.source")}
              className="rounded-control border border-line bg-surface px-3 py-1.5 text-sm text-fg"
              value={source}
              onChange={(event) => {
                const picked = IMPORT_SOURCES.find((item) => item === event.target.value);
                if (picked) {
                  setSource(picked);
                  setResult(null);
                }
              }}
            >
              {IMPORT_SOURCES.map((item) => (
                <option key={item} value={item}>
                  {t(`maintenance.importer.sources.${item}`)}
                </option>
              ))}
            </select>
          </label>

          <Input
            aria-label={t("maintenance.importer.playlistName")}
            placeholder={t("maintenance.importer.playlistName")}
            maxLength={128}
            value={playlistName}
            onChange={(event) => setPlaylistName(event.target.value)}
            className="min-w-48 flex-1"
          />

          <input
            ref={fileRef}
            type="file"
            accept=".json,.m3u,.m3u8,.txt"
            className="sr-only"
            onChange={(event) => void handleFileChange(event)}
          />
          <Button variant="outline" size="sm" onPress={() => fileRef.current?.click()}>
            <FileUp aria-hidden />
            {t("maintenance.importer.uploadFile")}
          </Button>
        </div>

        <textarea
          aria-label={t("maintenance.importer.contentLabel")}
          placeholder={t("maintenance.importer.contentPlaceholder")}
          rows={5}
          value={content}
          onChange={(event) => {
            setContent(event.target.value);
            setResult(null);
          }}
          className="w-full rounded-control border border-line bg-surface p-3 font-mono text-sm text-fg placeholder:text-fg-subtle focus:outline-none"
        />

        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            variant="outline"
            size="sm"
            isDisabled={!content.trim() || runImport.isPending}
            onPress={() => runImport.mutate(true)}
          >
            <History aria-hidden />
            {t("maintenance.importer.preview")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            isDisabled={!content.trim() || runImport.isPending}
            onPress={() => runImport.mutate(false)}
          >
            {runImport.isPending ? <Spinner size="sm" color="current" /> : <Upload aria-hidden />}
            {t("maintenance.importer.apply")}
          </Button>
        </div>

        {result ? (
          <div className="grid gap-2.5 rounded-tile border border-line bg-sunken p-3.5">
            <div className="flex flex-wrap items-center justify-between gap-2 text-sm">
              <span className="font-strong text-fg">
                {t(`maintenance.importer.sources.${result.detectedSource}`)}
              </span>
              <span className="text-fg-muted">
                {t("maintenance.importer.stats", {
                  total: result.totalEntries,
                  matched: result.matchedItems,
                  unmatched: result.unmatchedEntries,
                  favorites: result.favoritesImported,
                  history: result.historyImported,
                  tracks: result.playlistTracksImported,
                })}
              </span>
            </div>
            {result.samples.length > 0 ? (
              <ul className="grid max-h-48 divide-y divide-line overflow-y-auto text-sm">
                {result.samples.map((sample, idx) => (
                  <li key={`${sample.title}-${idx}`} className="flex items-center justify-between gap-3 py-1.5">
                    <span className="truncate text-fg">
                      {sample.title}
                      {sample.matchedTitle && sample.matchedTitle !== sample.title
                        ? ` → ${sample.matchedTitle}`
                        : ""}
                    </span>
                    <Chip
                      size="sm"
                      variant="soft"
                      className={sample.matched ? "bg-ok-soft text-ok" : "bg-surface text-fg-muted"}
                    >
                      {t(sample.matched ? "maintenance.importer.matchedBadge" : "maintenance.importer.unmatchedBadge")}
                    </Chip>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        ) : null}
      </div>
    </Section>
  );
}

function AuditLogSection() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const [actionFilter, setActionFilter] = useState("");

  const auditQuery = useInfiniteQuery({
    queryKey: queryKeys.maintenance.audit(actionFilter),
    queryFn: ({ pageParam, signal }) =>
      api.maintenance.audit({ action: actionFilter || undefined, cursor: pageParam, limit: 30 }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor,
  });

  const entries = auditQuery.data?.pages.flatMap((page) => page.entries) ?? [];

  return (
    <Section
      title={t("maintenance.audit.title")}
      count={entries.length > 0 ? entries.length : undefined}
      actions={
        <select
          aria-label={t("maintenance.audit.filterLabel")}
          className="rounded-control border border-line bg-surface px-2.5 py-1 text-sm text-fg"
          value={actionFilter}
          onChange={(event) => setActionFilter(event.target.value)}
        >
          {AUDIT_FILTERS.map((prefix) => (
            <option key={prefix || "all"} value={prefix}>
              {prefix ? prefix : t("maintenance.audit.allActions")}
            </option>
          ))}
        </select>
      }
    >
      {auditQuery.isPending ? (
        <LoadingRows rows={4} className={cn("pb-4", inset)} />
      ) : auditQuery.isError ? (
        <ErrorState compact error={auditQuery.error} onRetry={() => void auditQuery.refetch()} />
      ) : entries.length === 0 ? (
        <EmptyState compact icon={ShieldCheck} title={t("maintenance.audit.empty")} />
      ) : (
        <div className="overflow-x-auto border-t border-line">
          <table className="w-full border-collapse text-start text-sm">
            <thead>
              <tr className="border-b border-line text-fg-muted">
                <th className={cn("py-2.5 pe-4 text-start font-mid", inset)}>{t("maintenance.audit.columns.time")}</th>
                <th className="py-2.5 pe-4 text-start font-mid">{t("maintenance.audit.columns.actor")}</th>
                <th className="py-2.5 pe-4 text-start font-mid">{t("maintenance.audit.columns.action")}</th>
                <th className="py-2.5 pe-4 text-start font-mid">{t("maintenance.audit.columns.target")}</th>
                <th className="py-2.5 pe-4 text-start font-mid">{t("maintenance.audit.columns.detail")}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-line">
              {entries.map((entry) => (
                <tr key={entry.id} className="align-top">
                  <td className={cn("py-2.5 pe-4 whitespace-nowrap text-fg-muted", inset)}>
                    <RelativeTime value={entry.occurredAt} />
                  </td>
                  <td className="py-2.5 pe-4 whitespace-nowrap text-fg">
                    {entry.actorUsername ?? t("maintenance.audit.systemActor")}
                  </td>
                  <td className="py-2.5 pe-4 whitespace-nowrap">
                    <code className="rounded-control bg-sunken px-1.5 py-0.5 font-mono text-xs text-fg">
                      {entry.action}
                    </code>
                  </td>
                  <td className="py-2.5 pe-4 break-all text-fg">{entry.target ?? "—"}</td>
                  <td className="py-2.5 pe-4 font-mono text-xs break-all text-fg-muted">
                    {entry.detail ?? "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {auditQuery.hasNextPage ? (
            <div className="flex justify-center border-t border-line py-3">
              <Button
                variant="outline"
                size="sm"
                isPending={auditQuery.isFetchingNextPage}
                onPress={() => void auditQuery.fetchNextPage()}
              >
                {t("maintenance.audit.loadMore")}
              </Button>
            </div>
          ) : null}
        </div>
      )}
    </Section>
  );
}

/** 系统 › 安全与维护: diagnostics, live logs, metadata scrapers, WebDAV/Prometheus, snapshots, importer, and security audit log. */
export function MaintenancePage() {
  const { t } = useTranslation();
  const secure = useSecureSurface();
  return (
    <PageFrame>
      <PageHeader title={t("nav.maintenance")} summary={t("maintenance.summary")} />
      <div {...secure} className="grid gap-(--pv-gap-section)">
        <SystemDoctorSection />
        <LiveLogsSection />
        <MetadataConfigSection />
        <ProtocolEndpointsSection />
        <BackupsSection />
        <ImporterSection />
        <AuditLogSection />
      </div>
    </PageFrame>
  );
}
