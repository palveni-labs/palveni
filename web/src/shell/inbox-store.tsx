/**
 * Items modules put in the inbox (approvals, degraded states, notifications), collected once by
 * the shell and readable anywhere: the inbox popover, the activity window, the overview.
 */
import { useEffect, useSyncExternalStore } from "react";
import { useAccess } from "@/modules/access";
import { isEnabled, MODULES } from "@/modules/registry";
import type { InboxItem, InboxSource } from "@/modules/types";

let bySource: Record<string, InboxItem[]> = {};
let snapshot: InboxItem[] = [];
const listeners = new Set<() => void>();

function same(a: InboxItem[] | undefined, b: InboxItem[]): boolean {
  if (!a || a.length !== b.length) return false;
  return a.every((item, index) => {
    const other = b[index];
    return other !== undefined && item.id === other.id && item.title === other.title && item.detail === other.detail && item.kind === other.kind;
  });
}

function publish(id: string, items: InboxItem[]) {
  if (same(bySource[id], items)) return;
  bySource = { ...bySource, [id]: items };
  snapshot = Object.values(bySource)
    .flat()
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  for (const listener of listeners) listener();
}

function clear(id: string) {
  if (!(id in bySource)) return;
  const { [id]: _removed, ...rest } = bySource;
  bySource = rest;
  snapshot = Object.values(bySource).flat();
  for (const listener of listeners) listener();
}

/** Mounts one module's inbox hook and publishes its items. */
function Collector({ id, source }: { id: string; source: InboxSource }) {
  const useItems = source;
  const items = useItems();
  useEffect(() => publish(id, items), [id, items]);
  useEffect(() => () => clear(id), [id]);
  return null;
}

/** Rendered once by the shell. */
export function InboxCollectors() {
  const access = useAccess();
  return (
    <>
      {MODULES.filter((module) => module.inbox && isEnabled(module, access)).map((module) => (
        <Collector key={module.id} id={module.id} source={module.inbox as InboxSource} />
      ))}
    </>
  );
}

export function useInboxItems(): InboxItem[] {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => snapshot,
    () => snapshot,
  );
}

/** Things that need the user: approvals and degraded states. */
export function useAttentionCount(): number {
  return useInboxItems().filter((item) => item.kind !== "notification").length;
}

/** For tests. */
export function resetInboxStore() {
  bySource = {};
  snapshot = [];
}
