import type { CSSProperties, ComponentProps } from "react";
import { cn } from "@/lib/cn";
import { contrast } from "./color";
import { BUILTIN_SKIN } from "./registry";
import type { ColorToken, Mode, SkinManifest } from "./schema";
import { useActiveSkin } from "./scope";
import type { ContrastPair } from "./validate";

/** Every text/background pair a decision surface draws. */
export const SECURE_PAIRS: readonly ContrastPair[] = [
  { fg: "fg", bg: "surface-raised", min: 4.5, role: "dialog text" },
  { fg: "fg-muted", bg: "surface-raised", min: 4.5, role: "dialog secondary text" },
  { fg: "fg", bg: "surface", min: 4.5, role: "request text" },
  { fg: "fg-muted", bg: "surface", min: 4.5, role: "request secondary text" },
  { fg: "fg", bg: "attention-soft", min: 4.5, role: "request text on its tint" },
  { fg: "fg-muted", bg: "attention-soft", min: 4.5, role: "request secondary text on its tint" },
  { fg: "confirm-fg", bg: "confirm", min: 4.5, role: "confirm button" },
  { fg: "action-fg", bg: "action", min: 4.5, role: "primary button" },
  { fg: "secondary-fg", bg: "secondary", min: 4.5, role: "secondary button" },
  { fg: "fg", bg: "field", min: 4.5, role: "input text" },
  { fg: "fg-inverse", bg: "failed", min: 4.5, role: "destructive button" },
  { fg: "failed", bg: "surface-raised", min: 4.5, role: "warning text" },
  { fg: "line-control", bg: "surface-raised", min: 3, role: "control boundary" },
  { fg: "focus", bg: "surface-raised", min: 3, role: "focus ring" },
];

const SECURE_TOKENS: ColorToken[] = [...new Set(SECURE_PAIRS.flatMap((pair) => [pair.fg, pair.bg]))];

export function failingSecurePairs(skin: SkinManifest, mode: Mode): ContrastPair[] {
  const colors = skin.colors[mode];
  return SECURE_PAIRS.filter((pair) => contrast(colors[pair.fg], colors[pair.bg]) + 1e-6 < pair.min);
}

/** Style overrides that restore the built-in values, or undefined when the skin passes. */
export function secureOverrides(skin: SkinManifest, mode: Mode): CSSProperties | undefined {
  if (failingSecurePairs(skin, mode).length === 0) return undefined;
  const safe = BUILTIN_SKIN.manifest.colors[mode];
  const style: Record<string, string> = {};
  for (const token of SECURE_TOKENS) style[`--pv-${token}`] = safe[token];
  return style;
}

/** Wraps a decision surface; normally a plain element. */
export function SecureSurface({ className, style, ...props }: ComponentProps<"div">) {
  const { skin, mode } = useActiveSkin();
  const overrides = secureOverrides(skin, mode);
  return <div data-secure-surface={overrides ? "fallback" : undefined} className={cn(className)} style={overrides ? { ...style, ...overrides } : style} {...props} />;
}
