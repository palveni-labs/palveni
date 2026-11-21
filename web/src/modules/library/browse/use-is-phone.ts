import { useSyncExternalStore } from "react";

const PHONE = "(max-width: 767px)";

/** Below the md breakpoint, where sheets come up from the bottom. */
export function useIsPhone(): boolean {
  return useSyncExternalStore(
    (onChange) => {
      if (typeof window.matchMedia !== "function") return () => {};
      const list = window.matchMedia(PHONE);
      list.addEventListener("change", onChange);
      return () => list.removeEventListener("change", onChange);
    },
    () => (typeof window.matchMedia === "function" ? window.matchMedia(PHONE).matches : false),
    () => false,
  );
}
