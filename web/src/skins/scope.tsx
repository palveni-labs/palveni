/**
 * Slot access for host components, and a scope that renders a subtree in another skin.
 *
 * Host components ask `useSlot("progress")` and draw the matching variant; nothing in the app
 * looks at a skin's identity. Inside <SkinScope> (the skin picker's live previews) the same
 * hooks answer for the previewed skin, and its tokens are set on the scope element.
 */
import { createContext, use, useEffect, type CSSProperties, type ReactNode } from "react";
import { cn } from "@/lib/cn";
import { skinVariables } from "./css";
import { retainFonts } from "./fonts";
import type { SkinEntry } from "./registry";
import type { Mode, SkinManifest, SlotName, Slots } from "./schema";
import { useSkin } from "./store";

interface ScopeValue {
  skin: SkinManifest;
  mode: Mode;
}

const ScopeContext = createContext<ScopeValue | null>(null);

/** The skin and mode that apply where the calling component renders. */
export function useActiveSkin(): ScopeValue {
  const scope = use(ScopeContext);
  const { entry, resolved } = useSkin();
  return scope ?? { skin: entry.manifest, mode: resolved };
}

export function useSlots(): Slots {
  return useActiveSkin().skin.slots;
}

export function useSlot<K extends SlotName>(name: K): Slots[K] {
  return useSlots()[name];
}

export function useMode(): Mode {
  return useActiveSkin().mode;
}

/** Renders children in `entry`'s skin and `mode`, independent of the active skin. */
export function SkinScope({
  entry,
  mode,
  className,
  children,
}: {
  entry: SkinEntry;
  mode: Mode;
  className?: string;
  children: ReactNode;
}) {
  const skin = entry.manifest;
  useEffect(() => {
    let release: (() => void) | null = null;
    let cancelled = false;
    void entry.loadFonts().then((assets) => {
      if (!cancelled) release = retainFonts(`skin:${skin.id}`, assets);
    });
    return () => {
      cancelled = true;
      release?.();
    };
  }, [entry, skin.id]);
  const { "color-scheme": _scheme, ...variables } = skinVariables(skin, mode);
  const style = { ...variables, colorScheme: mode } as CSSProperties;
  return (
    <ScopeContext value={{ skin, mode }}>
      <div data-skin-scope={skin.id} data-theme={mode} className={cn("skin-scope", mode, className)} style={style}>
        {children}
      </div>
    </ScopeContext>
  );
}
