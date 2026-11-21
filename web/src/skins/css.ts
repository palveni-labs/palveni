/**
 * Turns a skin manifest into CSS custom properties for one mode.
 *
 * Skin tokens become `--pv-*`; HeroUI v3's theme variables are then pointed at them so every
 * HeroUI component follows the skin. Values were checked against their grammar by parseSkin and
 * are applied one property at a time through CSSOM (store.ts, scope.tsx), never as CSS text.
 */
import { COLOR_TOKENS, ELEVATION_TOKENS, SHAPE_TOKENS, TYPE_SIZE_TOKENS, type Mode, type SkinManifest } from "./schema";

const rem = (px: number) => `${Number((px / 16).toFixed(4))}rem`;

function familyList(stack: string[]): string {
  return stack.map((family) => (family.includes(" ") || /\d/.test(family) ? `"${family}"` : family)).join(", ");
}

/** HeroUI theme variables, mapped onto skin tokens (see @heroui/styles themes/default/variables.css). */
const HEROUI_MAPPING: Record<string, string> = {
  "--background": "var(--pv-bg)",
  "--foreground": "var(--pv-fg)",
  "--surface": "var(--pv-surface)",
  "--surface-foreground": "var(--pv-fg)",
  "--surface-secondary": "var(--pv-surface-sunken)",
  "--surface-secondary-foreground": "var(--pv-fg)",
  "--surface-tertiary": "var(--pv-hover)",
  "--surface-tertiary-foreground": "var(--pv-fg)",
  "--overlay": "var(--pv-surface-raised)",
  "--overlay-foreground": "var(--pv-fg)",
  "--muted": "var(--pv-fg-muted)",
  "--default": "var(--pv-secondary)",
  "--default-foreground": "var(--pv-secondary-fg)",
  "--accent": "var(--pv-action)",
  "--accent-foreground": "var(--pv-action-fg)",
  "--field-background": "var(--pv-field)",
  "--field-foreground": "var(--pv-fg)",
  "--field-placeholder": "var(--pv-fg-subtle)",
  "--field-border": "var(--pv-line-control)",
  "--field-border-width": "1px",
  "--success": "var(--pv-ok)",
  "--success-foreground": "var(--pv-fg-inverse)",
  "--warning": "var(--pv-degraded)",
  "--warning-foreground": "var(--pv-fg-inverse)",
  "--danger": "var(--pv-failed)",
  "--danger-foreground": "var(--pv-fg-inverse)",
  "--segment": "var(--pv-surface-raised)",
  "--segment-foreground": "var(--pv-fg)",
  "--border": "var(--pv-line)",
  "--separator": "var(--pv-line)",
  "--focus": "var(--pv-focus)",
  "--link": "var(--pv-link)",
  "--surface-shadow": "var(--pv-shadow-panel)",
  "--overlay-shadow": "var(--pv-shadow-overlay)",
  "--field-shadow": "0 0 0 0 transparent",
  "--radius": "calc(var(--pv-radius-panel) / 2)",
  "--field-radius": "var(--pv-radius-field)",
  "--scrollbar-thumb": "color-mix(in oklab, var(--pv-fg) 24%, transparent)",
  "--scrollbar-track": "transparent",
  "--scrollbar-color": "var(--scrollbar-thumb) var(--scrollbar-track)",
  "--scrollbar": "var(--scrollbar-thumb)",
  // HeroUI's calculated colours, re-declared so they follow the skin inside preview scopes too.
  "--surface-hover": "var(--pv-hover)",
  "--background-secondary": "var(--pv-surface-sunken)",
  "--background-tertiary": "var(--pv-hover)",
  "--background-inverse": "var(--pv-fg)",
  "--default-hover": "var(--pv-secondary-hover)",
  "--accent-hover": "var(--pv-action-hover)",
  "--success-hover": "color-mix(in oklab, var(--pv-ok) 90%, var(--pv-fg-inverse) 10%)",
  "--warning-hover": "color-mix(in oklab, var(--pv-degraded) 90%, var(--pv-fg-inverse) 10%)",
  "--danger-hover": "color-mix(in oklab, var(--pv-failed) 90%, var(--pv-fg-inverse) 10%)",
  "--field-hover": "var(--pv-field)",
  "--field-focus": "var(--pv-field)",
  "--field-border-hover": "color-mix(in oklab, var(--pv-line-control) 80%, var(--pv-fg) 20%)",
  "--field-border-focus": "var(--pv-focus)",
  "--default-soft": "var(--pv-secondary)",
  "--default-soft-foreground": "var(--pv-secondary-fg)",
  "--default-soft-hover": "var(--pv-secondary-hover)",
  "--accent-soft": "var(--pv-selected)",
  "--accent-soft-foreground": "var(--pv-selected-fg)",
  "--accent-soft-hover": "var(--pv-selected)",
  "--danger-soft": "var(--pv-failed-soft)",
  "--danger-soft-foreground": "var(--pv-failed)",
  "--danger-soft-hover": "var(--pv-failed-soft)",
  "--warning-soft": "var(--pv-degraded-soft)",
  "--warning-soft-foreground": "var(--pv-degraded)",
  "--warning-soft-hover": "var(--pv-degraded-soft)",
  "--success-soft": "var(--pv-ok-soft)",
  "--success-soft-foreground": "var(--pv-ok)",
  "--success-soft-hover": "var(--pv-ok-soft)",
  "--separator-secondary": "var(--pv-line)",
  "--separator-tertiary": "var(--pv-line-strong)",
  "--border-secondary": "var(--pv-line-strong)",
  "--border-tertiary": "var(--pv-line-control)",
};

/** Every CSS custom property a skin sets for one mode. */
export function skinVariables(skin: SkinManifest, mode: Mode): Record<string, string> {
  const vars: Record<string, string> = {};
  const colors = skin.colors[mode];
  for (const token of COLOR_TOKENS) vars[`--pv-${token}`] = colors[token];
  for (const token of ELEVATION_TOKENS) vars[`--pv-shadow-${token}`] = skin.elevation[mode][token];

  const { type, shape, motion } = skin;
  vars["--pv-font-heading"] = familyList(type.heading);
  vars["--pv-font-ui"] = familyList(type.ui);
  vars["--pv-font-mono"] = familyList(type.mono);
  for (const size of TYPE_SIZE_TOKENS) vars[`--pv-text-${size}`] = rem(type.size[size]);
  vars["--pv-weight-regular"] = String(type.weight.regular);
  vars["--pv-weight-medium"] = String(type.weight.medium);
  vars["--pv-weight-strong"] = String(type.weight.strong);
  vars["--pv-weight-heading"] = String(type.weight.heading);
  vars["--pv-leading"] = String(type.leading);
  vars["--pv-heading-tracking"] = `${type.headingTracking}em`;
  for (const token of SHAPE_TOKENS) {
    const value = shape[token];
    vars[`--pv-${token}`] = token.startsWith("radius") && value >= 999 ? "9999px" : rem(value);
  }
  vars["--pv-duration-fast"] = `${motion.fast}ms`;
  vars["--pv-duration-base"] = `${motion.base}ms`;
  vars["--pv-ease"] = `cubic-bezier(${motion.ease.join(", ")})`;

  const work = skin.work[mode];
  vars["--pv-wf-l"] = String(work.fieldL);
  vars["--pv-wf-k"] = String(work.fieldK);
  vars["--pv-wf-sub-l"] = String(work.subL);
  vars["--pv-ww-l"] = String(work.washL);
  vars["--pv-ww-k"] = String(work.washK);
  vars["--pv-wb-l"] = String(work.barL);
  vars["--pv-wb-k"] = String(work.barK);
  vars["--pv-on-field"] = work.onField;

  vars["color-scheme"] = mode;
  return { ...vars, ...HEROUI_MAPPING };
}

/** data-* attributes carrying the slot choices (for CSS that only needs a hint, not structure). */
export function slotAttributes(skin: SkinManifest): Record<string, string> {
  const attributes: Record<string, string> = {};
  for (const [name, value] of Object.entries(skin.slots)) {
    attributes[`data-slot-${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`] = value;
  }
  return attributes;
}
