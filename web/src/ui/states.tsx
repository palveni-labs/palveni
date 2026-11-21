/**
 * Empty, loading, error and unavailable states. Each says what is going on in words and, where
 * there is one, offers the way forward.
 */
import { Button, Skeleton } from "@heroui/react";
import { AlertCircle, RotateCw, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { describeError } from "@/lib/errors";

export function EmptyState({
  icon: Icon,
  title,
  detail,
  action,
  className,
  compact = false,
}: {
  icon: LucideIcon;
  title: ReactNode;
  detail?: ReactNode;
  action?: ReactNode;
  className?: string;
  compact?: boolean;
}) {
  return (
    <div className={cn("flex flex-col items-center justify-center gap-3 text-center", compact ? "py-8" : "min-h-[40vh] py-12", className)}>
      <span className="grid size-11 place-items-center rounded-tile bg-sunken text-fg-muted">
        <Icon className="size-5" strokeWidth={1.7} aria-hidden />
      </span>
      <div className="grid max-w-sm gap-1">
        <p className="font-strong text-fg">{title}</p>
        {detail ? <p className="text-sm text-fg-muted">{detail}</p> : null}
      </div>
      {action}
    </div>
  );
}

export function ErrorState({ error, onRetry, compact = false, className }: { error: unknown; onRetry?: () => void; compact?: boolean; className?: string }) {
  const { t } = useTranslation();
  const { title, detail } = describeError(t, error);
  return (
    <div role="alert" className={cn("flex flex-col items-center justify-center gap-3 text-center", compact ? "py-8" : "min-h-[40vh] py-12", className)}>
      <span className="grid size-11 place-items-center rounded-tile bg-failed-soft text-failed">
        <AlertCircle className="size-5" strokeWidth={1.8} aria-hidden />
      </span>
      <div className="grid max-w-sm gap-1">
        <p className="font-strong text-fg">{title}</p>
        {detail ? <p className="text-sm text-fg-muted">{detail}</p> : null}
      </div>
      {onRetry ? (
        <Button variant="outline" size="sm" onPress={onRetry}>
          <RotateCw aria-hidden />
          {t("common.retry")}
        </Button>
      ) : null}
    </div>
  );
}

/** Placeholder rows while a list loads. */
export function LoadingRows({ rows = 3, className, label }: { rows?: number; className?: string; label?: string }) {
  const { t } = useTranslation();
  return (
    <div role="status" aria-label={label ?? t("common.loading")} className={cn("grid gap-3 py-2", className)}>
      {Array.from({ length: rows }, (_, index) => (
        <div key={index} className="flex items-center gap-3">
          <Skeleton className="size-9 shrink-0 rounded-tile" />
          <div className="grid flex-1 gap-1.5">
            <Skeleton className="h-3.5 w-2/5 rounded-full" />
            <Skeleton className="h-3 w-3/5 rounded-full" />
          </div>
        </div>
      ))}
    </div>
  );
}
