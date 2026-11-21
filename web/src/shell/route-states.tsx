import { buttonVariants, Button, Skeleton } from "@heroui/react";
import { useQueryErrorResetBoundary } from "@tanstack/react-query";
import { Link, useRouter, type ErrorComponentProps } from "@tanstack/react-router";
import { CircleAlert, Compass, RotateCw } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { isApiError } from "@/api/errors";
import { describeError } from "@/lib/errors";
import { PageFrame } from "@/ui/frame";
import { EmptyState } from "@/ui/states";

/** Placeholder shown while a page's loader runs; mirrors a page header and two sections. */
export function PagePending() {
  const { t } = useTranslation();
  return (
    <PageFrame aria-busy aria-label={t("common.loading")}>
      <Skeleton className="h-7 w-40 rounded-tile" />
      <Skeleton className="h-40 rounded-panel" />
      <Skeleton className="h-72 rounded-panel" />
    </PageFrame>
  );
}

export function RouteError({ error, reset }: ErrorComponentProps) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryErrorResetBoundary = useQueryErrorResetBoundary();
  useEffect(() => queryErrorResetBoundary.reset(), [queryErrorResetBoundary]);

  if (isApiError(error, "not_found")) return <RouteNotFound />;
  const { title, detail } = describeError(t, error);
  return (
    <EmptyState
      icon={CircleAlert}
      title={title}
      detail={detail ?? t("errors.tryAgain")}
      action={
        <Button
          variant="outline"
          onPress={() => {
            reset();
            void router.invalidate();
          }}
        >
          <RotateCw aria-hidden />
          {t("common.retry")}
        </Button>
      }
    />
  );
}

export function RouteNotFound() {
  const { t } = useTranslation();
  return (
    <EmptyState
      icon={Compass}
      title={t("errors.pageNotFound")}
      action={
        <Link to="/" className={buttonVariants({ variant: "outline" })}>
          {t("common.backHome")}
        </Link>
      }
    />
  );
}
