import { Button, buttonVariants, Spinner } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { CircleCheck, CloudOff, CircleDot } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { RelativeTime } from "@/components/relative-time";
import type { InboxAction, InboxItem } from "@/modules/types";
import { useInboxItems } from "@/shell/inbox-store";
import { useSlot } from "@/skins/scope";
import { AttentionMark, Lamp, StatusLabel } from "@/ui/marks";
import { useRowInset, useSectionVariant } from "@/ui/section";
import { EmptyState } from "@/ui/states";

function ActionButton({ action }: { action: InboxAction }) {
  const Icon = action.icon;
  const variant = action.variant === "confirm" ? "primary" : action.variant;
  const className = cn(action.variant === "confirm" && "button--confirm");
  if (action.to) {
    return (
      <Link to={action.to} search={action.search} className={cn(buttonVariants({ variant, size: "sm" }), className)}>
        {Icon ? <Icon aria-hidden /> : null}
        {action.label}
      </Link>
    );
  }
  return (
    <Button size="sm" variant={variant} className={className} onPress={action.run} isPending={action.busy}>
      {action.busy ? <Spinner size="sm" color="current" /> : Icon ? <Icon aria-hidden /> : null}
      {action.label}
    </Button>
  );
}

function Lead({ item }: { item: InboxItem }) {
  const attention = useSlot("attention");
  if (item.kind === "approval") return <AttentionMark previews={item.previews} title={item.title} icon={CircleDot} />;
  // Degraded states: the same lead column, with words.
  if (attention === "thumbs") {
    return (
      <span className="flex w-[7rem] shrink-0 items-center gap-1.5 font-title text-degraded">
        <CloudOff className="size-5" aria-hidden />
        {item.state}
      </span>
    );
  }
  if (attention === "lamp") return <Lamp tone="degraded" className="mt-[0.45em] size-2.5 rounded-[2px]" />;
  return <CloudOff className="mt-[0.15em] size-[1.15rem] shrink-0 text-degraded" aria-hidden />;
}

function Row({ item }: { item: InboxItem }) {
  const { t } = useTranslation();
  const attention = useSlot("attention");
  const inset = useRowInset();
  const variant = useSectionVariant();
  const tint = attention === "tint";
  const alert = item.kind === "alert";
  return (
    <li
      className={cn(
        "flex flex-wrap items-center gap-x-4 gap-y-3 py-3.5 sm:flex-nowrap",
        tint ? "rounded-tile border px-4" : inset,
        tint && (alert ? "border-degraded-line bg-degraded-soft" : "border-attention-line bg-attention-soft"),
        !tint && alert && attention === "lamp" && "bg-degraded-soft px-4",
        variant === "plain" && !tint && "rounded-tile",
      )}
    >
      <div className="flex min-w-0 flex-1 items-start gap-3.5">
        <Lead item={item} />
        <div className="grid min-w-0 flex-1 gap-0.5">
          <h3 className="text-md font-mid text-fg">{item.title}</h3>
          <p className="text-sm text-fg-muted">
            {alert && attention !== "thumbs" ? (
              <>
                <StatusLabel tone="degraded" className="inline-flex align-baseline" lamp>
                  {item.state}
                </StatusLabel>
                <span aria-hidden> · </span>
              </>
            ) : null}
            {!alert ? (
              <>
                <span className="font-mid text-attention">{t("attention.waiting")}</span>
                <span aria-hidden> · </span>
              </>
            ) : null}
            {alert && item.detail ? (
              <>
                {item.detail}
                <span aria-hidden> · </span>
              </>
            ) : null}
            <RelativeTime value={item.createdAt} />
            {item.scope ? (
              <>
                <span aria-hidden> · </span>
                {item.scope}
              </>
            ) : null}
          </p>
        </div>
      </div>
      {item.actions?.length ? (
        <div className="flex shrink-0 flex-wrap items-center gap-1.5 max-sm:w-full max-sm:justify-end">
          {item.actions.map((action) => (
            <ActionButton key={action.id} action={action} />
          ))}
        </div>
      ) : null}
    </li>
  );
}

/** 需要你处理: degraded states first (they are blocking work), then approvals, newest first. */
export default function AttentionWidget() {
  const { t } = useTranslation();
  const attention = useSlot("attention");
  const variant = useSectionVariant();
  const items = useInboxItems().filter((item) => item.kind !== "notification");
  const ordered = [...items.filter((item) => item.kind === "alert"), ...items.filter((item) => item.kind === "approval")];
  if (ordered.length === 0) {
    return <EmptyState compact icon={CircleCheck} title={t("attention.emptyTitle")} detail={t("attention.emptyDetail")} />;
  }
  return (
    <ul className={cn(attention === "tint" ? "grid gap-2" : "divide-y divide-line", variant === "panel" && attention !== "tint" && "border-t border-line")}>
      {ordered.map((item) => (
        <Row key={item.id} item={item} />
      ))}
    </ul>
  );
}
