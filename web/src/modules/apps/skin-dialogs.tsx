/**
 * Install consent and uninstall confirmation for skin plugins (admin). Both are decision
 * surfaces, so their content sits in a SecureSurface: if the active skin's values fail the
 * contrast rules for these pairs, the built-in values apply here.
 */
import { AlertDialog, Button, Spinner } from "@heroui/react";
import { AlertCircle, Download, ShieldCheck, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { currentLanguage } from "@/i18n";
import { formatBytes } from "@/lib/format";
import { errorText } from "@/lib/errors";
import type { SkinPackageInfo } from "@/api/types";
import { BUILTIN_SKIN } from "@/skins/registry";
import { localized } from "@/skins/schema";
import { SecureSurface } from "@/skins/secure";
import { Measure } from "@/ui/marks";
import { splitMeasure } from "./measure";

function DialogFrame({
  isOpen,
  onClose,
  busy,
  icon,
  heading,
  children,
  footer,
}: {
  isOpen: boolean;
  onClose: () => void;
  busy: boolean;
  icon: ReactNode;
  heading: string;
  children: ReactNode;
  footer: ReactNode;
}) {
  return (
    <AlertDialog.Backdrop
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
      isKeyboardDismissDisabled={busy}
    >
      <AlertDialog.Container size="md">
        <AlertDialog.Dialog className="p-0">
          <SecureSurface className="flex min-h-0 flex-col bg-raised p-6 text-fg">
            <AlertDialog.Header className="flex-row items-center gap-3">
              {icon}
              <AlertDialog.Heading className="font-heading text-lg font-title text-heading">{heading}</AlertDialog.Heading>
            </AlertDialog.Header>
            <AlertDialog.Body className="text-base text-fg-muted">{children}</AlertDialog.Body>
            <AlertDialog.Footer className="flex-wrap">{footer}</AlertDialog.Footer>
          </SecureSurface>
        </AlertDialog.Dialog>
      </AlertDialog.Container>
    </AlertDialog.Backdrop>
  );
}

function DialogError({ error, action }: { error: unknown; action: "install" | "uninstall" }) {
  const { t } = useTranslation();
  if (!error) return null;
  return (
    <p role="alert" className="mt-4 flex items-start gap-2 rounded-tile bg-failed-soft px-3 py-2 text-sm text-failed">
      <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden />
      <span>{t(`appearance.${action}.failed`, { reason: errorText(t, error) })}</span>
    </p>
  );
}

export function InstallSkinDialog({
  skin,
  onClose,
  onConfirm,
  busy,
  error,
}: {
  skin: SkinPackageInfo | null;
  onClose: () => void;
  onConfirm: () => void;
  busy: boolean;
  error: unknown;
}) {
  const { t } = useTranslation();
  const language = currentLanguage();
  const name = skin ? localized(skin.name, language) : "";
  return (
    <DialogFrame
      isOpen={skin !== null}
      onClose={onClose}
      busy={busy}
      icon={
        <span className="grid size-10 shrink-0 place-items-center rounded-tile bg-sunken text-fg-muted">
          <ShieldCheck className="size-5" strokeWidth={1.8} aria-hidden />
        </span>
      }
      heading={t("appearance.install.title", { name })}
      footer={
        <>
          <Button slot="close" variant="secondary" isDisabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" className="button--confirm" onPress={onConfirm} isPending={busy}>
            {busy ? <Spinner size="sm" color="current" /> : <Download aria-hidden />}
            {busy ? t("appearance.install.pending") : t("appearance.install.confirm")}
          </Button>
        </>
      }
    >
      <p>{t("appearance.install.dataOnly")}</p>
      <p className="mt-2">{t("appearance.install.everyone")}</p>
      {skin ? (
        <dl className="mt-4 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1.5 rounded-tile bg-sunken px-3.5 py-3 text-sm">
          <dt className="text-fg-muted">{t("appearance.facts.plugin")}</dt>
          <dd className="truncate font-mono text-fg" title={skin.pluginId}>
            {skin.pluginId}
          </dd>
          <dt className="text-fg-muted">{t("appearance.facts.version")}</dt>
          <dd className="text-fg tabular-nums">{skin.version}</dd>
          <dt className="text-fg-muted">{t("appearance.facts.size")}</dt>
          <dd className="text-fg">
            <Measure {...splitMeasure(formatBytes(skin.sizeBytes, language))} />
          </dd>
          <dt className="text-fg-muted">{t("appearance.facts.permissions")}</dt>
          <dd className="text-fg">{t("appearance.install.noPermissions")}</dd>
        </dl>
      ) : null}
      <DialogError error={error} action="install" />
    </DialogFrame>
  );
}

export function UninstallSkinDialog({
  skin,
  inUse,
  onClose,
  onConfirm,
  busy,
  error,
}: {
  skin: SkinPackageInfo | null;
  inUse: boolean;
  onClose: () => void;
  onConfirm: () => void;
  busy: boolean;
  error: unknown;
}) {
  const { t } = useTranslation();
  const language = currentLanguage();
  const name = skin ? localized(skin.name, language) : "";
  const builtin = localized(BUILTIN_SKIN.manifest.name, language);
  return (
    <DialogFrame
      isOpen={skin !== null}
      onClose={onClose}
      busy={busy}
      icon={
        <span className="grid size-10 shrink-0 place-items-center rounded-tile bg-failed-soft text-failed">
          <Trash2 className="size-5" strokeWidth={1.8} aria-hidden />
        </span>
      }
      heading={t("appearance.uninstall.title", { name })}
      footer={
        <>
          <Button slot="close" variant="secondary" isDisabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button variant="danger" onPress={onConfirm} isPending={busy}>
            {busy ? <Spinner size="sm" color="current" /> : <Trash2 aria-hidden />}
            {busy ? t("appearance.uninstall.pending") : t("appearance.uninstall.confirm")}
          </Button>
        </>
      }
    >
      <p>{t("appearance.uninstall.detail", { builtin })}</p>
      {inUse ? <p className="mt-2 font-mid text-fg">{t("appearance.uninstall.inUse", { builtin })}</p> : null}
      <DialogError error={error} action="uninstall" />
    </DialogFrame>
  );
}
