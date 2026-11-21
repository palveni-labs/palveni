import { contrast, isPurpleHue, oklchToSrgb, rgbToOklch, toHex, type Oklch } from "./color";
import type { WorkRecipe } from "./schema";

export interface WorkHue {
  /** Degrees; meaningless when `c` is 0. */
  h: number;
  /** OKLCH chroma; 0 for a neutral (monochrome) cover. */
  c: number;
}

/** Pixels below this chroma count as grey. */
const CHROMATIC = 0.035;
/** A cover needs at least this share of coloured pixels, or it is treated as monochrome. */
const MIN_COLOURED_SHARE = 0.08;
/** A secondary colour must cover at least this share of the coloured pixels to stand in. */
const MIN_SECONDARY_SHARE = 0.05;
const BIN_DEGREES = 15;
const MAX_CHROMA = 0.16;

export const NEUTRAL: WorkHue = { h: 0, c: 0 };

function circularMean(hues: number[], weights: number[]): number {
  let x = 0;
  let y = 0;
  hues.forEach((hue, index) => {
    const weight = weights[index] ?? 1;
    x += Math.cos((hue * Math.PI) / 180) * weight;
    y += Math.sin((hue * Math.PI) / 180) * weight;
  });
  const mean = (Math.atan2(y, x) * 180) / Math.PI;
  return mean < 0 ? mean + 360 : mean;
}

/**
 * Main colour of an RGBA pixel buffer (as from canvas getImageData). Returns NEUTRAL for a
 * monochrome cover, and never a hue in the purple range.
 */
export function extractWorkHue(pixels: ArrayLike<number>): WorkHue {
  const bins = new Map<number, { weight: number; hues: number[]; chromas: number[]; weights: number[] }>();
  let opaque = 0;
  let coloured = 0;
  for (let index = 0; index + 3 < pixels.length; index += 4) {
    const alpha = (pixels[index + 3] ?? 0) / 255;
    if (alpha < 0.5) continue;
    opaque += 1;
    const color = rgbToOklch({ r: (pixels[index] ?? 0) / 255, g: (pixels[index + 1] ?? 0) / 255, b: (pixels[index + 2] ?? 0) / 255 });
    if (color.c < CHROMATIC || color.l < 0.12 || color.l > 0.97) continue;
    coloured += 1;
    const bin = Math.floor(color.h / BIN_DEGREES) % (360 / BIN_DEGREES);
    // Stronger colours count for more, so a small saturated title does not lose to a wash.
    const weight = Math.min(color.c, MAX_CHROMA);
    const entry = bins.get(bin) ?? { weight: 0, hues: [], chromas: [], weights: [] };
    entry.weight += weight;
    entry.hues.push(color.h);
    entry.chromas.push(color.c);
    entry.weights.push(weight);
    bins.set(bin, entry);
  }
  if (opaque === 0 || coloured / opaque < MIN_COLOURED_SHARE) return NEUTRAL;

  // Merge each bin with its neighbours so a hue on a bin edge is not split in two.
  const total = [...bins.values()].reduce((sum, entry) => sum + entry.weight, 0);
  const ranked = [...bins.entries()]
    .map(([bin, entry]) => {
      const neighbours = [bin - 1, bin + 1].map((n) => bins.get((n + 24) % 24)?.weight ?? 0);
      return { bin, entry, score: entry.weight + 0.5 * (neighbours[0] ?? 0) + 0.5 * (neighbours[1] ?? 0) };
    })
    .sort((a, b) => b.score - a.score);

  for (const candidate of ranked) {
    const hue = circularMean(candidate.entry.hues, candidate.entry.weights);
    if (isPurpleHue(hue)) continue;
    if (candidate !== ranked[0] && candidate.entry.weight / total < MIN_SECONDARY_SHARE) break;
    const chromas = [...candidate.entry.chromas].sort((a, b) => a - b);
    const median = chromas[Math.floor(chromas.length / 2)] ?? 0;
    return { h: Number(hue.toFixed(1)), c: Number(Math.min(median, MAX_CHROMA).toFixed(4)) };
  }
  return NEUTRAL;
}

export interface WorkColors {
  /** Whole region in the work's colour. */
  field: string;
  /** Text on the field. */
  fieldFg: string;
  /** Secondary text on the field. */
  fieldSub: string;
  /** Progress wash behind a row of ordinary text. */
  wash: string;
  /** The exact progress edge. */
  bar: string;
}

export interface WorkContext {
  /** Text drawn on the wash (the skin's body text colour for the mode). */
  fg: string;
  /** The track a progress edge is read against. */
  track: string;
  mode: "light" | "dark";
}

function hex(color: Oklch): string {
  return toHex(oklchToSrgb(color));
}

/** Moves lightness one step at a time (away from the text colour) until `ok` holds. */
function settle(start: Oklch, ok: (candidate: string) => boolean, direction: 1 | -1): Oklch {
  let color = start;
  for (let step = 0; step < 40 && !ok(hex(color)); step++) {
    color = { ...color, l: Math.min(0.98, Math.max(0.05, color.l + direction * 0.01)) };
  }
  return color;
}

/** The colours one mode draws for a work, each meeting its contrast floor (sRGB hex, gamut-mapped). */
export function renderWorkColor(hue: WorkHue, recipe: WorkRecipe, context: WorkContext): WorkColors {
  const { h, c } = hue;
  const onField = recipe.onField;
  const onFieldIsLight = contrast(onField, "#000000") > contrast(onField, "#ffffff");
  const fgIsLight = contrast(context.fg, "#000000") > contrast(context.fg, "#ffffff");

  const field = settle({ l: recipe.fieldL, c: c * recipe.fieldK, h }, (value) => contrast(onField, value) >= 4.5, onFieldIsLight ? -1 : 1);
  const sub = settle({ l: recipe.subL, c: c * 0.4, h }, (value) => contrast(value, hex(field)) >= 4.5, onFieldIsLight ? 1 : -1);
  const wash = settle({ l: recipe.washL, c: c * recipe.washK, h }, (value) => contrast(context.fg, value) >= 4.5, fgIsLight ? -1 : 1);
  const bar = settle({ l: recipe.barL, c: c * recipe.barK, h }, (value) => contrast(value, context.track) >= 3, context.mode === "light" ? -1 : 1);

  return {
    field: hex(field),
    fieldFg: onField,
    fieldSub: hex(sub),
    wash: hex(wash),
    bar: hex(bar),
  };
}
