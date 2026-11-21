import { buttonVariants } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { Lock, PlugZap } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useSystemInfo } from "@/api/queries/system";
import { useAccess } from "@/modules/access";
import { isVisible } from "@/modules/registry";
import type { Availability, MessageKey } from "@/modules/types";
import { useActiveModule } from "./use-shell-modules";

/** Shown instead of a page whose server feature is missing. No fake data. */
export function FeatureUnavailable({ titleKey, compact = false }: { titleKey: MessageKey; compact?: boolean }) {
  const { t } = useTranslation();
  const { data: info } = useSystemInfo();
  return (
    <div className={compact ? "grid place-items-center gap-3 px-6 py-10 text-center" : "grid min-h-[60vh] place-items-center px-6"}>
      <div className="flex max-w-sm flex-col items-center gap-4 text-center">
        <span className="grid size-12 place-items-center rounded-tile border border-dashed border-line-strong text-fg-muted">
          <PlugZap className="size-5" strokeWidth={1.7} aria-hidden />
        </span>
        <div className="grid gap-1.5">
          <h2 className="font-heading text-lg font-title text-heading">{t("feature.unavailableTitle")}</h2>
          <p className="text-sm text-balance text-fg-muted">
            {t("feature.unavailableDetail", { feature: t(titleKey), version: info?.version ?? "" })}
          </p>
          <p className="text-sm text-balance text-fg-muted">{t("feature.unavailableHint")}</p>
        </div>
        {!compact ? (
          <Link to="/" className={buttonVariants({ variant: "outline" })}>
            {t("common.backHome")}
          </Link>
        ) : null}
      </div>
    </div>
  );
}

function NotPermitted() {
  const { t } = useTranslation();
  return (
    <div className="grid min-h-[60vh] place-items-center px-6">
      <div className="flex flex-col items-center gap-3 text-center">
        <Lock className="size-6 text-fg-muted" aria-hidden />
        <p className="text-sm text-fg-muted">{t("feature.adminOnly")}</p>
      </div>
    </div>
  );
}

function missing(item: Availability | undefined, features: readonly string[]): boolean {
  return Boolean(item?.feature && !features.includes(item.feature));
}

/** Wraps the page outlet: module and sub-page availability come from the registry. */
export function FeatureGate({ children }: { children: ReactNode }) {
  const access = useAccess();
  const { module, entry } = useActiveModule();
  if (!module) return <>{children}</>;
  if (!isVisible(module, access) || (entry && !isVisible(entry, access))) return <NotPermitted />;
  if (missing(module, access.features)) return <FeatureUnavailable titleKey={module.titleKey} />;
  if (entry && missing(entry, access.features)) return <FeatureUnavailable titleKey={entry.titleKey} />;
  return <>{children}</>;
}
