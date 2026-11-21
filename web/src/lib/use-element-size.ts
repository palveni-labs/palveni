import { useLayoutEffect, useState, type RefObject } from "react";

/** Content-box width of an element, updated through ResizeObserver. */
export function useElementWidth(ref: RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    setWidth(element.clientWidth);
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) setWidth(Math.round(entry.contentRect.width));
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return width;
}

/**
 * Distance from the top of the scroll container's content to `ref`, i.e. the virtualizer's
 * `scrollMargin` when a list starts below other page content.
 */
export function useOffsetWithin(
  ref: RefObject<HTMLElement | null>,
  containerRef: RefObject<HTMLElement | null>,
  deps: readonly unknown[] = [],
): number {
  const [offset, setOffset] = useState(0);
  useLayoutEffect(() => {
    const element = ref.current;
    const container = containerRef.current;
    if (!element || !container) return;
    const measure = () => {
      const next =
        element.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop;
      setOffset((previous) => (Math.abs(previous - next) < 0.5 ? previous : next));
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    const page = container.firstElementChild;
    if (page) observer.observe(page);
    observer.observe(container);
    return () => observer.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- callers pass layout-affecting values
  }, [ref, containerRef, ...deps]);
  return offset;
}
