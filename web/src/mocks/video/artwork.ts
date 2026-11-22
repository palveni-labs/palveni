/**
 * Procedural SVG artwork for mock video posters (2:3), backdrops (16:9), episode thumbs (16:9)
 * and cast portraits. Self-contained, deterministic and fast — zero external network requests.
 */
import artData from "./art.json";

export interface ArtWorkSpec {
  id: string;
  title: string;
  original: string;
  year: number;
  scene: string;
  palette: [string, string, string, string, string, string];
  seed: number;
}

export interface ArtPersonSpec {
  id: string;
  bg: string;
  skin: string;
  hair: string;
  style: string;
}

export const ART_WORKS = artData.works as unknown as ArtWorkSpec[];
export const ART_PEOPLE = artData.people as unknown as ArtPersonSpec[];

function esc(text: string): string {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function prng(seed: number) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function renderSceneLayer(spec: ArtWorkSpec, w: number, h: number, variant = 0): string {
  const [c0, c1, c2, c3, c4, c5] = spec.palette;
  const rand = prng(spec.seed * 97 + variant * 313);
  const shift = variant === 0 ? 0 : (rand() - 0.5) * w * 0.18;
  const horizonY = Math.round(h * (0.56 + (variant ? (rand() - 0.5) * 0.12 : 0)));
  const uid = `${spec.id}-${w}x${h}-${variant}`;

  const sky = `
    <defs>
      <linearGradient id="sky-${uid}" x1="0" y1="0" x2="0.18" y2="1">
        <stop offset="0%" stop-color="${c0}"/>
        <stop offset="56%" stop-color="${c1}"/>
        <stop offset="100%" stop-color="${c4}"/>
      </linearGradient>
      <radialGradient id="glow-${uid}" cx="${(0.58 + shift / w).toFixed(2)}" cy="${(horizonY / h - 0.08).toFixed(2)}" r="0.58">
        <stop offset="0%" stop-color="${c2}" stop-opacity="0.72"/>
        <stop offset="48%" stop-color="${c2}" stop-opacity="0.18"/>
        <stop offset="100%" stop-color="${c2}" stop-opacity="0"/>
      </radialGradient>
      <linearGradient id="ground-${uid}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="${c3}"/>
        <stop offset="55%" stop-color="${c4}"/>
        <stop offset="100%" stop-color="${c5}"/>
      </linearGradient>
    </defs>
    <rect width="${w}" height="${h}" fill="url(#sky-${uid})"/>
    <rect width="${w}" height="${h}" fill="url(#glow-${uid})"/>
  `;

  switch (spec.scene) {
    case "sea":
    case "harbour": {
      const sunX = Math.round(w * 0.54 + shift);
      const sunR = Math.round(Math.min(w, h) * 0.16);
      const waves = Array.from({ length: 6 }, (_, i) => {
        const y = horizonY + Math.round(((h - horizonY) / 6) * (i + 0.4));
        const amp = 6 + i * 2;
        const fill = i % 2 === 0 ? c4 : c5;
        return `<path d="M0 ${y} Q${Math.round(w * 0.25)} ${y - amp} ${Math.round(w * 0.5)} ${y} T${w} ${y} V${h} H0 Z" fill="${fill}" opacity="${(0.45 + i * 0.09).toFixed(2)}"/>`;
      }).join("");
      const crane = spec.scene === "harbour"
        ? `<g fill="${c5}" opacity="0.92">
             <rect x="${Math.round(w * 0.08)}" y="${horizonY - Math.round(h * 0.18)}" width="${Math.round(w * 0.018)}" height="${Math.round(h * 0.19)}"/>
             <path d="M${Math.round(w * 0.06)} ${horizonY - Math.round(h * 0.17)} L${Math.round(w * 0.28)} ${horizonY - Math.round(h * 0.21)} L${Math.round(w * 0.28)} ${horizonY - Math.round(h * 0.195)} L${Math.round(w * 0.06)} ${horizonY - Math.round(h * 0.155)} Z"/>
             <rect x="${Math.round(w * 0.66)}" y="${horizonY - Math.round(h * 0.12)}" width="${Math.round(w * 0.22)}" height="${Math.round(h * 0.12)}" rx="2"/>
           </g>`
        : "";
      return `${sky}
        <circle cx="${sunX}" cy="${horizonY}" r="${sunR}" fill="${c2}" opacity="0.9"/>
        <rect y="${horizonY}" width="${w}" height="${h - horizonY}" fill="url(#ground-${uid})"/>
        <line x1="0" y1="${horizonY}" x2="${w}" y2="${horizonY}" stroke="${c2}" stroke-width="2" opacity="0.65"/>
        <ellipse cx="${sunX}" cy="${horizonY + Math.round(h * 0.09)}" rx="${Math.round(sunR * 1.15)}" ry="${Math.round(sunR * 0.14)}" fill="${c2}" opacity="0.28"/>
        ${crane}
        ${waves}`;
    }

    case "city":
    case "rain": {
      const buildings: string[] = [];
      const windows: string[] = [];
      let x = -10;
      while (x < w + 20) {
        const bw = Math.round(w * (0.065 + rand() * 0.075));
        const bh = Math.round(h * (0.18 + rand() * 0.36));
        const by = h - bh;
        buildings.push(`<rect x="${x}" y="${by}" width="${bw}" height="${bh}" fill="${rand() > 0.45 ? c5 : c4}"/>`);
        const cols = Math.max(1, Math.floor((bw - 10) / 11));
        const rows = Math.max(2, Math.floor((bh - 18) / 16));
        for (let r = 0; r < rows; r++) {
          for (let col = 0; col < cols; col++) {
            if (rand() > 0.68) {
              windows.push(`<rect x="${x + 6 + col * 11}" y="${by + 10 + r * 16}" width="5" height="7" fill="${c2}" opacity="${(0.35 + rand() * 0.55).toFixed(2)}"/>`);
            }
          }
        }
        x += bw - 4;
      }
      const rain = spec.scene === "rain"
        ? Array.from({ length: 42 }, () => {
            const rx = Math.round(rand() * w);
            const ry = Math.round(rand() * h);
            const len = Math.round(24 + rand() * 52);
            return `<line x1="${rx}" y1="${ry}" x2="${rx - 6}" y2="${ry + len}" stroke="${c2}" stroke-width="1.2" opacity="${(0.14 + rand() * 0.28).toFixed(2)}"/>`;
          }).join("")
        : "";
      return `${sky}
        <circle cx="${Math.round(w * 0.72 + shift)}" cy="${Math.round(h * 0.28)}" r="${Math.round(Math.min(w, h) * 0.11)}" fill="${c2}" opacity="0.72"/>
        ${buildings.join("")}
        ${windows.join("")}
        ${rain}`;
    }

    case "mountains":
    case "snow": {
      const layer = (baseY: number, peaks: number, amp: number, fill: string, opacity = 1) => {
        const pts: string[] = [`M0 ${h}`, `L0 ${baseY}`];
        for (let i = 1; i <= peaks; i++) {
          const px = Math.round((w / peaks) * i);
          const py = Math.round(baseY - rand() * amp);
          const mx = Math.round((w / peaks) * (i - 0.5));
          const my = Math.round(baseY - (0.35 + rand() * 0.65) * amp);
          pts.push(`L${mx} ${my}`, `L${px} ${py}`);
        }
        pts.push(`L${w} ${h} Z`);
        return `<path d="${pts.join(" ")}" fill="${fill}" opacity="${opacity}"/>`;
      };
      const flakes = spec.scene === "snow"
        ? Array.from({ length: 34 }, () => `<circle cx="${Math.round(rand() * w)}" cy="${Math.round(rand() * h)}" r="${(1 + rand() * 2.2).toFixed(1)}" fill="${c2}" opacity="${(0.25 + rand() * 0.5).toFixed(2)}"/>`).join("")
        : "";
      return `${sky}
        <circle cx="${Math.round(w * 0.34 + shift)}" cy="${Math.round(h * 0.29)}" r="${Math.round(Math.min(w, h) * 0.11)}" fill="${c2}" opacity="0.82"/>
        ${layer(Math.round(h * 0.64), 4, h * 0.22, c3, 0.72)}
        ${layer(Math.round(h * 0.74), 5, h * 0.2, c4, 0.9)}
        ${layer(Math.round(h * 0.85), 3, h * 0.15, c5, 1)}
        ${flakes}`;
    }

    case "desert": {
      const sunX = Math.round(w * 0.65 + shift);
      return `${sky}
        <circle cx="${sunX}" cy="${Math.round(h * 0.36)}" r="${Math.round(Math.min(w, h) * 0.16)}" fill="${c2}" opacity="0.88"/>
        <path d="M0 ${Math.round(h * 0.66)} Q${Math.round(w * 0.28)} ${Math.round(h * 0.52)} ${Math.round(w * 0.58)} ${Math.round(h * 0.64)} T${w} ${Math.round(h * 0.58)} V${h} H0 Z" fill="${c3}"/>
        <path d="M0 ${Math.round(h * 0.76)} Q${Math.round(w * 0.35)} ${Math.round(h * 0.66)} ${Math.round(w * 0.72)} ${Math.round(h * 0.78)} T${w} ${Math.round(h * 0.73)} V${h} H0 Z" fill="${c4}"/>
        <path d="M0 ${Math.round(h * 0.88)} Q${Math.round(w * 0.45)} ${Math.round(h * 0.8)} ${w} ${Math.round(h * 0.89)} V${h} H0 Z" fill="${c5}"/>`;
    }

    case "space":
    case "aurora": {
      const stars = Array.from({ length: 60 }, () => {
        const sx = Math.round(rand() * w);
        const sy = Math.round(rand() * h * 0.82);
        const sr = (0.7 + rand() * 1.9).toFixed(1);
        return `<circle cx="${sx}" cy="${sy}" r="${sr}" fill="#ffffff" opacity="${(0.25 + rand() * 0.7).toFixed(2)}"/>`;
      }).join("");
      const ribbon = spec.scene === "aurora"
        ? `<path d="M0 ${Math.round(h * 0.48)} C${Math.round(w * 0.22)} ${Math.round(h * 0.24)} ${Math.round(w * 0.52)} ${Math.round(h * 0.6)} ${w} ${Math.round(h * 0.26)} V${Math.round(h * 0.52)} C${Math.round(w * 0.56)} ${Math.round(h * 0.74)} ${Math.round(w * 0.24)} ${Math.round(h * 0.42)} 0 ${Math.round(h * 0.66)} Z" fill="${c2}" opacity="0.34"/>
           <path d="M0 ${Math.round(h * 0.44)} C${Math.round(w * 0.3)} ${Math.round(h * 0.18)} ${Math.round(w * 0.62)} ${Math.round(h * 0.5)} ${w} ${Math.round(h * 0.22)}" fill="none" stroke="${c2}" stroke-width="5" opacity="0.65"/>`
        : `<circle cx="${Math.round(w * 0.62 + shift)}" cy="${Math.round(h * 0.42)}" r="${Math.round(Math.min(w, h) * 0.18)}" fill="${c3}"/>
           <circle cx="${Math.round(w * 0.62 + shift - Math.min(w, h) * 0.05)}" cy="${Math.round(h * 0.42 - Math.min(w, h) * 0.04)}" r="${Math.round(Math.min(w, h) * 0.15)}" fill="${c2}" opacity="0.35"/>
           <ellipse cx="${Math.round(w * 0.62 + shift)}" cy="${Math.round(h * 0.42)}" rx="${Math.round(Math.min(w, h) * 0.32)}" ry="${Math.round(Math.min(w, h) * 0.06)}" fill="none" stroke="${c2}" stroke-width="3" opacity="0.65" transform="rotate(-14 ${Math.round(w * 0.62 + shift)} ${Math.round(h * 0.42)})"/>`;
      return `${sky}
        ${stars}
        ${ribbon}
        <path d="M0 ${Math.round(h * 0.84)} Q${Math.round(w * 0.42)} ${Math.round(h * 0.75)} ${w} ${Math.round(h * 0.86)} V${h} H0 Z" fill="${c5}"/>`;
    }

    case "lighthouse": {
      const tx = Math.round(w * 0.64 + shift * 0.5);
      const base = Math.round(h * 0.72);
      const top = Math.round(h * 0.34);
      return `${sky}
        <polygon points="${tx},${top + 10} 0,${top - Math.round(h * 0.12)} 0,${top + Math.round(h * 0.14)}" fill="${c2}" opacity="0.28"/>
        <polygon points="${tx},${top + 10} ${w},${top - Math.round(h * 0.08)} ${w},${top + Math.round(h * 0.16)}" fill="${c2}" opacity="0.22"/>
        <rect y="${base}" width="${w}" height="${h - base}" fill="${c4}"/>
        <path d="M${tx - 16} ${base} L${tx - 9} ${top} L${tx + 9} ${top} L${tx + 16} ${base} Z" fill="${c5}"/>
        <circle cx="${tx}" cy="${top + 8}" r="11" fill="${c2}"/>
        <path d="M0 ${Math.round(h * 0.8)} Q${Math.round(w * 0.35)} ${Math.round(h * 0.74)} ${w} ${Math.round(h * 0.82)} V${h} H0 Z" fill="${c5}"/>`;
    }

    case "lanterns": {
      const lanterns = Array.from({ length: 14 }, () => {
        const lx = Math.round(w * (0.1 + rand() * 0.8));
        const ly = Math.round(h * (0.16 + rand() * 0.62));
        const r = Math.round(8 + rand() * 18);
        return `<g>
          <circle cx="${lx}" cy="${ly}" r="${r * 2.1}" fill="${c2}" opacity="0.16"/>
          <rect x="${lx - r * 0.7}" y="${ly - r}" width="${r * 1.4}" height="${r * 1.8}" rx="${r * 0.45}" fill="${c2}" opacity="0.88"/>
        </g>`;
      }).join("");
      return `${sky}${lanterns}<rect y="${Math.round(h * 0.82)}" width="${w}" height="${Math.round(h * 0.18)}" fill="${c5}" opacity="0.88"/>`;
    }

    case "rail": {
      const vx = Math.round(w * 0.5 + shift * 0.4);
      const vy = Math.round(h * 0.52);
      const ties = Array.from({ length: 9 }, (_, i) => {
        const t = (i + 1) / 10;
        const y = Math.round(vy + (h - vy) * t * t);
        const span = Math.round(w * 0.05 + w * 0.36 * t * t);
        return `<line x1="${vx - span}" y1="${y}" x2="${vx + span}" y2="${y}" stroke="${c5}" stroke-width="${(2 + t * 5).toFixed(1)}" opacity="0.85"/>`;
      }).join("");
      return `${sky}
        <circle cx="${vx}" cy="${vy - Math.round(h * 0.06)}" r="${Math.round(Math.min(w, h) * 0.14)}" fill="${c2}" opacity="0.85"/>
        <rect y="${vy}" width="${w}" height="${h - vy}" fill="url(#ground-${uid})"/>
        ${ties}
        <line x1="${vx}" y1="${vy}" x2="${Math.round(w * 0.16)}" y2="${h}" stroke="${c2}" stroke-width="3" opacity="0.85"/>
        <line x1="${vx}" y1="${vy}" x2="${Math.round(w * 0.84)}" y2="${h}" stroke="${c2}" stroke-width="3" opacity="0.85"/>`;
    }

    case "forest":
    case "fields": {
      const trees = Array.from({ length: 16 }, (_, i) => {
        const tx = Math.round((w / 15) * i + (rand() - 0.5) * 24);
        const th = Math.round(h * (0.24 + rand() * 0.3));
        const tw = Math.round(w * (0.04 + rand() * 0.04));
        return `<polygon points="${tx},${h - th} ${tx - tw},${h} ${tx + tw},${h}" fill="${i % 2 === 0 ? c4 : c5}" opacity="0.92"/>`;
      }).join("");
      return `${sky}
        <circle cx="${Math.round(w * 0.7 + shift)}" cy="${Math.round(h * 0.28)}" r="${Math.round(Math.min(w, h) * 0.13)}" fill="${c2}" opacity="0.85"/>
        <path d="M0 ${Math.round(h * 0.72)} Q${Math.round(w * 0.4)} ${Math.round(h * 0.6)} ${w} ${Math.round(h * 0.74)} V${h} H0 Z" fill="${c3}"/>
        ${spec.scene === "forest" ? trees : `<path d="M0 ${Math.round(h * 0.82)} Q${Math.round(w * 0.6)} ${Math.round(h * 0.74)} ${w} ${Math.round(h * 0.84)} V${h} H0 Z" fill="${c4}"/>`}`;
    }

    case "table": {
      return `${sky}
        <rect y="${Math.round(h * 0.55)}" width="${w}" height="${Math.round(h * 0.45)}" fill="${c1}"/>
        <circle cx="${Math.round(w * 0.5 + shift * 0.4)}" cy="${Math.round(h * 0.62)}" r="${Math.round(Math.min(w, h) * 0.24)}" fill="${c0}" stroke="${c5}" stroke-width="4" opacity="0.95"/>
        <circle cx="${Math.round(w * 0.5 + shift * 0.4)}" cy="${Math.round(h * 0.62)}" r="${Math.round(Math.min(w, h) * 0.16)}" fill="${c2}" opacity="0.88"/>
        <circle cx="${Math.round(w * 0.24)}" cy="${Math.round(h * 0.74)}" r="${Math.round(Math.min(w, h) * 0.08)}" fill="${c3}" opacity="0.9"/>
        <circle cx="${Math.round(w * 0.78)}" cy="${Math.round(h * 0.72)}" r="${Math.round(Math.min(w, h) * 0.06)}" fill="${c4}" opacity="0.92"/>`;
    }

    default: {
      // "town" and any other fallback
      const roofs = Array.from({ length: 10 }, (_, i) => {
        const bx = Math.round((w / 9) * i - 10);
        const bw = Math.round(w * 0.14);
        const bh = Math.round(h * (0.2 + rand() * 0.22));
        const by = h - bh;
        return `<g fill="${i % 2 === 0 ? c4 : c5}">
          <rect x="${bx}" y="${by}" width="${bw}" height="${bh}"/>
          <polygon points="${bx - 6},${by} ${bx + bw / 2},${by - 28} ${bx + bw + 6},${by}"/>
          <rect x="${bx + Math.round(bw * 0.34)}" y="${by + 20}" width="10" height="14" fill="${c2}" opacity="0.82"/>
        </g>`;
      }).join("");
      return `${sky}
        <circle cx="${Math.round(w * 0.5 + shift)}" cy="${Math.round(h * 0.32)}" r="${Math.round(Math.min(w, h) * 0.15)}" fill="${c2}" opacity="0.82"/>
        ${roofs}`;
    }
  }
}

/** 2:3 movie/series/season poster SVG with crisp title typography. */
export function renderPosterSvg(spec: ArtWorkSpec, seasonNumber?: number): string {
  const w = 400;
  const h = 600;
  const lightGround = spec.scene === "table";
  const textPrimary = lightGround ? "#221912" : "#ffffff";
  const textSub = lightGround ? "#554231" : "rgba(255,255,255,0.78)";
  const scrimTop = lightGround ? "rgba(233,220,196,0.55)" : "rgba(0,0,0,0.42)";
  const scrimBottom = lightGround ? "rgba(233,220,196,0.88)" : "rgba(0,0,0,0.84)";
  const subline = seasonNumber != null
    ? `${seasonNumber === 0 ? "SPECIALS" : `SEASON ${seasonNumber}`} · ${spec.original.toUpperCase()}`
    : `${spec.year} · ${spec.original.toUpperCase()}`;

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" font-family="'PingFang SC', 'Noto Sans CJK SC', 'Noto Sans SC', Figtree, sans-serif">
    ${renderSceneLayer(spec, w, h, seasonNumber ?? 0)}
    <defs>
      <linearGradient id="poster-scrim-${spec.id}-${seasonNumber ?? 0}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="${scrimTop}"/>
        <stop offset="28%" stop-color="rgba(0,0,0,0)"/>
        <stop offset="62%" stop-color="rgba(0,0,0,0.12)"/>
        <stop offset="100%" stop-color="${scrimBottom}"/>
      </linearGradient>
    </defs>
    <rect width="${w}" height="${h}" fill="url(#poster-scrim-${spec.id}-${seasonNumber ?? 0})"/>
    <text x="28" y="514" font-size="38" font-weight="700" fill="${textPrimary}">${esc(spec.title)}</text>
    <text x="28" y="548" font-size="14" font-weight="500" letter-spacing="1.4" fill="${textSub}">${esc(subline)}</text>
  </svg>`;
}

/** 16:9 full-bleed backdrop SVG without embedded text (hero & detail pages overlay real text). */
export function renderBackdropSvg(spec: ArtWorkSpec): string {
  const w = 1280;
  const h = 720;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}">
    ${renderSceneLayer(spec, w, h, 0)}
    <rect width="${w}" height="${h}" fill="rgba(0,0,0,0.1)"/>
  </svg>`;
}

/** 16:9 episode thumbnail SVG with subtle per-episode lighting and framing variation. */
export function renderThumbSvg(spec: ArtWorkSpec, episodeVariant: number): string {
  const w = 640;
  const h = 360;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}">
    ${renderSceneLayer(spec, w, h, Math.max(1, episodeVariant))}
  </svg>`;
}

/** Cast portrait SVG (studio head-and-shoulders illustration). */
export function renderPersonSvg(person: ArtPersonSpec): string {
  const w = 300;
  const h = 360;
  let hairBack = "";
  let hairFront: string;
  if (person.style === "long") {
    hairBack = `<path d="M88 122 Q82 220 96 270 H204 Q218 220 212 122 Q200 68 150 68 Q100 68 88 122 Z" fill="${person.hair}"/>`;
    hairFront = `<path d="M98 128 Q112 88 154 88 Q192 88 202 128 Q186 104 150 104 Q116 104 98 128 Z" fill="${person.hair}"/>`;
  } else if (person.style === "bob") {
    hairBack = `<path d="M92 128 Q90 198 106 218 H194 Q210 198 208 128 Q196 72 150 72 Q104 72 92 128 Z" fill="${person.hair}"/>`;
    hairFront = `<path d="M100 124 Q124 94 156 96 Q188 98 200 124 Q182 106 150 106 Q120 106 100 124 Z" fill="${person.hair}"/>`;
  } else if (person.style === "bun") {
    hairBack = `<circle cx="150" cy="64" r="24" fill="${person.hair}"/>`;
    hairFront = `<path d="M100 126 Q108 78 150 78 Q192 78 200 126 Q180 100 150 100 Q120 100 100 126 Z" fill="${person.hair}"/>`;
  } else {
    hairFront = `<path d="M100 126 Q104 76 150 76 Q196 76 200 126 Q182 102 150 100 Q118 102 100 126 Z" fill="${person.hair}"/>`;
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}">
    <defs>
      <radialGradient id="pbg-${person.id}" cx="0.5" cy="0.32" r="0.72">
        <stop offset="0%" stop-color="${person.bg}"/>
        <stop offset="100%" stop-color="#14181e"/>
      </radialGradient>
    </defs>
    <rect width="${w}" height="${h}" fill="url(#pbg-${person.id})"/>
    ${hairBack}
    <path d="M54 360 C60 286 102 252 150 252 C198 252 240 286 246 360 Z" fill="#1e242c"/>
    <rect x="132" y="206" width="36" height="54" rx="16" fill="${person.skin}"/>
    <ellipse cx="150" cy="152" rx="48" ry="60" fill="${person.skin}"/>
    ${hairFront}
    <circle cx="132" cy="152" r="3.4" fill="#1b1614" opacity="0.82"/>
    <circle cx="168" cy="152" r="3.4" fill="#1b1614" opacity="0.82"/>
    <path d="M138 182 Q150 190 162 182" fill="none" stroke="#8a5a44" stroke-width="2.5" stroke-linecap="round" opacity="0.75"/>
  </svg>`;
}
