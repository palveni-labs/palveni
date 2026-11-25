/**
 * 应用 › 外观: the light/dark setting, the skins this browser can use (the built-in one and those
 * installed on the server) and, with the server feature "skins", the skins the registry offers.
 *
 * Everything about a skin comes from its manifest or its package info: nothing here looks at a
 * skin's identity. Installed skins are previewed live (their manifest is loaded and checked
 * first); skins not installed yet show the package's preview image or a calm stand-in.
 */
import { Button, Modal, Spinner, toast } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { AlertCircle, AlertTriangle, Download, FileWarning, Lock, PackageCheck, RotateCw, SlidersHorizontal, Trash2 } from "lucide-react";
import { useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { currentLanguage } from "@/i18n";
import { errorText } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import { useFeature } from "@/modules/access";
import { MAX_SKIN_BYTES, useInstallSkin, useInstalledSkins, useSkinRegistry, useUninstallSkin, useUploadSkin } from "@/api/queries/skins";
import type { InstalledSkin, RegistrySkin, SkinPackageInfo } from "@/api/types";
import { FeatureUnavailable } from "@/shell/feature-gate";
import { SkinLoadError, type SkinLoadFailure } from "@/skins/loader";
import { BUILTIN_SKIN, BUILTIN_SKINS, type SkinEntry } from "@/skins/registry";
import { localized } from "@/skins/schema";
import { useSkin } from "@/skins/store";
import { PageFrame } from "@/ui/frame";
import { Measure, StatusLabel } from "@/ui/marks";
import { PageHeader } from "@/ui/page";
import { Section, useRowInset, useSectionVariant } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { splitMeasure } from "./measure";
import { NavCustomizer } from "@/ui/nav-customizer";
import { NavSetting } from "@/ui/nav-setting";
import { ModeSetting } from "@/ui/mode-setting";
import { useModeSummary } from "@/ui/use-mode-summary";
import { InstallSkinDialog, UninstallSkinDialog } from "./skin-dialogs";
import { PreviewStandIn, SkinPreviewPair } from "@/ui/skin-preview";

/* ---------------------------------------------------------------- helpers */

function failureOf(error: unknown): SkinLoadFailure {
  return error instanceof SkinLoadError ? error.reason : "invalid";
}

/** The checker's findings, one per line (only for skins refused by validation). */
function findingsOf(error: unknown): string[] {
  if (!(error instanceof SkinLoadError) || error.reason !== "invalid") return [];
  return error.message.split("\n").filter(Boolean).slice(0, 12);
}

/** Loads (fetches and checks) an installed skin for its live preview; never applies it. */
function useSkinEntry(id: string) {
  const { loadSkin } = useSkin();
  return useQuery({
    queryKey: ["skins", "entry", id],
    queryFn: () => loadSkin(id),
    retry: false,
    staleTime: Infinity,
  });
}

/* ---------------------------------------------------------------- shared pieces of a skin row */

/** Content inside a section, inset the way the section's rows are. */
function Inset({ className, children }: { className?: string; children: ReactNode }) {
  const inset = useRowInset();
  return <div className={cn(inset, className)}>{children}</div>;
}

function Problem({ title, reason, findings, action }: { title: string; reason: string; findings: string[]; action?: ReactNode }) {
  const { t } = useTranslation();
  return (
    <div role="alert" className="flex items-start gap-2.5 rounded-tile bg-failed-soft px-3.5 py-2.5 text-sm">
      <AlertCircle className="mt-0.5 size-4 shrink-0 text-failed" aria-hidden />
      <div className="grid min-w-0 flex-1 gap-1">
        <p className="text-failed">
          <span className="font-strong">{title}</span> {reason}
        </p>
        {findings.length > 0 ? (
          <details className="text-failed">
            <summary className="w-fit cursor-pointer rounded-check font-mid">{t("appearance.problem.findings", { count: findings.length })}</summary>
            <ul className="mt-1.5 grid gap-1 break-words">
              {findings.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          </details>
        ) : null}
        {action ? <div className="pt-1">{action}</div> : null}
      </div>
    </div>
  );
}

function SkinGrid({ children }: { children: ReactNode }) {
  const variant = useSectionVariant();
  return <ul className={cn("grid gap-4 lg:grid-cols-2", variant === "panel" && "border-t border-line p-(--pv-pad-panel)", variant === "rule" && "pt-2")}>{children}</ul>;
}

function SkinRow({
  name,
  description,
  meta,
  preview,
  actions,
  notice,
  current = false,
}: {
  name: string;
  description: string;
  meta: ReactNode;
  preview: ReactNode;
  actions: ReactNode;
  notice?: ReactNode;
  current?: boolean;
}) {
  return (
    <li
      aria-current={current ? "true" : undefined}
      data-selected={current ? "true" : undefined}
      className={cn(
        "pv-surface-card pv-surface-card--interactive flex min-w-0 flex-col gap-3.5 rounded-panel p-4",
      )}
    >
      <div className="min-w-0">{preview}</div>
      <div className="flex min-w-0 flex-1 flex-col gap-2.5">
        <div className="flex flex-wrap items-start justify-between gap-x-3 gap-y-1">
          <div className="grid min-w-0 flex-1 gap-0.5">
            <h3 className="font-heading text-md font-title text-heading">{name}</h3>
            {description ? <p className="text-sm text-fg-muted">{description}</p> : null}
          </div>
          <div className="flex shrink-0 flex-wrap items-center gap-1.5">{actions}</div>
        </div>
        <p className="mt-auto text-xs text-fg-subtle">{meta}</p>
        {notice}
      </div>
    </li>
  );
}

function Meta({ parts }: { parts: ReactNode[] }) {
  return (
    <>
      {parts.map((part, index) => (
        <span key={index}>
          {index > 0 ? <span aria-hidden> · </span> : null}
          {part}
        </span>
      ))}
    </>
  );
}

/** Version and package size of a skin plugin. */
function usePackageMeta(info: SkinPackageInfo): ReactNode[] {
  const { t } = useTranslation();
  const version: string = t("appearance.facts.versionValue", { version: info.version });
  const { value, unit } = splitMeasure(formatBytes(info.sizeBytes, currentLanguage()));
  return [version, <Measure key="size" value={value} unit={unit} />];
}

/* ---------------------------------------------------------------- applying */

interface ApplyState {
  /** The skin whose apply failed, and why. */
  failed: { id: string; error: unknown } | null;
  /** The last skin applied from this page (announced to assistive technology). */
  applied: string | null;
  apply: (id: string) => void;
}

function useApply(): ApplyState {
  const { setSkin } = useSkin();
  const [failed, setFailed] = useState<ApplyState["failed"]>(null);
  const [applied, setApplied] = useState<string | null>(null);
  return {
    failed,
    applied,
    apply: (id) => {
      setFailed(null);
      setApplied(null);
      setSkin(id).then(
        (entry) => setApplied(localized(entry.manifest.name, currentLanguage())),
        (error: unknown) => setFailed({ id, error }),
      );
    },
  };
}

function ApplyAction({
  id,
  name,
  current,
  pending,
  disabled = false,
  onApply,
}: {
  id: string;
  name: string;
  current: boolean;
  pending: boolean;
  /** Not ready yet (its manifest is still loading). */
  disabled?: boolean;
  onApply: (id: string) => void;
}) {
  const { t } = useTranslation();
  if (current) {
    return (
      <StatusLabel tone="ok" className="h-(--pv-control-h-sm) text-sm">
        {t("appearance.inUse")}
      </StatusLabel>
    );
  }
  return (
    <Button size="sm" variant="outline" onPress={() => onApply(id)} isPending={pending} isDisabled={disabled} aria-label={pending ? undefined : t("appearance.applyNamed", { name })}>
      {pending ? <Spinner size="sm" color="current" /> : null}
      {pending ? t("appearance.applying") : t("appearance.apply")}
    </Button>
  );
}

function useReasons() {
  const { t } = useTranslation();
  return (failure: SkinLoadFailure) => t(`appearance.reasons.${failure}`);
}

/* ---------------------------------------------------------------- rows */

function BuiltinRow({ entry, applyState }: { entry: SkinEntry; applyState: ApplyState }) {
  const { t } = useTranslation();
  const language = currentLanguage();
  const { active, status, skinId } = useSkin();
  const manifest = entry.manifest;
  const name = localized(manifest.name, language);
  const current = active.manifest.id === manifest.id;
  return (
    <SkinRow
      current={current}
      name={name}
      description={localized(manifest.description, language)}
      meta={<Meta parts={[t("appearance.builtin")]} />}
      preview={<SkinPreviewPair entry={entry} />}
      actions={<ApplyAction id={manifest.id} name={name} current={current} pending={status === "loading" && skinId === manifest.id} onApply={applyState.apply} />}
    />
  );
}

function InstalledRow({ info, applyState, onUninstall }: { info: InstalledSkin; applyState: ApplyState; onUninstall?: (info: InstalledSkin) => void }) {
  const { t } = useTranslation();
  const language = currentLanguage();
  const reason = useReasons();
  const { active, status, skinId } = useSkin();
  const entry = useSkinEntry(info.id);
  const manifestName = entry.data ? localized(entry.data.manifest.name, language) : null;
  const name = manifestName || localized(info.name, language) || info.id;
  const description = entry.data ? localized(entry.data.manifest.description, language) : localized(info.description, language);
  const current = active.manifest.id === info.id && status !== "fallback";
  const pending = status === "loading" && skinId === info.id && !current;
  const failed = applyState.failed?.id === info.id ? applyState.failed.error : null;
  const packageMeta = usePackageMeta(info);

  let preview: ReactNode;
  if (entry.data) preview = <SkinPreviewPair entry={entry.data} />;
  else if (entry.isError) preview = <PreviewStandIn alt="" icon={FileWarning} note={t("appearance.problem.noPreview")} />;
  else preview = <PreviewStandIn alt="" note={t("appearance.loadingPreview")} />;

  const notice = entry.isError ? (
    <Problem
      title={t("appearance.problem.cannotUse")}
      reason={reason(failureOf(entry.error))}
      findings={findingsOf(entry.error)}
      action={
        failureOf(entry.error) === "unreachable" ? (
          <Button size="sm" variant="outline" onPress={() => void entry.refetch()}>
            <RotateCw aria-hidden />
            {t("common.retry")}
          </Button>
        ) : undefined
      }
    />
  ) : failed ? (
    <Problem title={t("appearance.problem.notApplied")} reason={reason(failureOf(failed))} findings={findingsOf(failed)} />
  ) : null;

  return (
    <SkinRow
      current={current}
      name={name}
      description={description}
      meta={<Meta parts={[t(info.source === "upload" ? "appearance.unverified" : "appearance.installedPlugin"), ...packageMeta]} />}
      preview={preview}
      notice={notice}
      actions={
        <>
          {entry.isError ? null : <ApplyAction id={info.id} name={name} current={current} pending={pending} disabled={entry.isPending} onApply={applyState.apply} />}
          {onUninstall ? (
            <Button size="sm" variant="ghost" onPress={() => onUninstall(info)} aria-label={t("appearance.uninstall.named", { name })}>
              <Trash2 aria-hidden />
              {t("appearance.uninstall.action")}
            </Button>
          ) : null}
        </>
      }
    />
  );
}

function CatalogRow({ info, onInstall }: { info: RegistrySkin; onInstall?: (info: RegistrySkin) => void }) {
  const { t } = useTranslation();
  const language = currentLanguage();
  const name = localized(info.name, language) || info.id;
  const packageMeta = usePackageMeta(info);
  return (
    <SkinRow
      name={name}
      description={localized(info.description, language)}
      meta={<Meta parts={packageMeta} />}
      preview={<PreviewStandIn src={info.previewUrl} alt={t("appearance.previewAlt", { name })} note={t("appearance.previewAfterInstall")} />}
      actions={
        onInstall ? (
          <Button size="sm" variant="primary" onPress={() => onInstall(info)} aria-label={t("appearance.install.named", { name })}>
            <Download aria-hidden />
            {t("appearance.install.action")}
          </Button>
        ) : (
          <span className="inline-flex items-center gap-1.5 text-sm text-fg-muted">
            <Lock className="size-3.5" aria-hidden />
            {t("appearance.install.adminOnly")}
          </span>
        )
      }
    />
  );
}

/* ---------------------------------------------------------------- notices */

/** The chosen skin could not be loaded at start-up; the built-in one is in use. */
function FallbackNotice({ builtinName, skinsEnabled }: { builtinName: string; skinsEnabled: boolean }) {
  const { t } = useTranslation();
  const { status, failure, skinId, setSkin } = useSkin();
  const [retrying, setRetrying] = useState(false);
  if (status !== "fallback" || !failure) return null;
  const retry = () => {
    setRetrying(true);
    setSkin(skinId)
      .catch(() => undefined)
      .finally(() => setRetrying(false));
  };
  return (
    <div role="status" className="flex flex-wrap items-start gap-x-3 gap-y-2 rounded-tile border border-degraded-line bg-degraded-soft px-4 py-3">
      <AlertTriangle className="mt-0.5 size-[1.125rem] shrink-0 text-degraded" aria-hidden />
      <div className="grid min-w-0 flex-1 basis-60 gap-0.5">
        <p className="font-strong text-fg">{t("appearance.fallback.title", { builtin: builtinName })}</p>
        <p className="text-sm text-fg">{skinsEnabled ? t(`appearance.fallback.${failure}`) : t("appearance.fallback.unavailable")}</p>
      </div>
      {failure === "unreachable" && skinsEnabled ? (
        <Button size="sm" variant="outline" onPress={retry} isPending={retrying}>
          {retrying ? <Spinner size="sm" color="current" /> : <RotateCw aria-hidden />}
          {t("common.retry")}
        </Button>
      ) : null}
    </div>
  );
}

/* ---------------------------------------------------------------- page */

export function UploadSkinButton() {
  const language = currentLanguage();
  const { t } = useTranslation();
  const upload = useUploadSkin();
  const fileInput = useRef<HTMLInputElement>(null);
  return (
    <>
      <Button size="sm" variant="outline" isPending={upload.isPending} onPress={() => fileInput.current?.click()}>
        {t("appearance.upload")}
      </Button>
      <input type="file" accept=".tar,application/x-tar" hidden ref={fileInput} onChange={(event) => {
        const file = event.currentTarget.files?.[0];
        event.currentTarget.value = "";
        if (!file) return;
        if (file.size > MAX_SKIN_BYTES) {
          toast.danger(t("appearance.tooLarge"));
          return;
        }
        upload.mutate(file, {
          onSuccess: (skin) => toast.success(t("appearance.install.done", { name: localized(skin.name, language) })),
          onError: (error) => toast.danger(errorText(t, error)),
        });
      }} />
    </>
  );
}

export function OfficialSkinCatalog({ enabled, search = "", store = false }: { enabled: boolean; search?: string; store?: boolean }) {
  const { t } = useTranslation();
  const registry = useSkinRegistry(enabled);
  const install = useInstallSkin();
  const [installing, setInstalling] = useState<RegistrySkin | null>(null);
  const language = currentLanguage();
  const available = (registry.data?.skins ?? []).filter((skin) => {
    if (skin.installed) return false;
    const fields = [skin.id, skin.pluginId, ...Object.values(skin.name), ...Object.values(skin.description), t("plugins.points.skin")];
    return fields.some((field) => field.toLocaleLowerCase().includes(search));
  });
  const confirmInstall = () => {
    if (!installing) return;
    const name = localized(installing.name, language);
    install.mutate(installing, {
      onSuccess: () => {
        setInstalling(null);
        toast.success(t("appearance.install.done", { name }));
      },
    });
  };

  const openInstall = (info: RegistrySkin) => {
    install.reset();
    setInstalling(info);
  };
  return (
    <>
      <Section title={t(store ? "plugins.available" : "appearance.catalog.title")} count={enabled && registry.data ? available.length : undefined} unframed={available.length > 0}>
        {!enabled ? (
          <FeatureUnavailable titleKey="appearance.catalog.feature" compact />
        ) : registry.isPending ? (
          <Inset>
            <LoadingRows rows={2} label={t("appearance.catalog.loading")} />
          </Inset>
        ) : registry.isError ? (
          <ErrorState compact error={registry.error} onRetry={() => void registry.refetch()} />
        ) : available.length === 0 && search ? (
          <EmptyState compact icon={PackageCheck} title={t("plugins.noMatches")} />
        ) : available.length === 0 ? (
          <EmptyState compact icon={PackageCheck} title={t("appearance.catalog.allInstalled")} detail={t("appearance.catalog.allInstalledDetail")} />
        ) : (
          <SkinGrid>
            {available.map((info) => (
              <CatalogRow key={info.id} info={info} onInstall={openInstall} />
            ))}
          </SkinGrid>
        )}
      </Section>

      <InstallSkinDialog skin={installing} onClose={() => setInstalling(null)} onConfirm={confirmInstall} busy={install.isPending} error={install.error} />
    </>
  );
}

export function AppearancePage() {
  const { t } = useTranslation();
  const language = currentLanguage();
  const skinsEnabled = useFeature("skins");
  const isAdmin = useIsAdmin();
  const { active, status, mode } = useSkin();
  const modeSummary = useModeSummary();
  const installed = useInstalledSkins(skinsEnabled);
  const uninstall = useUninstallSkin();
  const applyState = useApply();
  const [removing, setRemoving] = useState<InstalledSkin | null>(null);
  const [navEditorOpen, setNavEditorOpen] = useState(false);

  const builtinName = localized(BUILTIN_SKIN.manifest.name, language);
  const activeName = localized(active.manifest.name, language);
  const installedSkins = skinsEnabled ? (installed.data?.skins ?? []) : [];

  const confirmUninstall = () => {
    if (!removing) return;
    const name = localized(removing.name, language);
    uninstall.mutate(removing.id, {
      onSuccess: () => {
        setRemoving(null);
        toast.success(t("appearance.uninstall.done", { name }));
      },
    });
  };

  const openUninstall = (info: InstalledSkin) => {
    uninstall.reset();
    setRemoving(info);
  };

  return (
    <PageFrame>
      <PageHeader title={t("nav.appearance")} summary={t("appearance.summary", { skin: activeName, mode: modeSummary })} actions={skinsEnabled && isAdmin ? <UploadSkinButton /> : undefined} />
      <p aria-live="polite" className="sr-only">
        {applyState.applied ? t("appearance.applied", { name: applyState.applied }) : ""}
      </p>

      <div className="grid gap-(--pv-gap-section) lg:grid-cols-2">
        <Section title={t("appearance.mode.title")}>
          <Inset className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 pb-4">
            <ModeSetting />
            {mode === "system" ? <span className="text-sm text-fg-muted">{modeSummary}</span> : null}
          </Inset>
        </Section>
        <Section
          title={t("nav.layout")}
          actions={
            <Button size="sm" variant="ghost" onPress={() => setNavEditorOpen(true)}>
              <SlidersHorizontal className="size-3.5" aria-hidden />
              {t("nav.customizeButton")}
            </Button>
          }
        >
          <Inset className="flex flex-wrap items-center gap-x-4 gap-y-2 pb-4">
            <NavSetting />
          </Inset>
        </Section>
      </div>

      <Section title={t("appearance.skins.title")} count={BUILTIN_SKINS.length + installedSkins.length} unframed aria-busy={installed.isFetching || undefined}>
        {status === "fallback" && !applyState.failed ? (
          <Inset className="pb-2">
            <FallbackNotice builtinName={builtinName} skinsEnabled={skinsEnabled} />
          </Inset>
        ) : null}
        <SkinGrid>
          {BUILTIN_SKINS.map((entry) => (
            <BuiltinRow key={entry.manifest.id} entry={entry} applyState={applyState} />
          ))}
          {installedSkins.map((info) => (
            <InstalledRow key={info.id} info={info} applyState={applyState} onUninstall={isAdmin ? openUninstall : undefined} />
          ))}
        </SkinGrid>
        {skinsEnabled && installed.isPending ? (
          <Inset>
            <LoadingRows rows={2} label={t("appearance.skins.loading")} />
          </Inset>
        ) : null}
        {skinsEnabled && installed.isError ? <ErrorState compact error={installed.error} onRetry={() => void installed.refetch()} /> : null}
      </Section>

      {(!skinsEnabled || isAdmin) && <OfficialSkinCatalog enabled={skinsEnabled} />}

      <UninstallSkinDialog
        skin={removing}
        inUse={removing !== null && active.manifest.id === removing.id}
        onClose={() => setRemoving(null)}
        onConfirm={confirmUninstall}
        busy={uninstall.isPending}
        error={uninstall.error}
      />
      <Modal.Backdrop isOpen={navEditorOpen} onOpenChange={setNavEditorOpen}>
        <Modal.Container size="md">
          <Modal.Dialog aria-label={t("nav.customizeSection")}>
            <Modal.Header>
              <Modal.Heading>{t("nav.customizeSection")}</Modal.Heading>
            </Modal.Header>
            <Modal.Body className="grid gap-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="text-sm font-mid text-fg">{t("nav.layout")}</span>
                <NavSetting />
              </div>
              <NavCustomizer />
            </Modal.Body>
            <Modal.Footer>
              <Button variant="primary" size="sm" onPress={() => setNavEditorOpen(false)}>
                {t("common.done")}
              </Button>
            </Modal.Footer>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </PageFrame>
  );
}

/** Kept for the route module and older imports. */
export const ThemesPage = AppearancePage;
