/**
 * Generated media for the mock: original abstract album covers (SVG drawn from a seed, rasterised
 * to PNG in the page) and short synthesised audio (WAV) so the player can really play.
 */
import { rng } from "./fixtures";

/** Field colours: warm, green, blue and neutral families (no purple, as the official skins). */
const FIELDS = ["#7d2b22", "#b2482a", "#a8581d", "#d8b04a", "#5f7f3a", "#24584f", "#0f3a41", "#2e4f86", "#1c3e6c", "#1d1d1f", "#e6e0d2", "#3f4a3c", "#8a3b2e", "#c46a3c", "#2f6a73", "#e9c3a8"];
const INKS = ["#f3dccb", "#e9c3a8", "#f2d3a0", "#d9e2ee", "#e6e0d2", "#33270a", "#163a34", "#141414", "#f6efe3", "#e7b39b"];

function pick<T>(random: () => number, list: readonly T[]): T {
  return list[Math.floor(random() * list.length)]!;
}

export function coverSvg(seed: number, label: string): string {
  const random = rng(seed);
  const field = pick(random, FIELDS);
  let ink = pick(random, INKS);
  if (ink === field) ink = "#f6efe3";
  const accent = pick(random, FIELDS.filter((color) => color !== field));
  const style = Math.floor(random() * 6);
  const parts: string[] = [`<rect width="200" height="200" fill="${field}"/>`];
  if (style === 0) {
    parts.push(`<circle cx="${60 + random() * 80}" cy="${60 + random() * 70}" r="${36 + random() * 30}" fill="${ink}"/>`);
    parts.push(`<rect y="${140 + random() * 20}" width="200" height="80" fill="${accent}" opacity="0.85"/>`);
  } else if (style === 1) {
    const bands = 5 + Math.floor(random() * 6);
    for (let index = 0; index < bands; index++) {
      parts.push(`<rect y="${(index * 200) / bands}" width="200" height="${100 / bands}" fill="${index % 2 ? ink : accent}" opacity="${0.35 + random() * 0.6}"/>`);
    }
  } else if (style === 2) {
    for (let ring = 6; ring > 0; ring--) {
      parts.push(`<circle cx="100" cy="100" r="${ring * 15}" fill="none" stroke="${ring % 2 ? ink : accent}" stroke-width="${4 + random() * 6}"/>`);
    }
  } else if (style === 3) {
    const cells = 4;
    for (let x = 0; x < cells; x++) {
      for (let y = 0; y < cells; y++) {
        if (random() < 0.45) parts.push(`<rect x="${x * 50 + 6}" y="${y * 50 + 6}" width="38" height="38" rx="${random() < 0.5 ? 19 : 2}" fill="${random() < 0.5 ? ink : accent}"/>`);
      }
    }
  } else if (style === 4) {
    parts.push(`<path d="M0 ${120 + random() * 30} Q 60 ${80 + random() * 40} 110 ${120 + random() * 20} T 200 ${110 + random() * 30} V200 H0Z" fill="${accent}"/>`);
    parts.push(`<path d="M0 ${150 + random() * 20} Q 80 ${130 + random() * 30} 140 ${160 + random() * 10} T 200 ${150 + random() * 20} V200 H0Z" fill="${ink}" opacity="0.9"/>`);
    parts.push(`<circle cx="${40 + random() * 120}" cy="${50 + random() * 30}" r="${14 + random() * 12}" fill="${ink}"/>`);
  } else {
    const angle = Math.floor(random() * 4) * 45;
    parts.push(`<g transform="rotate(${angle} 100 100)">${Array.from({ length: 7 }, (_, index) => `<rect x="-60" y="${-40 + index * 46}" width="320" height="${10 + random() * 16}" fill="${index % 2 ? ink : accent}"/>`).join("")}</g>`);
  }
  if (random() < 0.55) {
    const text = label.replace(/[<>&"]/g, "").slice(0, 22);
    const dark = parseInt(field.slice(1, 3), 16) + parseInt(field.slice(3, 5), 16) + parseInt(field.slice(5, 7), 16) < 380;
    parts.push(`<text x="14" y="${random() < 0.5 ? 26 : 186}" font-family="Figtree, 'Noto Sans SC', 'PingFang SC', sans-serif" font-size="12" font-weight="700" letter-spacing="1.2" fill="${dark ? "#f6efe3" : "#1d1d1f"}">${text.toUpperCase()}</text>`);
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">${parts.join("")}</svg>`;
}

const rasterCache = new Map<string, Promise<ArrayBuffer | null>>();

/** PNG bytes of an SVG at `size` px, rasterised with the page's canvas (null where there is none). */
export function rasterise(key: string, svg: string, size: number): Promise<ArrayBuffer | null> {
  const cacheKey = `${key}@${size}`;
  const cached = rasterCache.get(cacheKey);
  if (cached) return cached;
  const job = (async () => {
    if (typeof document === "undefined" || typeof Image === "undefined") return null;
    const image = new Image();
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
    try {
      await image.decode();
    } catch {
      return null;
    }
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = size;
    const context = canvas.getContext("2d");
    if (!context) return null;
    context.drawImage(image, 0, 0, size, size);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
    return blob ? blob.arrayBuffer() : null;
  })();
  rasterCache.set(cacheKey, job);
  return job;
}

const SAMPLE_RATE = 8000;
const NOTES = [0, 4, 7, 11, 14, 11, 7, 4];

/**
 * A gentle arpeggio (8 kHz, 8-bit mono WAV) lasting `durationMs`, starting `startMs` in. Each track
 * gets its own key so skipping is audible.
 */
export function synthesise(seed: number, durationMs: number, startMs = 0): Uint8Array {
  const random = rng(seed);
  const root = 196 * 2 ** (Math.floor(random() * 12) / 12);
  const beat = 0.32 + random() * 0.18;
  const first = Math.floor((startMs / 1000) * SAMPLE_RATE);
  const total = Math.max(0, Math.floor((durationMs / 1000) * SAMPLE_RATE) - first);
  const bytes = new Uint8Array(44 + total);
  const view = new DataView(bytes.buffer);
  const text = (offset: number, value: string) => [...value].forEach((char, index) => view.setUint8(offset + index, char.charCodeAt(0)));
  text(0, "RIFF");
  view.setUint32(4, 36 + total, true);
  text(8, "WAVE");
  text(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, SAMPLE_RATE, true);
  view.setUint32(28, SAMPLE_RATE, true);
  view.setUint16(32, 1, true);
  view.setUint16(34, 8, true);
  text(36, "data");
  view.setUint32(40, total, true);
  const step = (2 * Math.PI) / SAMPLE_RATE;
  for (let index = 0; index < total; index++) {
    const t = (first + index) / SAMPLE_RATE;
    const noteIndex = Math.floor(t / beat);
    const local = t - noteIndex * beat;
    const freq = root * 2 ** (NOTES[noteIndex % NOTES.length]! / 12);
    const envelope = Math.exp(-local * 5) * Math.min(1, local * 80);
    const pad = 0.18 * Math.sin(step * (first + index) * root * 0.5);
    const value = envelope * (0.55 * Math.sin(2 * Math.PI * freq * t) + 0.18 * Math.sin(4 * Math.PI * freq * t)) + pad;
    bytes[44 + index] = Math.max(0, Math.min(255, Math.round(128 + value * 70)));
  }
  return bytes;
}
