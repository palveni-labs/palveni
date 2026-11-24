/**
 * Skins: pure data that decides how the workspace looks.
 *
 * A skin is a typed manifest of token values (light and dark), one choice per variant slot, and
 * font assets. It contains no code and no CSS: every value goes through a narrow grammar before
 * it reaches a CSS variable, so a third-party skin cannot inject `url(...)`, selectors or script.
 * Built-in skins are parsed by the same function.
 */
import { isColor } from "./color";

export const SKIN_FORMAT = 1;

/* ---------------------------------------------------------------- variant slots */

/** Structural variants the host implements; a skin picks one value per slot. */
export const SLOT_VALUES = {
  nav: ["sidebar", "topbar"],
  navPhone: ["bottom-bar", "top-scroll"],
  status: ["activity-window", "inline"],
  progress: ["bar", "separator", "row-fill"],
  attention: ["tint", "seal", "lamp", "thumbs"],
  section: ["plain", "rule", "panel"],
  itemColor: ["on", "off"],
  measure: ["sans", "mono"],
  pageTitle: ["toolbar", "content"],
  summary: ["cells", "inline"],
} as const;

export type SlotName = keyof typeof SLOT_VALUES;
export type Slots = { [K in SlotName]: (typeof SLOT_VALUES)[K][number] };
export const SLOT_NAMES = Object.keys(SLOT_VALUES) as SlotName[];

/* ---------------------------------------------------------------- tokens */

/** Colour tokens, named by job. Every skin sets all of them, separately for light and dark. */
export const COLOR_TOKENS = [
  // 底色
  "bg",
  "shell",
  "aside",
  "surface",
  "surface-raised",
  "surface-sunken",
  "hover",
  "field",
  // 文字
  "fg",
  "fg-muted",
  "fg-subtle",
  "fg-disabled",
  "fg-inverse",
  "heading",
  "icon",
  // 外壳
  "shell-fg",
  "shell-fg-muted",
  "shell-line",
  "shell-field",
  "nav-track",
  "nav-active",
  "nav-active-fg",
  "nav-indicator",
  // 线
  "line",
  "line-strong",
  "line-control",
  // 行动
  "action",
  "action-fg",
  "action-hover",
  "secondary",
  "secondary-fg",
  "secondary-hover",
  "confirm",
  "confirm-fg",
  "link",
  "selected",
  "selected-fg",
  "focus",
  "text-selection",
  // 状态：正在发生、需要你、正常、降级、失败、空闲
  "active",
  "active-mark",
  "active-soft",
  "attention",
  "attention-mark",
  "attention-mark-fg",
  "attention-soft",
  "attention-line",
  "ok",
  "ok-soft",
  "degraded",
  "degraded-soft",
  "degraded-line",
  "failed",
  "failed-soft",
  "idle",
  "idle-soft",
  // 数据
  "progress-done",
  "progress-rest",
  "chart-1",
  "chart-2",
  "chart-3",
  "chart-4",
  "chart-5",
] as const;

export type ColorToken = (typeof COLOR_TOKENS)[number];
export type ColorTokens = Record<ColorToken, string>;

/** 层次: one shadow per job, per mode. */
export const ELEVATION_TOKENS = ["panel", "raised", "overlay", "art"] as const;
export type ElevationToken = (typeof ELEVATION_TOKENS)[number];
export type ElevationTokens = Record<ElevationToken, string>;

/** 字体: type scale in px, weights, line height and heading tracking. */
export const TYPE_SIZE_TOKENS = ["xs", "sm", "base", "md", "lg", "xl", "page", "stat"] as const;
export type TypeSize = (typeof TYPE_SIZE_TOKENS)[number];

export interface TypeTokens {
  /** Family stacks, most specific first; the last entry is a generic family. */
  heading: string[];
  ui: string[];
  mono: string[];
  size: Record<TypeSize, number>;
  weight: { regular: number; medium: number; strong: number; heading: number };
  /** Body line height (unitless). */
  leading: number;
  /** Heading letter spacing in em. */
  headingTracking: number;
}

/** 形状: radii, control and row heights, spacing, and the shell's fixed measures (px). */
export const SHAPE_TOKENS = [
  "radius-sm",
  "radius-control",
  "radius-field",
  "radius-chip",
  "radius-tile",
  "radius-panel",
  "control-h",
  "control-h-sm",
  "row-h",
  "space",
  "pad-panel",
  "gap-section",
  "gutter",
  "shell-h",
  "sidebar-w",
  "aside-w",
  "conversation-w",
] as const;
export type ShapeToken = (typeof SHAPE_TOKENS)[number];
export type ShapeTokens = Record<ShapeToken, number>;

/** 动效 */
export interface MotionTokens {
  fast: number;
  base: number;
  /** cubic-bezier control points. */
  ease: [number, number, number, number];
}

/**
 * Work colour recipe, per mode. The hue and chroma come from the cover; the skin
 * only fixes lightness and a chroma factor for each use.
 */
export interface WorkRecipe {
  /** Whole region in the work's colour (white text on it). */
  fieldL: number;
  fieldK: number;
  /** Secondary text on a field. */
  subL: number;
  /** Progress wash behind a row (ink text on it). */
  washL: number;
  washK: number;
  /** The exact progress edge. */
  barL: number;
  barK: number;
  /** Text colour on a field. */
  onField: string;
}

export interface FontAsset {
  family: string;
  /** Same-origin URL of a woff2 file. */
  src: string;
  /** "400" or a variable range such as "100 900". */
  weight: string;
  style: "normal" | "italic";
  unicodeRange?: string;
  /** Percent; tunes a Latin face against the CJK face it is paired with. */
  sizeAdjust?: number;
  /** Percent of the em; lines up mixed Chinese and Latin line boxes. */
  ascentOverride?: number;
  descentOverride?: number;
  /** Declared file size; the server refuses packages whose files exceed it or MAX_FONT_BYTES. */
  bytes?: number;
}

export interface LocalizedText {
  "zh-CN"?: string;
  en?: string;
}

export interface SkinManifest {
  format: typeof SKIN_FORMAT;
  id: string;
  name: LocalizedText;
  description?: LocalizedText;
  author?: string;
  slots: Slots;
  colors: { light: ColorTokens; dark: ColorTokens };
  elevation: { light: ElevationTokens; dark: ElevationTokens };
  work: { light: WorkRecipe; dark: WorkRecipe };
  type: TypeTokens;
  shape: ShapeTokens;
  motion: MotionTokens;
  fonts: FontAsset[];
}

export type Mode = "light" | "dark";
export const MODES: readonly Mode[] = ["light", "dark"];

export class SkinError extends Error {
  override readonly name = "SkinError";
}

/* ---------------------------------------------------------------- parsing */

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function record(value: unknown, field: string): Record<string, unknown> {
  if (!isRecord(value)) throw new SkinError(`${field} must be an object`);
  return value;
}

/** No control characters and no markup brackets. */
function isPlainText(value: string): boolean {
  for (const char of value) {
    const code = char.charCodeAt(0);
    if (code < 0x20 || code === 0x7f || char === "<" || char === ">") return false;
  }
  return true;
}

function text(value: unknown, field: string): LocalizedText {
  const input = record(value, field);
  const result: LocalizedText = {};
  for (const lang of ["zh-CN", "en"] as const) {
    const entry = input[lang];
    if (entry === undefined) continue;
    if (typeof entry !== "string" || entry.length === 0 || entry.length > 120 || !isPlainText(entry)) {
      throw new SkinError(`${field}.${lang} must be a short line of plain text`);
    }
    result[lang] = entry;
  }
  if (!result["zh-CN"] && !result.en) throw new SkinError(`${field} needs at least one language`);
  return result;
}

function number(value: unknown, field: string, min: number, max: number): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) {
    throw new SkinError(`${field} must be a number from ${min} to ${max}`);
  }
  return value;
}

function exactKeys<K extends string>(input: Record<string, unknown>, keys: readonly K[], field: string) {
  for (const key of Object.keys(input)) {
    if (!(keys as readonly string[]).includes(key)) throw new SkinError(`${field}.${key} is not a known token`);
  }
  for (const key of keys) {
    if (!(key in input)) throw new SkinError(`${field}.${key} is missing`);
  }
}

function colors(value: unknown, field: string): ColorTokens {
  const input = record(value, field);
  exactKeys(input, COLOR_TOKENS, field);
  const result = {} as ColorTokens;
  for (const token of COLOR_TOKENS) {
    const entry = input[token];
    if (!isColor(entry)) throw new SkinError(`${field}.${token} is not a colour`);
    result[token] = entry.trim();
  }
  return result;
}

const LENGTH = String.raw`-?\d+(?:\.\d+)?(?:px)?`;
const SHADOW_COLOR = String.raw`(?:#[0-9a-fA-F]{3,8}|(?:rgba?|oklch|oklab)\([0-9.,%\s/+-]+\))`;
const SHADOW_LAYER = new RegExp(String.raw`^(?:inset\s+)?${LENGTH}\s+${LENGTH}(?:\s+${LENGTH}){0,2}\s+${SHADOW_COLOR}$`);

/** "none" or comma-separated layers of `[inset] x y [blur [spread]] colour`. */
export function isShadow(value: unknown): value is string {
  if (typeof value !== "string" || value.length > 400) return false;
  const trimmed = value.trim();
  if (trimmed === "none") return true;
  // Split on commas that are not inside parentheses.
  const layers = trimmed.split(/,(?![^(]*\))/).map((layer) => layer.trim());
  return layers.length > 0 && layers.every((layer) => SHADOW_LAYER.test(layer));
}

function elevation(value: unknown, field: string): ElevationTokens {
  const input = record(value, field);
  exactKeys(input, ELEVATION_TOKENS, field);
  const result = {} as ElevationTokens;
  for (const token of ELEVATION_TOKENS) {
    const entry = input[token];
    if (!isShadow(entry)) throw new SkinError(`${field}.${token} is not a shadow`);
    result[token] = entry.trim();
  }
  return result;
}

function work(value: unknown, field: string): WorkRecipe {
  const input = record(value, field);
  const keys = ["fieldL", "fieldK", "subL", "washL", "washK", "barL", "barK", "onField"] as const;
  exactKeys(input, keys, field);
  if (!isColor(input.onField)) throw new SkinError(`${field}.onField is not a colour`);
  return {
    fieldL: number(input.fieldL, `${field}.fieldL`, 0, 1),
    fieldK: number(input.fieldK, `${field}.fieldK`, 0, 2),
    subL: number(input.subL, `${field}.subL`, 0, 1),
    washL: number(input.washL, `${field}.washL`, 0, 1),
    washK: number(input.washK, `${field}.washK`, 0, 2),
    barL: number(input.barL, `${field}.barL`, 0, 1),
    barK: number(input.barK, `${field}.barK`, 0, 2),
    onField: input.onField.trim(),
  };
}

const GENERIC_FAMILIES = ["sans-serif", "serif", "monospace", "system-ui", "ui-monospace", "ui-sans-serif", "ui-serif"];
/** Plain family names only (letters, digits, spaces, hyphens); `-apple-system` is the one keyword allowed. */
const FAMILY_NAME = /^(?:-apple-system|[A-Za-z0-9][A-Za-z0-9 -]{0,62})$/;

function familyStack(value: unknown, field: string): string[] {
  if (!Array.isArray(value) || value.length === 0 || value.length > 16) throw new SkinError(`${field} must list 1–16 families`);
  const stack = value.map((entry, index) => {
    if (typeof entry !== "string" || !FAMILY_NAME.test(entry)) throw new SkinError(`${field}[${index}] is not a font family name`);
    return entry;
  });
  if (!GENERIC_FAMILIES.includes(stack[stack.length - 1] ?? "")) throw new SkinError(`${field} must end with a generic family`);
  return stack;
}

function type(value: unknown): TypeTokens {
  const input = record(value, "type");
  exactKeys(input, ["heading", "ui", "mono", "size", "weight", "leading", "headingTracking"], "type");
  const size = record(input.size, "type.size");
  exactKeys(size, TYPE_SIZE_TOKENS, "type.size");
  const weight = record(input.weight, "type.weight");
  exactKeys(weight, ["regular", "medium", "strong", "heading"], "type.weight");
  const sizes = {} as Record<TypeSize, number>;
  for (const token of TYPE_SIZE_TOKENS) sizes[token] = number(size[token], `type.size.${token}`, 9, 48);
  const w = (key: string) => number(weight[key], `type.weight.${key}`, 100, 900);
  return {
    heading: familyStack(input.heading, "type.heading"),
    ui: familyStack(input.ui, "type.ui"),
    mono: familyStack(input.mono, "type.mono"),
    size: sizes,
    weight: { regular: w("regular"), medium: w("medium"), strong: w("strong"), heading: w("heading") },
    leading: number(input.leading, "type.leading", 1, 2.2),
    headingTracking: number(input.headingTracking, "type.headingTracking", -0.06, 0.1),
  };
}

const SHAPE_LIMITS: Record<ShapeToken, [number, number]> = {
  "radius-sm": [0, 12],
  "radius-control": [0, 999],
  "radius-field": [0, 999],
  "radius-chip": [0, 999],
  "radius-tile": [0, 32],
  "radius-panel": [0, 32],
  "control-h": [24, 48],
  "control-h-sm": [20, 40],
  "row-h": [28, 96],
  space: [2, 8],
  "pad-panel": [0, 40],
  "gap-section": [8, 80],
  gutter: [8, 80],
  "shell-h": [40, 80],
  "sidebar-w": [180, 320],
  "aside-w": [280, 440],
  "conversation-w": [340, 520],
};

function shape(value: unknown): ShapeTokens {
  const input = record(value, "shape");
  exactKeys(input, SHAPE_TOKENS, "shape");
  const result = {} as ShapeTokens;
  for (const token of SHAPE_TOKENS) {
    const [min, max] = SHAPE_LIMITS[token];
    result[token] = number(input[token], `shape.${token}`, min, max);
  }
  return result;
}

function motion(value: unknown): MotionTokens {
  const input = record(value, "motion");
  exactKeys(input, ["fast", "base", "ease"], "motion");
  const ease = input.ease;
  if (!Array.isArray(ease) || ease.length !== 4) throw new SkinError("motion.ease must be four cubic-bezier numbers");
  const [x1, y1, x2, y2] = ease.map((entry, index) => number(entry, `motion.ease[${index}]`, index % 2 === 0 ? 0 : -2, index % 2 === 0 ? 1 : 3));
  return {
    fast: number(input.fast, "motion.fast", 0, 600),
    base: number(input.base, "motion.base", 0, 900),
    ease: [x1 ?? 0, y1 ?? 0, x2 ?? 1, y2 ?? 1],
  };
}

function slots(value: unknown): Slots {
  const input = record(value, "slots");
  exactKeys(input, SLOT_NAMES, "slots");
  const result: Record<string, string> = {};
  for (const name of SLOT_NAMES) {
    const entry = input[name];
    const allowed = SLOT_VALUES[name] as readonly string[];
    if (typeof entry !== "string" || !allowed.includes(entry)) throw new SkinError(`slots.${name} must be one of ${allowed.join(", ")}`);
    result[name] = entry;
  }
  return result as Slots;
}

/**
 * Where a font file may come from. A skin package names files inside its own `fonts/` folder;
 * the host resolves them to the server's asset URL. Only the built-in skin, compiled into the
 * app, uses same-origin absolute paths. Nothing is ever loaded from another host.
 */
export type FontSource = "package" | "builtin";
const PACKAGE_FONT = /^fonts\/[A-Za-z0-9][A-Za-z0-9._-]{0,120}\.woff2$/;
const BUILTIN_FONT = /^\/(?!\/)[A-Za-z0-9._~@\-/]+\.woff2$/;
const UNICODE_RANGE = /^U\+[0-9A-Fa-f?]{1,6}(?:-[0-9A-Fa-f]{1,6})?(?:,\s*U\+[0-9A-Fa-f?]{1,6}(?:-[0-9A-Fa-f]{1,6})?)*$/;
const WEIGHT = /^\d{3}(?: \d{3})?$/;
/** Largest font file a skin may declare (bytes). */
export const MAX_FONT_BYTES = 2 * 1024 * 1024;

export function parseFontAsset(value: unknown, field: string, source: FontSource = "package"): FontAsset {
  const input = record(value, field);
  const allowed = ["family", "src", "weight", "style", "unicodeRange", "sizeAdjust", "ascentOverride", "descentOverride", "bytes"];
  for (const key of Object.keys(input)) if (!allowed.includes(key)) throw new SkinError(`${field}.${key} is not a known field`);
  const { family, src, weight, style, unicodeRange, sizeAdjust, ascentOverride, descentOverride, bytes } = input;
  if (typeof family !== "string" || !FAMILY_NAME.test(family)) throw new SkinError(`${field}.family is not a font family name`);
  const pattern = source === "package" ? PACKAGE_FONT : BUILTIN_FONT;
  if (typeof src !== "string" || !pattern.test(src) || src.includes("..")) {
    throw new SkinError(source === "package" ? `${field}.src must name a .woff2 file in the package's fonts/ folder` : `${field}.src must be a same-origin .woff2 path`);
  }
  if (typeof weight !== "string" || !WEIGHT.test(weight)) throw new SkinError(`${field}.weight must look like "400" or "100 900"`);
  if (style !== "normal" && style !== "italic") throw new SkinError(`${field}.style must be normal or italic`);
  if (unicodeRange !== undefined && (typeof unicodeRange !== "string" || unicodeRange.length > 6000 || !UNICODE_RANGE.test(unicodeRange))) {
    throw new SkinError(`${field}.unicodeRange is not a unicode-range`);
  }
  if (bytes !== undefined) number(bytes, `${field}.bytes`, 1, MAX_FONT_BYTES);
  const percent = (value: unknown, name: string, min: number, max: number) =>
    value === undefined ? {} : { [name]: number(value, `${field}.${name}`, min, max) };
  return {
    family,
    src,
    weight,
    style,
    ...(unicodeRange ? { unicodeRange } : {}),
    ...percent(sizeAdjust, "sizeAdjust", 50, 200),
    ...percent(ascentOverride, "ascentOverride", 0, 200),
    ...percent(descentOverride, "descentOverride", 0, 200),
    ...(typeof bytes === "number" ? { bytes } : {}),
  };
}

const ROOT_KEYS = ["format", "id", "name", "description", "author", "slots", "colors", "elevation", "work", "type", "shape", "motion", "fonts"];

/**
 * Validates untrusted JSON and returns a normalized manifest, or throws SkinError. Unknown keys
 * are refused at every level; every value must match the grammar of its type.
 */
export function parseSkin(input: unknown, fontSource: FontSource = "package"): SkinManifest {
  const root = record(input, "skin");
  for (const key of Object.keys(root)) if (!ROOT_KEYS.includes(key)) throw new SkinError(`${key} is not a known field`);
  if (root.format !== SKIN_FORMAT) throw new SkinError(`Unsupported format (expected ${SKIN_FORMAT})`);
  const id = root.id;
  if (typeof id !== "string" || !/^[a-z0-9][a-z0-9.-]{1,62}$/.test(id)) {
    throw new SkinError("id must be 2–63 lowercase letters, digits, dots or dashes");
  }
  if (root.author !== undefined && (typeof root.author !== "string" || root.author.length > 80 || !isPlainText(root.author))) {
    throw new SkinError("author must be a short string");
  }
  const colorSet = record(root.colors, "colors");
  const shadowSet = record(root.elevation, "elevation");
  const workSet = record(root.work, "work");
  const fonts = root.fonts ?? [];
  if (!Array.isArray(fonts) || fonts.length > 400) throw new SkinError("fonts must be a list of at most 400 assets");
  return {
    format: SKIN_FORMAT,
    id,
    name: text(root.name, "name"),
    description: root.description === undefined ? undefined : text(root.description, "description"),
    author: typeof root.author === "string" ? root.author : undefined,
    slots: slots(root.slots),
    colors: { light: colors(colorSet.light, "colors.light"), dark: colors(colorSet.dark, "colors.dark") },
    elevation: { light: elevation(shadowSet.light, "elevation.light"), dark: elevation(shadowSet.dark, "elevation.dark") },
    work: { light: work(workSet.light, "work.light"), dark: work(workSet.dark, "work.dark") },
    type: type(root.type),
    shape: shape(root.shape),
    motion: motion(root.motion),
    fonts: fonts.map((entry, index) => parseFontAsset(entry, `fonts[${index}]`, fontSource)),
  };
}

export function localized(text: LocalizedText | undefined, language: string): string {
  if (!text) return "";
  return (language === "zh-CN" ? (text["zh-CN"] ?? text.en) : (text.en ?? text["zh-CN"])) ?? "";
}
