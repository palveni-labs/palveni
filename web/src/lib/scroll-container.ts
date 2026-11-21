import { createContext, useContext, type RefObject } from "react";

/** The workspace scroll container (the window does not scroll inside the app shell). */
export const ScrollContainerContext = createContext<RefObject<HTMLDivElement | null> | null>(null);

export function useScrollContainer(): RefObject<HTMLDivElement | null> {
  const ref = useContext(ScrollContainerContext);
  if (!ref) throw new Error("useScrollContainer used outside the app shell");
  return ref;
}
