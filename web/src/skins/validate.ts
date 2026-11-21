import { contrast, isPurple, parseColor } from "./color";
import { COLOR_TOKENS, MODES, type ColorToken, type Mode, type SkinManifest } from "./schema";

export interface ContrastPair {
  fg: ColorToken;
  bg: ColorToken;
  /** 4.5 for text, 3 for large text, icons and control boundaries. */
  min: 3 | 4.5;
  role: string;
}

/** The pairs the host renders; a skin passes only if every one meets its minimum. */
export const KEY_PAIRS: readonly ContrastPair[] = [
  // Body and secondary text on every ground it appears on.
  { fg: "fg", bg: "bg", min: 4.5, role: "body text on page" },
  { fg: "fg", bg: "surface", min: 4.5, role: "body text on surface" },
  { fg: "fg", bg: "surface-raised", min: 4.5, role: "body text in overlays" },
  { fg: "fg", bg: "surface-sunken", min: 4.5, role: "body text in wells" },
  { fg: "fg", bg: "aside", min: 4.5, role: "body text in the side column" },
  { fg: "fg-muted", bg: "bg", min: 4.5, role: "secondary text on page" },
  { fg: "fg-muted", bg: "surface", min: 4.5, role: "secondary text on surface" },
  { fg: "fg-muted", bg: "aside", min: 4.5, role: "secondary text in the side column" },
  { fg: "fg-subtle", bg: "surface", min: 4.5, role: "meta text on surface" },
  { fg: "fg-subtle", bg: "bg", min: 4.5, role: "meta text on the page" },
  { fg: "fg-subtle", bg: "field", min: 4.5, role: "placeholder in fields" },
  { fg: "heading", bg: "bg", min: 4.5, role: "headings on page" },
  { fg: "fg", bg: "field", min: 4.5, role: "field text" },
  // Shell.
  { fg: "shell-fg", bg: "shell", min: 4.5, role: "shell text" },
  { fg: "shell-fg-muted", bg: "shell", min: 4.5, role: "secondary shell text" },
  { fg: "nav-active-fg", bg: "nav-active", min: 4.5, role: "current navigation item" },
  { fg: "shell-fg-muted", bg: "shell-field", min: 4.5, role: "search placeholder in the shell" },
  // Actions.
  { fg: "action-fg", bg: "action", min: 4.5, role: "primary button" },
  { fg: "secondary-fg", bg: "secondary", min: 4.5, role: "secondary button" },
  { fg: "confirm-fg", bg: "confirm", min: 4.5, role: "confirm button" },
  { fg: "selected-fg", bg: "selected", min: 4.5, role: "selected item" },
  { fg: "link", bg: "bg", min: 4.5, role: "links on page" },
  { fg: "link", bg: "surface", min: 4.5, role: "links on surface" },
  // States: text on surface and on its own soft ground.
  { fg: "active", bg: "surface", min: 4.5, role: "in-progress label" },
  { fg: "attention", bg: "surface", min: 4.5, role: "needs-you label" },
  { fg: "attention", bg: "attention-soft", min: 4.5, role: "needs-you label on its tint" },
  { fg: "fg", bg: "attention-soft", min: 4.5, role: "text in a needs-you tint" },
  { fg: "ok", bg: "surface", min: 4.5, role: "healthy label" },
  { fg: "degraded", bg: "surface", min: 4.5, role: "degraded label" },
  { fg: "degraded", bg: "bg", min: 4.5, role: "degraded label on the page" },
  { fg: "degraded", bg: "degraded-soft", min: 4.5, role: "degraded label on its tint" },
  { fg: "fg", bg: "degraded-soft", min: 4.5, role: "text in a degraded tint" },
  { fg: "failed", bg: "surface", min: 4.5, role: "failure label" },
  { fg: "failed", bg: "bg", min: 4.5, role: "failure label on the page" },
  { fg: "failed", bg: "failed-soft", min: 4.5, role: "failure label on its tint" },
  { fg: "idle", bg: "surface", min: 4.5, role: "idle label" },
  // Non-text: control boundaries, focus, marks and progress.
  { fg: "line-control", bg: "surface", min: 3, role: "control boundary on surface" },
  { fg: "line-control", bg: "bg", min: 3, role: "control boundary on page" },
  { fg: "focus", bg: "bg", min: 3, role: "focus ring on page" },
  { fg: "focus", bg: "surface", min: 3, role: "focus ring on surface" },
  { fg: "active-mark", bg: "bg", min: 3, role: "in-progress mark" },
  { fg: "attention-mark", bg: "bg", min: 3, role: "needs-you mark" },
  { fg: "attention-mark-fg", bg: "attention-mark", min: 4.5, role: "count on a needs-you mark" },
  { fg: "icon", bg: "bg", min: 3, role: "icons on page" },
  { fg: "icon", bg: "surface", min: 3, role: "icons on surface" },
  { fg: "progress-done", bg: "progress-rest", min: 3, role: "progress against its track" },
  { fg: "progress-done", bg: "surface", min: 3, role: "progress on surface" },
  { fg: "action", bg: "bg", min: 3, role: "primary button against the page" },
];

export type SkinIssue =
  | { kind: "purple"; mode: Mode | "shared"; token: string; value: string }
  | { kind: "contrast"; mode: Mode; pair: ContrastPair; ratio: number }
  | { kind: "opaque"; mode: Mode; token: ColorToken; value: string };

/** Tokens drawn as grounds must be opaque so contrast is measured against what is rendered. */
const OPAQUE: readonly ColorToken[] = [
  "bg",
  "shell",
  "aside",
  "surface",
  "surface-raised",
  "surface-sunken",
  "field",
  "nav-active",
  "action",
  "secondary",
  "confirm",
  "selected",
  "attention-soft",
  "degraded-soft",
  "failed-soft",
  "progress-rest",
];

const SHADOW_COLOURS = /#[0-9a-fA-F]{3,8}|(?:rgba?|oklch|oklab)\([^)]*\)/g;

export function checkSkin(skin: SkinManifest): SkinIssue[] {
  const issues: SkinIssue[] = [];
  for (const mode of MODES) {
    const colors = skin.colors[mode];
    for (const token of COLOR_TOKENS) {
      if (isPurple(colors[token])) issues.push({ kind: "purple", mode, token, value: colors[token] });
    }
    for (const token of OPAQUE) {
      if ((parseColor(colors[token])?.a ?? 1) < 1) issues.push({ kind: "opaque", mode, token, value: colors[token] });
    }
    for (const [token, shadow] of Object.entries(skin.elevation[mode])) {
      for (const value of shadow.match(SHADOW_COLOURS) ?? []) {
        if (isPurple(value)) issues.push({ kind: "purple", mode, token: `elevation.${token}`, value });
      }
    }
    if (isPurple(skin.work[mode].onField)) issues.push({ kind: "purple", mode, token: "work.onField", value: skin.work[mode].onField });
    for (const pair of KEY_PAIRS) {
      const ratio = contrast(colors[pair.fg], colors[pair.bg]);
      if (ratio + 1e-6 < pair.min) issues.push({ kind: "contrast", mode, pair, ratio });
    }
  }
  return issues;
}

export function describeIssue(issue: SkinIssue): string {
  switch (issue.kind) {
    case "purple":
      return `${issue.mode}: ${issue.token} (${issue.value}) is in the purple range`;
    case "opaque":
      return `${issue.mode}: ${issue.token} (${issue.value}) must be opaque`;
    case "contrast":
      return `${issue.mode}: ${issue.pair.fg} on ${issue.pair.bg} is ${issue.ratio.toFixed(2)}:1, needs ${issue.pair.min}:1 (${issue.pair.role})`;
  }
}
