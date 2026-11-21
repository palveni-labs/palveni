import { describe, expect, it } from "vitest";
import workcolor from "./builtin/workcolor";
import { contrast, isPurpleHue, oklchToSrgb } from "./color";
import { parseSkin } from "./schema";
import { extractWorkHue, NEUTRAL, renderWorkColor } from "./work-color";

/** An RGBA buffer: `share` of the pixels in colour A, the rest in colour B. */
function pixels(a: [number, number, number], b: [number, number, number], share: number, total = 400): Uint8ClampedArray {
  const data = new Uint8ClampedArray(total * 4);
  for (let index = 0; index < total; index++) {
    const [r, g, bl] = index < total * share ? a : b;
    data.set([r, g, bl, 255], index * 4);
  }
  return data;
}

const rgb = (l: number, c: number, h: number): [number, number, number] => {
  const { r, g, b } = oklchToSrgb({ l, c, h });
  return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
};

const MAROON = rgb(0.34, 0.11, 18);
const TEAL = rgb(0.45, 0.07, 205);
const PURPLE = rgb(0.45, 0.16, 300);
const GREY = rgb(0.5, 0, 0);
const skin = parseSkin(workcolor.manifest, "builtin");

describe("work colour", () => {
  it("takes the cover's main colour, the same every time", () => {
    const cover = pixels(MAROON, TEAL, 0.7);
    const hue = extractWorkHue(cover);
    expect(hue.h).toBeGreaterThan(5);
    expect(hue.h).toBeLessThan(35);
    expect(extractWorkHue(cover)).toEqual(hue);
  });

  it("skips the purple range and uses the cover's next colour", () => {
    const hue = extractWorkHue(pixels(PURPLE, TEAL, 0.7));
    expect(isPurpleHue(hue.h)).toBe(false);
    expect(hue.h).toBeGreaterThan(190);
    expect(hue.h).toBeLessThan(220);
  });

  it("falls back to neutral grey for a monochrome or all-purple cover", () => {
    expect(extractWorkHue(pixels(GREY, rgb(0.15, 0, 0), 0.5))).toEqual(NEUTRAL);
    expect(extractWorkHue(pixels(PURPLE, GREY, 0.9))).toEqual(NEUTRAL);
  });

  it("is adjusted per mode so the text on it stays readable", () => {
    for (const mode of ["light", "dark"] as const) {
      const tokens = skin.colors[mode];
      for (let h = 0; h < 360; h += 15) {
        if (isPurpleHue(h)) continue;
        for (const c of [0, 0.06, 0.12, 0.16]) {
          const colors = renderWorkColor({ h, c }, skin.work[mode], { fg: tokens.fg, track: tokens["progress-rest"], mode });
          expect(contrast(colors.fieldFg, colors.field), `${mode} field h${h} c${c}`).toBeGreaterThanOrEqual(4.5);
          expect(contrast(colors.fieldSub, colors.field), `${mode} sub h${h} c${c}`).toBeGreaterThanOrEqual(4.5);
          expect(contrast(tokens.fg, colors.wash), `${mode} wash h${h} c${c}`).toBeGreaterThanOrEqual(4.5);
          expect(contrast(colors.bar, tokens["progress-rest"]), `${mode} bar h${h} c${c}`).toBeGreaterThanOrEqual(3);
        }
      }
    }
  });
});
