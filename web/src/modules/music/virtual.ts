import type { Rect, Virtualizer } from "@tanstack/react-virtual";

const FALLBACK_RECT: Rect = { width: 1024, height: 800 };

/**
 * ResizeObserver-backed `observeElementRect` for TanStack Virtual that falls back to a sensible
 * viewport rect when running in a layout-less environment (e.g. jsdom unit tests where rects are 0×0).
 */
export function observeVirtualRect<T extends Element>(
  instance: Virtualizer<T, Element>,
  cb: (rect: Rect) => void,
) {
  const element = instance.scrollElement;
  if (!element) return;

  const emit = () => {
    const box = element.getBoundingClientRect();
    cb(box.height > 0 || box.width > 0 ? { width: box.width, height: box.height } : FALLBACK_RECT);
  };

  emit();

  if (typeof ResizeObserver === "undefined") return;
  const observer = new ResizeObserver(emit);
  observer.observe(element);
  return () => observer.disconnect();
}
