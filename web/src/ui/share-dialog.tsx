import { Button, buttonVariants, toast } from "@heroui/react";
import { Check, Copy, ExternalLink, Link2, ShieldCheck, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useCreateShare,
  type ShareLinkSummary,
  type ShareTargetKind,
} from "@/api/queries/shares-collections";
import { errorText } from "@/lib/errors";
import { FieldInput, Select } from "@/ui/controls";

export interface ShareDialogProps {
  isOpen: boolean;
  onClose: () => void;
  targetKind: ShareTargetKind;
  targetId: string;
  libraryId?: string | null;
  defaultTitle: string;
}

export function ShareDialog({
  isOpen,
  onClose,
  targetKind,
  targetId,
  libraryId,
  defaultTitle,
}: ShareDialogProps) {
  const { t } = useTranslation();
  const createShare = useCreateShare();

  const [title, setTitle] = useState(defaultTitle);
  const [password, setPassword] = useState("");
  const [expiryHours, setExpiryHours] = useState<string>("168");
  const [maxViews, setMaxViews] = useState<string>("");
  const [allowDownload, setAllowDownload] = useState(false);
  const [stripGps, setStripGps] = useState(true);
  const [created, setCreated] = useState<ShareLinkSummary | null>(null);
  const [copied, setCopied] = useState(false);

  if (!isOpen) return null;

  const fullUrl = created ? `${window.location.origin}${created.sharePath}` : "";

  const handleCopy = async () => {
    if (!fullUrl) return;
    try {
      await navigator.clipboard.writeText(fullUrl);
      setCopied(true);
      toast.success(t("sharesApp.copiedToast"));
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback remains selectable in input
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const parsedViews = maxViews.trim() ? Number.parseInt(maxViews.trim(), 10) : null;
    const hours = expiryHours === "never" ? null : Number.parseInt(expiryHours, 10);

    createShare.mutate(
      {
        targetKind,
        targetId,
        libraryId: libraryId ?? null,
        title: title.trim() || defaultTitle,
        password: password || null,
        allowDownload,
        stripGps,
        expiresInHours: hours,
        maxViews: parsedViews && parsedViews > 0 ? parsedViews : null,
      },
      {
        onSuccess: (res) => {
          setCreated(res);
        },
        onError: (err) => toast.danger(errorText(t, err)),
      },
    );
  };

  const handleClose = () => {
    setCreated(null);
    setPassword("");
    setCopied(false);
    onClose();
  };

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t("sharesApp.dialogTitle")}
        className="grid w-full max-w-md gap-4 rounded-panel border border-line bg-surface p-5 text-fg shadow-overlay"
      >
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Link2 className="size-4 text-fg-muted" aria-hidden />
            <h2 className="font-heading text-lg font-strong text-heading">
              {t("sharesApp.dialogTitle")}
            </h2>
          </div>
          <button
            type="button"
            aria-label={t("a11y.close")}
            onClick={handleClose}
            className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>

        {created ? (
          <div className="grid gap-4">
            <div className="flex items-center gap-2 rounded-tile bg-sunken px-3 py-2 text-sm font-mid text-fg">
              <ShieldCheck className="size-4 shrink-0 text-ok" aria-hidden />
              <span>{t("sharesApp.createdBanner")}</span>
            </div>

            <div className="flex items-center gap-2">
              <input
                type="text"
                readOnly
                value={fullUrl}
                aria-label={t("sharesApp.dialogTitle")}
                onFocus={(e) => e.currentTarget.select()}
                className="h-9 min-w-0 flex-1 rounded-control border border-line-strong bg-sunken px-3 font-mono text-xs text-fg"
              />
              <Button size="sm" variant="primary" onPress={() => void handleCopy()}>
                {copied ? <Check className="size-4" aria-hidden /> : <Copy className="size-4" aria-hidden />}
                {t("sharesApp.copyLink")}
              </Button>
            </div>

            <div className="flex justify-end gap-2 pt-1">
              <a
                href={created.sharePath}
                target="_blank"
                rel="noreferrer"
                className={buttonVariants({ variant: "outline", size: "sm" })}
              >
                <ExternalLink className="size-3.5" aria-hidden />
                {t("sharesApp.openLink")}
              </a>
              <Button size="sm" variant="ghost" onPress={handleClose}>
                {t("common.done")}
              </Button>
            </div>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="grid gap-3.5">
            <div className="grid gap-1.5">
              <label htmlFor="share-title-input" className="text-sm font-mid text-fg">
                {t("sharesApp.shareTitleLabel")}
              </label>
              <FieldInput
                id="share-title-input"
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                maxLength={255}
                required
              />
            </div>

            <div className="grid gap-1.5">
              <label htmlFor="share-password-input" className="text-sm font-mid text-fg">
                {t("sharesApp.passwordLabel")}
              </label>
              <FieldInput
                id="share-password-input"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder={t("sharesApp.passwordPlaceholder")}
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="grid gap-1.5">
                <label htmlFor="share-expiry-select" className="text-sm font-mid text-fg">
                  {t("sharesApp.expiryLabel")}
                </label>
                <Select
                  id="share-expiry-select"
                  value={expiryHours}
                  onChange={(e) => setExpiryHours(e.target.value)}
                  className="w-full"
                >
                  <option value="24">{t("sharesApp.expiry24h")}</option>
                  <option value="168">{t("sharesApp.expiry7d")}</option>
                  <option value="720">{t("sharesApp.expiry30d")}</option>
                  <option value="never">{t("sharesApp.expiryNever")}</option>
                </Select>
              </div>

              <div className="grid gap-1.5">
                <label htmlFor="share-max-views" className="text-sm font-mid text-fg">
                  {t("sharesApp.maxViewsLabel")}
                </label>
                <FieldInput
                  id="share-max-views"
                  type="number"
                  min={1}
                  max={1000000}
                  value={maxViews}
                  onChange={(e) => setMaxViews(e.target.value)}
                  placeholder={t("sharesApp.maxViewsPlaceholder")}
                  className="tabular-nums"
                />
              </div>
            </div>

            <label className="flex cursor-pointer items-center gap-2.5 rounded-field border border-line bg-sunken/60 px-3 py-2 text-sm text-fg">
              <input
                type="checkbox"
                checked={stripGps}
                onChange={(e) => setStripGps(e.target.checked)}
                className="size-4"
              />
              <span>{t("sharesApp.stripGpsLabel")}</span>
            </label>

            <label className="flex cursor-pointer items-center gap-2.5 rounded-field border border-line bg-sunken/60 px-3 py-2 text-sm text-fg">
              <input
                type="checkbox"
                checked={allowDownload}
                onChange={(e) => setAllowDownload(e.target.checked)}
                className="size-4"
              />
              <span>{t("sharesApp.allowDownloadLabel")}</span>
            </label>

            <div className="flex justify-end gap-2 pt-1">
              <Button size="sm" variant="ghost" onPress={handleClose}>
                {t("common.cancel")}
              </Button>
              <Button type="submit" size="sm" variant="primary" isDisabled={createShare.isPending}>
                {t("sharesApp.createSubmit")}
              </Button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
