import { useSyncExternalStore } from "react";

function useMediaQuery(query: string, fallback = false): boolean {
  return useSyncExternalStore(
    (onChange) => {
      if (typeof window.matchMedia !== "function") return () => {};
      const list = window.matchMedia(query);
      list.addEventListener("change", onChange);
      return () => list.removeEventListener("change", onChange);
    },
    () => (typeof window.matchMedia === "function" ? window.matchMedia(query).matches : fallback),
    () => fallback,
  );
}

/** Wide enough to dock side panels next to the page (≥ 1280px). */
export function useIsWide(): boolean {
  return useMediaQuery("(min-width: 1280px)");
}

/** Tablet width and up (≥ 768px, Tailwind's `md`): where the shell's toolbar shows page titles. */
export function useIsTabletUp(): boolean {
  return useMediaQuery("(min-width: 768px)");
}
