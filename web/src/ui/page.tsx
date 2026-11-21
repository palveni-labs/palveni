/**
 * Page header and summary facts.
 *
 * Where the page title lives is the `pageTitle` slot: `content` sets it as the first line of the
 * page, `toolbar` leaves it and the page's actions to the shell's toolbar (the page keeps an
 * accessible h1 and its summary line). Phones have no toolbar title, so there the page header is
 * always in the content. Summary
 * facts — "重复 23 组 · 可释放 186 MB" — follow the `summary` slot: separate `cells` with the label
 * above the value, or one `inline` line.
 */
import { useSyncExternalStore, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/cn";
import { useIsTabletUp } from "@/lib/use-media";
import { useSlot } from "@/skins/scope";

type ToolbarSlot = "title" | "actions";
const toolbarTargets: Record<ToolbarSlot, HTMLElement | null> = { title: null, actions: null };
const toolbarListeners = new Set<() => void>();

function registerTarget(slot: ToolbarSlot, element: HTMLElement | null) {
  if (toolbarTargets[slot] === element) return;
  toolbarTargets[slot] = element;
  for (const listener of toolbarListeners) listener();
}

export const registerToolbarActions = (element: HTMLElement | null) => registerTarget("actions", element);
export const registerToolbarTitle = (element: HTMLElement | null) => registerTarget("title", element);

function subscribeToolbar(listener: () => void) {
  toolbarListeners.add(listener);
  return () => { toolbarListeners.delete(listener); };
}

function ToolbarPortal({ slot, children }: { slot: ToolbarSlot; children: ReactNode }) {
  const target = useSyncExternalStore(subscribeToolbar, () => toolbarTargets[slot], () => null);
  return target ? createPortal(children, target) : null;
}

export function PageHeader({ title, summary, actions, className }: { title: string; summary?: ReactNode; actions?: ReactNode; className?: string }) {
  const placement = useSlot("pageTitle");
  const toolbar = useIsTabletUp();
  if (placement === "toolbar" && toolbar) {
    return (
      <>
        <h1 className="sr-only">{title}</h1>
        <ToolbarPortal slot="title">{title}</ToolbarPortal>
        {summary ? <p className={cn("min-w-0 text-base text-fg-muted", className)}>{summary}</p> : null}
        {actions ? <ToolbarPortal slot="actions">{actions}</ToolbarPortal> : null}
      </>
    );
  }
  return (
    <div className={cn("flex flex-wrap items-baseline gap-x-4 gap-y-1", className)}>
      <h1 className="font-heading text-headline font-title text-heading">{title}</h1>
      {summary ? <p className="min-w-0 text-base text-fg-muted">{summary}</p> : null}
      {actions ? <div className="ms-auto flex flex-wrap items-center gap-2 self-center">{actions}</div> : null}
    </div>
  );
}

export interface Fact {
  label: string;
  value: ReactNode;
  unit?: ReactNode;
  /** Small words after the value ("只是相似，不是完全相同"). */
  note?: ReactNode;
  tone?: "default" | "attention" | "degraded";
}

export function SummaryFacts({ facts, className, onField = false }: { facts: Fact[]; className?: string; onField?: boolean }) {
  const summary = useSlot("summary");
  if (summary === "cells") {
    return (
      <dl className={cn("grid grid-cols-[repeat(auto-fit,minmax(9rem,1fr))] border-y border-line", className)}>
        {facts.map((fact) => (
          <div key={fact.label} className="grid content-start gap-1 border-line px-(--pv-pad-panel) py-3.5 [&:not(:first-child)]:border-s">
            <dt className="text-sm text-fg-muted">{fact.label}</dt>
            <dd className="flex flex-wrap items-baseline gap-x-1.5">
              <span className="text-stat font-strong tabular-nums" data-num>
                {fact.value}
              </span>
              {fact.unit ? <span className="text-base font-mid">{fact.unit}</span> : null}
              {fact.note ? <span className="basis-full text-sm text-fg-muted">{fact.note}</span> : null}
            </dd>
          </div>
        ))}
      </dl>
    );
  }
  return (
    <dl className={cn("flex flex-wrap items-baseline gap-x-5 gap-y-1", className)}>
      {facts.map((fact, index) => (
        <div key={fact.label} className={cn("flex items-baseline gap-1.5", index > 0 && "border-s ps-5", onField ? "border-current/25" : "border-line")}>
          <dt className={onField ? "text-(--work-field-sub)" : "text-fg-muted"}>{fact.label}</dt>
          <dd className="flex items-baseline gap-1">
            <span className="text-md font-strong tabular-nums" data-num>
              {fact.value}
            </span>
            {fact.unit ? <span className="font-mid">{fact.unit}</span> : null}
            {fact.note ? <span className={cn("ms-1 text-sm", onField ? "text-(--work-field-sub)" : "text-fg-muted")}>{fact.note}</span> : null}
          </dd>
        </div>
      ))}
    </dl>
  );
}
