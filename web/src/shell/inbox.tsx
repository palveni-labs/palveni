import { Button, Popover } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { AlertTriangle, CheckCircle2, CircleDot, Inbox as InboxIcon, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { RelativeTime } from "@/components/relative-time";
import type { InboxItem } from "@/modules/types";
import { CountMark } from "@/ui/count-mark";
import { useAttentionCount, useInboxItems } from "./inbox-store";
import { shell, useShellState } from "./shell-store";

function ItemIcon({ item }: { item: InboxItem }) {
  if (item.kind === "approval") return <CircleDot className="size-4 text-attention-mark" aria-hidden />;
  if (item.kind === "alert") return <AlertTriangle className="size-4 text-degraded" aria-hidden />;
  if (item.tone === "danger") return <XCircle className="size-4 text-failed" aria-hidden />;
  return <CheckCircle2 className="size-4 text-ok" aria-hidden />;
}

function Row({ item }: { item: InboxItem }) {
  return (
    <li>
      <Link
        to={item.to}
        search={item.search}
        onClick={() => shell.setInbox(false)}
        className="flex items-start gap-3 rounded-tile px-3 py-2.5 outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
      >
        <span className="mt-0.5">
          <ItemIcon item={item} />
        </span>
        <span className="grid min-w-0 flex-1 gap-0.5">
          <span className="truncate text-sm font-mid text-fg">{item.title}</span>
          {item.detail ? <span className="line-clamp-2 text-xs text-fg-muted">{item.detail}</span> : null}
        </span>
        <RelativeTime value={item.createdAt} className="shrink-0 text-xs text-fg-muted" />
      </Link>
    </li>
  );
}

function Group({ title, items }: { title: string; items: InboxItem[] }) {
  if (items.length === 0) return null;
  return (
    <section>
      <h3 className="px-3 pt-1 pb-1 text-xs font-mid text-fg-muted">{title}</h3>
      <ul>
        {items.map((item) => (
          <Row key={item.id} item={item} />
        ))}
      </ul>
    </section>
  );
}

/** Approvals, degraded states and recent notifications, contributed by modules. */
export function Inbox({ tone = "shell" }: { tone?: "shell" | "plain" }) {
  const { t } = useTranslation();
  const { inboxOpen } = useShellState();
  const items = useInboxItems();
  const count = useAttentionCount();
  const approvals = items.filter((item) => item.kind === "approval");
  const alerts = items.filter((item) => item.kind === "alert");
  const notifications = items.filter((item) => item.kind === "notification").slice(0, 8);

  return (
    <Popover isOpen={inboxOpen} onOpenChange={(open) => shell.setInbox(open)}>
      <Button
        variant="ghost"
        isIconOnly
        aria-label={count > 0 ? t("inbox.titleWithCount", { count }) : t("inbox.title")}
        className={cn("relative", tone === "shell" && "text-shell-fg-muted hover:text-shell-fg data-[hovered=true]:bg-shell-field")}
      >
        <InboxIcon aria-hidden />
        {count > 0 ? <CountMark count={count} className="absolute -top-0.5 -right-0.5" /> : null}
      </Button>
      <Popover.Content placement="bottom end" className="w-[min(24rem,calc(100vw-2rem))] p-0">
        <Popover.Dialog aria-label={t("inbox.title")} className="p-2 outline-none">
          <div className="flex items-center justify-between px-3 pt-1.5 pb-2">
            <Popover.Heading className="text-base font-strong text-fg">{t("inbox.title")}</Popover.Heading>
          </div>
          {items.length === 0 ? (
            <p className="px-3 py-8 text-center text-sm text-fg-muted">{t("inbox.empty")}</p>
          ) : (
            <div className="grid max-h-[60vh] gap-2 overflow-y-auto">
              <Group title={t("inbox.alerts", { count: alerts.length })} items={alerts} />
              <Group title={t("inbox.approvals", { count: approvals.length })} items={approvals} />
              <Group title={t("inbox.notifications")} items={notifications} />
            </div>
          )}
        </Popover.Dialog>
      </Popover.Content>
    </Popover>
  );
}
