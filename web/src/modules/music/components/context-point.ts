import { useState, type MouseEvent } from "react";

export interface ContextPoint {
  x: number;
  y: number;
}

export function useContextPoint() {
  const [point, setPoint] = useState<ContextPoint | null>(null);
  const onContextMenu = (event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    setPoint({ x: event.clientX, y: event.clientY });
  };
  return { point, openAt: setPoint, close: () => setPoint(null), onContextMenu };
}
