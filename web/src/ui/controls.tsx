/**
 * Unified native-semantic UI primitives bound directly to the active skin's `--pv-*` tokens.
 *
 * Keeps native HTML semantics (`<select>`, `<input>`, `<textarea>`, `<article>`) so screen
 * readers, keyboard navigation, IME, and form submission work natively while guaranteeing
 * identical control heights, border radii, padding, and focus states across every module.
 */
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

export type ControlSize = "sm" | "md";

export interface SelectProps extends Omit<ComponentProps<"select">, "size"> {
  /** `"md"` matches `--pv-control-h`; `"sm"` matches `--pv-control-h-sm`. */
  size?: ControlSize;
}

export function Select({ size = "md", className, ...props }: SelectProps) {
  return (
    <select
      data-control-size={size}
      className={cn("pv-select", size === "sm" && "pv-select--sm", className)}
      {...props}
    />
  );
}

export interface FieldInputProps extends Omit<ComponentProps<"input">, "size"> {
  size?: ControlSize;
}

export function FieldInput({ size = "md", className, ...props }: FieldInputProps) {
  return (
    <input
      data-control-size={size}
      className={cn("pv-field", size === "sm" && "pv-field--sm", className)}
      {...props}
    />
  );
}

export type FieldTextareaProps = ComponentProps<"textarea">;

export function FieldTextarea({ className, ...props }: FieldTextareaProps) {
  return <textarea className={cn("pv-textarea", className)} {...props} />;
}

export type FilterBarProps = ComponentProps<"div">;

export function FilterBar({ className, ...props }: FilterBarProps) {
  return <div className={cn("pv-filter-bar", className)} {...props} />;
}

export interface SurfaceCardProps extends ComponentProps<"div"> {
  padding?: "none" | "sm" | "md" | "panel";
  interactive?: boolean;
  selected?: boolean;
}

export function SurfaceCard({
  padding = "md",
  interactive = false,
  selected = false,
  className,
  ...props
}: SurfaceCardProps) {
  return (
    <div
      data-selected={selected ? "true" : undefined}
      className={cn(
        "pv-surface-card",
        padding === "sm" && "p-3",
        padding === "md" && "p-4",
        padding === "panel" && "p-(--pv-pad-panel)",
        interactive && "pv-surface-card--interactive",
        className,
      )}
      {...props}
    />
  );
}
