import { useMemo, type CSSProperties, type ComponentProps } from "react";
import { contrast, contrastRatio, oklabToRgb, parseColor, rgbToOklab, toHex } from "./color";
import { skinVariables } from "./css";
import { BUILTIN_SKIN, type SkinEntry } from "./registry";
import type { ColorToken, Mode, SkinManifest } from "./schema";
import { useActiveSkin } from "./scope";

const decisionGrounds: ColorToken[] = ["bg", "surface", "surface-raised", "surface-sunken", "attention-soft"];
const textGrounds: [ColorToken, ColorToken[]][] = [
  ["heading", decisionGrounds],
  ["fg", [...decisionGrounds, "field", "hover"]],
  ["fg-muted", [...decisionGrounds, "hover", "secondary"]],
  ["fg-subtle", ["field", "bg", "surface"]],
  ["failed", ["bg", "surface", "surface-raised", "failed-soft"]],
  ["fg-inverse", ["failed"]],
  ["action-fg", ["action", "action-hover"]],
  ["secondary-fg", ["secondary", "secondary-hover"]],
  ["confirm-fg", ["confirm"]],
  ["selected-fg", ["selected"]],
  ["attention", ["surface", "attention-soft"]],
  ["link", ["bg", "surface", "hover"]],
  ["ok", [...decisionGrounds, "ok-soft"]],
];

function passesTextContrast(skin: SkinManifest, mode: Mode) {
  const colors = skin.colors[mode];
  return textGrounds.every(([fg, grounds]) => {
    const foreground = parseColor(colors[fg]);
    return foreground && grounds.every((token) => {
      const ground = parseColor(colors[token]);
      return ground?.a === 1 && contrastRatio(foreground, ground) >= 4.5;
    });
  });
}

// Keep the original hover/press tint where it is readable, reducing its strength as needed.
function buttonTint(background: string, foreground: string, amount: number): string {
  const bg = parseColor(background);
  const fg = parseColor(foreground);
  if (!bg || !fg || bg.a !== 1 || fg.a !== 1) return background;
  const start = rgbToOklab(bg.r, bg.g, bg.b);
  const end = rgbToOklab(fg.r, fg.g, fg.b);
  for (const weight of [amount, amount / 2, amount / 4]) {
    const mix = (key: keyof typeof start) => start[key] + (end[key] - start[key]) * weight;
    const rgb = oklabToRgb(mix("l"), mix("a"), mix("b"));
    const tint = toHex(rgb);
    if (contrast(foreground, tint) >= 4.5) return tint;
  }
  return background;
}

export function secureSurfaceProps(entry: SkinEntry, mode: Mode) {
  // A manifest (including one shown in a preview) cannot vouch for its own fonts or geometry.
  const trusted = entry.source === "builtin" || entry.packageSource === "official";
  const host = !trusted || !passesTextContrast(entry.manifest, mode);
  const skin = host ? BUILTIN_SKIN.manifest : entry.manifest;
  const colors = skin.colors[mode];
  const style: Record<string, string> = {};
  if (host) {
    const { "color-scheme": scheme, ...vars } = skinVariables(skin, mode);
    Object.assign(style, vars, {
      colorScheme: scheme,
      "--pv-font-heading": "system-ui, sans-serif",
      "--pv-font-ui": "system-ui, sans-serif",
      "--pv-font-mono": "ui-monospace, monospace",
    });
  }
  for (const token of ["focus", "line-control"] as const) {
    if (decisionGrounds.some((bg) => contrast(colors[token], colors[bg]) < 3)) {
      style[`--pv-${token}`] = colors["fg-muted"];
    }
  }
  style["--pv-secure-danger-hover"] = buttonTint(colors.failed, colors["fg-inverse"], 0.12);
  style["--pv-secure-danger-pressed"] = buttonTint(colors.failed, colors["fg-inverse"], 0.10);
  style["--pv-secure-confirm-hover"] = buttonTint(colors.confirm, colors["confirm-fg"], 0.12);
  style["--pv-secure-confirm-pressed"] = buttonTint(colors.confirm, colors["confirm-fg"], 0.18);
  return { "data-secure-surface": host ? "host" as const : "theme" as const, style: style as CSSProperties };
}

/** Apply to the element that paints the surface, including a portalled dialog or menu. */
export function useSecureSurface() {
  const { entry, mode } = useActiveSkin();
  return useMemo(() => secureSurfaceProps(entry, mode), [entry, mode]);
}

export function SecureSurface({ style, ...props }: ComponentProps<"div">) {
  const secure = useSecureSurface();
  return <div {...props} {...secure} style={{ ...style, ...secure.style }} />;
}
