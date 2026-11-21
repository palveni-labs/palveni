import { useCallback, useState } from "react";
import { readLocal, writeLocal } from "@/lib/storage";

const KEY = "palveni.overview.layout";

export interface OverviewLayout {
  /** Widget ids in display order; widgets not listed follow in registry order. */
  order: string[];
  hidden: string[];
}

const EMPTY: OverviewLayout = { order: [], hidden: [] };

function read(): OverviewLayout {
  try {
    const parsed = JSON.parse(readLocal(KEY) ?? "null") as Partial<OverviewLayout> | null;
    if (!parsed) return EMPTY;
    return {
      order: Array.isArray(parsed.order) ? parsed.order.filter((id): id is string => typeof id === "string") : [],
      hidden: Array.isArray(parsed.hidden) ? parsed.hidden.filter((id): id is string => typeof id === "string") : [],
    };
  } catch {
    return EMPTY;
  }
}

/** Applies a stored layout to the widgets that exist now. */
export function arrange<T extends { id: string }>(widgets: T[], layout: OverviewLayout): { shown: T[]; hidden: T[] } {
  const rank = (id: string) => {
    const index = layout.order.indexOf(id);
    return index === -1 ? layout.order.length + widgets.findIndex((widget) => widget.id === id) : index;
  };
  const sorted = [...widgets].sort((a, b) => rank(a.id) - rank(b.id));
  return {
    shown: sorted.filter((widget) => !layout.hidden.includes(widget.id)),
    hidden: sorted.filter((widget) => layout.hidden.includes(widget.id)),
  };
}

/** Per-browser dashboard layout (order and hidden widgets). */
export function useOverviewLayout() {
  const [layout, setLayout] = useState<OverviewLayout>(read);
  const save = useCallback((next: OverviewLayout) => {
    setLayout(next);
    writeLocal(KEY, JSON.stringify(next));
  }, []);

  const move = (ids: string[], id: string, delta: -1 | 1) => {
    const index = ids.indexOf(id);
    const target = index + delta;
    if (index === -1 || target < 0 || target >= ids.length) return;
    const next = [...ids];
    [next[index], next[target]] = [next[target] as string, next[index] as string];
    save({ ...layout, order: next });
  };
  const hide = (id: string) => save({ ...layout, hidden: [...new Set([...layout.hidden, id])] });
  const show = (id: string) => save({ ...layout, hidden: layout.hidden.filter((entry) => entry !== id) });
  const reset = () => save(EMPTY);
  return { layout, move, hide, show, reset };
}
