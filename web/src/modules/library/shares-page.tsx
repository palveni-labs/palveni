import { Button, buttonVariants, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { Copy, ExternalLink, Link2, Lock, ShieldCheck, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  useRevokeShare,
  useShares,
  type ShareLinkSummary,
} from "@/api/queries/shares-collections";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

function ShareStatusBadge({ share }: { share: ShareLinkSummary }) {
  const { t } = useTranslation();
  if (share.revokedAt) {
    return (
      <span className="rounded-chip bg-sunken px-2 py-0.5 text-xs font-mid text-fg-muted">
        {t("sharesApp.statusRevoked")}
      </span>
    );
  }
  if (share.expired) {
    return (
      <span className="rounded-chip bg-sunken px-2 py-0.5 text-xs font-mid text-fg-muted">
        {t("sharesApp.statusExpired")}
      </span>
    );
  }
  return (
    <span className="rounded-chip bg-ok/15 px-2 py-0.5 text-xs font-strong text-ok">
      {t("sharesApp.statusActive")}
    </span>
  );
}

export function SharesPage() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "zh-CN";
  const query = useShares();
  const revoke = useRevokeShare();

  const shares = query.data?.shares ?? [];

  const copyShareUrl = async (share: ShareLinkSummary) => {
    const url = `${window.location.origin}${share.sharePath}`;
    try {
      await navigator.clipboard.writeText(url);
      toast.success(t("sharesApp.copiedToast"));
    } catch {
      // Clipboard may be restricted in non-HTTPS contexts
    }
  };

  const handleRevoke = (id: string) => {
    revoke.mutate(id, {
      onSuccess: () => toast.success(t("sharesApp.revokedToast")),
      onError: (err) => toast.danger(errorText(t, err)),
    });
  };

  return (
    <PageFrame>
      <PageHeader title={t("sharesApp.title")} summary={t("sharesApp.summary")} />

      {query.isPending ? (
        <LoadingRows rows={4} />
      ) : query.isError ? (
        <ErrorState error={query.error} onRetry={() => void query.refetch()} />
      ) : shares.length === 0 ? (
        <EmptyState
          icon={Link2}
          title={t("sharesApp.emptyTitle")}
          detail={t("sharesApp.emptyDetail")}
        />
      ) : (
        <Section title={t("sharesApp.title")} count={shares.length}>
          <SectionRows>
            {shares.map((share) => {
              const inactive = Boolean(share.revokedAt) || share.expired;
              return (
                <li
                  key={share.id}
                  className={cn(
                    "flex flex-col gap-3 px-4 py-3.5 sm:flex-row sm:items-center sm:justify-between",
                    inactive && "opacity-70",
                  )}
                >
                  <div className="grid min-w-0 flex-1 gap-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="truncate font-strong text-heading">{share.title}</span>
                      <ShareStatusBadge share={share} />
                      <span className="rounded-chip bg-sunken px-2 py-0.5 text-xs font-mid text-fg-muted">
                        {t(`sharesApp.targetKinds.${share.targetKind}`)}
                      </span>
                      {share.hasPassword ? (
                        <span className="inline-flex items-center gap-1 rounded-chip bg-sunken px-2 py-0.5 text-xs font-mid text-fg">
                          <Lock className="size-3" aria-hidden />
                          {t("sharesApp.passwordBadge")}
                        </span>
                      ) : null}
                      {share.stripGps ? (
                        <span className="inline-flex items-center gap-1 rounded-chip bg-sunken px-2 py-0.5 text-xs font-mid text-fg-muted">
                          <ShieldCheck className="size-3 text-ok" aria-hidden />
                          {t("sharesApp.gpsStrippedBadge")}
                        </span>
                      ) : null}
                    </div>

                    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-fg-muted tabular-nums">
                      <span className="font-mono">{share.sharePath}</span>
                      <span aria-hidden>·</span>
                      <span data-num>
                        {share.maxViews != null
                          ? t("sharesApp.viewsWithMax", {
                              count: share.viewCount,
                              max: share.maxViews,
                            })
                          : t("sharesApp.viewsCount", { count: share.viewCount })}
                      </span>
                      {share.expiresAt ? (
                        <>
                          <span aria-hidden>·</span>
                          <span>
                            {t("sharesApp.expiresAt", {
                              time: new Date(share.expiresAt).toLocaleString(locale),
                            })}
                          </span>
                        </>
                      ) : null}
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center gap-1.5">
                    <Button
                      size="sm"
                      variant="outline"
                      onPress={() => void copyShareUrl(share)}
                    >
                      <Copy className="size-3.5" aria-hidden />
                      {t("sharesApp.copyLink")}
                    </Button>
                    {!inactive ? (
                      <Link
                        to="/s/$token"
                        params={{ token: share.token }}
                        className={buttonVariants({ variant: "ghost", size: "sm" })}
                      >
                        <ExternalLink className="size-3.5" aria-hidden />
                        {t("sharesApp.openLink")}
                      </Link>
                    ) : null}
                    {!share.revokedAt ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        isDisabled={revoke.isPending}
                        onPress={() => handleRevoke(share.id)}
                      >
                        <Trash2 className="size-3.5" aria-hidden />
                        {t("sharesApp.revoke")}
                      </Button>
                    ) : null}
                  </div>
                </li>
              );
            })}
          </SectionRows>
        </Section>
      )}
    </PageFrame>
  );
}
