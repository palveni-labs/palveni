export interface Rgba {
  /** 0–1, gamma-encoded sRGB. */
  r: number;
  g: number;
  b: number;
  /** 0–1. */
  a: number;
}

export interface Oklch {
  l: number;
  c: number;
  /** Degrees, 0–360. */
  h: number;
}

const NUMBER = String.raw`[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?`;
const HEX = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const FUNC = new RegExp(String.raw`^(rgba?|oklch|oklab)\(\s*(${NUMBER}%?)[\s,]+(${NUMBER}%?)[\s,]+(${NUMBER}(?:deg)?%?)\s*(?:[,/]\s*(${NUMBER}%?)\s*)?\)$`, "i");

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

function amount(raw: string, scale: number): number {
  return raw.endsWith("%") ? (parseFloat(raw) / 100) * scale : parseFloat(raw);
}

function alpha(raw: string | undefined): number {
  if (raw === undefined) return 1;
  return clamp01(raw.endsWith("%") ? parseFloat(raw) / 100 : parseFloat(raw));
}

/* ---------------------------------------------------------------- conversions */

const toLinear = (value: number) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
const toGamma = (value: number) => (value <= 0.0031308 ? value * 12.92 : 1.055 * value ** (1 / 2.4) - 0.055);

export function oklabToRgb(l: number, a: number, b: number): { r: number; g: number; b: number } {
  const l_ = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m_ = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s_ = (l - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return {
    r: toGamma(4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_),
    g: toGamma(-1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_),
    b: toGamma(-0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_),
  };
}

export function rgbToOklab(r: number, g: number, b: number): { l: number; a: number; b: number } {
  const lr = toLinear(r);
  const lg = toLinear(g);
  const lb = toLinear(b);
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
  return {
    l: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  };
}

export function rgbToOklch(color: Pick<Rgba, "r" | "g" | "b">): Oklch {
  const lab = rgbToOklab(color.r, color.g, color.b);
  const c = Math.hypot(lab.a, lab.b);
  let h = (Math.atan2(lab.b, lab.a) * 180) / Math.PI;
  if (h < 0) h += 360;
  return { l: lab.l, c, h: c < 1e-4 ? 0 : h };
}

export function oklchToRgb({ l, c, h }: Oklch): { r: number; g: number; b: number } {
  const rad = (h * Math.PI) / 180;
  return oklabToRgb(l, c * Math.cos(rad), c * Math.sin(rad));
}

function inGamut({ r, g, b }: { r: number; g: number; b: number }, epsilon = 1e-4) {
  return [r, g, b].every((channel) => channel >= -epsilon && channel <= 1 + epsilon);
}

/** Reduces chroma until the colour fits sRGB (keeps lightness and hue, like CSS gamut mapping). */
export function oklchToSrgb(color: Oklch): { r: number; g: number; b: number } {
  let rgb = oklchToRgb(color);
  if (inGamut(rgb)) return { r: clamp01(rgb.r), g: clamp01(rgb.g), b: clamp01(rgb.b) };
  let low = 0;
  let high = color.c;
  for (let step = 0; step < 24; step++) {
    const mid = (low + high) / 2;
    if (inGamut(oklchToRgb({ ...color, c: mid }))) low = mid;
    else high = mid;
  }
  rgb = oklchToRgb({ ...color, c: low });
  return { r: clamp01(rgb.r), g: clamp01(rgb.g), b: clamp01(rgb.b) };
}

/* ---------------------------------------------------------------- parsing */

/** Parses the colour grammar skins may use; returns null for anything else. */
export function parseColor(input: string): Rgba | null {
  const value = input.trim();
  if (value.length > 80) return null;
  const hex = HEX.exec(value);
  if (hex) {
    let digits = hex[1] ?? "";
    if (digits.length <= 4) digits = [...digits].map((digit) => digit + digit).join("");
    const channel = (index: number) => parseInt(digits.slice(index * 2, index * 2 + 2), 16) / 255;
    return { r: channel(0), g: channel(1), b: channel(2), a: digits.length === 8 ? channel(3) : 1 };
  }
  const match = FUNC.exec(value);
  if (!match) return null;
  const [, fn = "", x = "0", y = "0", z = "0", a] = match;
  switch (fn.toLowerCase()) {
    case "rgb":
    case "rgba":
      return { r: clamp01(amount(x, 255) / 255), g: clamp01(amount(y, 255) / 255), b: clamp01(amount(z, 255) / 255), a: alpha(a) };
    case "oklch": {
      const rgb = oklchToSrgb({ l: amount(x, 1), c: amount(y, 0.4), h: parseFloat(z) });
      return { ...rgb, a: alpha(a) };
    }
    case "oklab": {
      const rgb = oklabToRgb(amount(x, 1), amount(y, 0.4), amount(z, 0.4));
      return { r: clamp01(rgb.r), g: clamp01(rgb.g), b: clamp01(rgb.b), a: alpha(a) };
    }
    default:
      return null;
  }
}

export function isColor(value: unknown): value is string {
  return typeof value === "string" && parseColor(value) !== null;
}

/** OKLCH of a colour string (the declared hue for oklch() input, measured otherwise). */
export function toOklch(input: string): Oklch | null {
  const match = FUNC.exec(input.trim());
  if (match && match[1]?.toLowerCase() === "oklch") {
    return { l: amount(match[2] ?? "0", 1), c: amount(match[3] ?? "0", 0.4), h: parseFloat(match[4] ?? "0") };
  }
  const rgb = parseColor(input);
  return rgb ? rgbToOklch(rgb) : null;
}

/* ---------------------------------------------------------------- measurement */

export function relativeLuminance({ r, g, b }: Pick<Rgba, "r" | "g" | "b">): number {
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

/** Composites a translucent colour over an opaque one. */
export function over(top: Rgba, bottom: Pick<Rgba, "r" | "g" | "b">): Rgba {
  return {
    r: top.r * top.a + bottom.r * (1 - top.a),
    g: top.g * top.a + bottom.g * (1 - top.a),
    b: top.b * top.a + bottom.b * (1 - top.a),
    a: 1,
  };
}

/** WCAG 2 contrast ratio; a translucent foreground is composited over the background first. */
export function contrastRatio(foreground: Rgba, background: Rgba): number {
  const fg = foreground.a < 1 ? over(foreground, background) : foreground;
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(background);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

export function contrast(foreground: string, background: string): number {
  const fg = parseColor(foreground);
  const bg = parseColor(background);
  if (!fg || !bg) return 0;
  return contrastRatio(fg, bg);
}

export const PURPLE_RANGE = { from: 270, to: 330 } as const;
/** Below this chroma a colour reads as neutral grey, whatever its nominal hue. */
export const NEUTRAL_CHROMA = 0.003;

export function isPurpleHue(hue: number): boolean {
  const h = ((hue % 360) + 360) % 360;
  return h >= PURPLE_RANGE.from && h <= PURPLE_RANGE.to;
}

export function isPurple(input: string): boolean {
  const color = toOklch(input);
  return color !== null && color.c >= NEUTRAL_CHROMA && isPurpleHue(color.h);
}

/* ---------------------------------------------------------------- formatting */

const round = (value: number, digits: number) => Number(value.toFixed(digits));

export function formatOklch({ l, c, h }: Oklch): string {
  return `oklch(${round(l, 4)} ${round(c, 4)} ${round(h, 2)})`;
}

export function toHex({ r, g, b }: Pick<Rgba, "r" | "g" | "b">): string {
  const part = (value: number) => Math.round(clamp01(value) * 255).toString(16).padStart(2, "0");
  return `#${part(r)}${part(g)}${part(b)}`;
}
