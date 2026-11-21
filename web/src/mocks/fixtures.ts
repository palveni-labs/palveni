/**
 * Deterministic test fixtures. Everything is derived from fixed seeds so ids stay
 * stable across reloads (deep links into folders keep working).
 */
import type { HardwareReport, Item, ItemKind, LibraryKind } from "@/api/types";
import { hashString } from "@/lib/format";

/** mulberry32: small seeded PRNG. */
export function createRandom(seed: number) {
  let state = seed >>> 0;
  const next = () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  return {
    next,
    int: (min: number, max: number) => min + Math.floor(next() * (max - min + 1)),
    pick: <T,>(values: readonly T[]): T => values[Math.floor(next() * values.length)] as T,
  };
}

export type Random = ReturnType<typeof createRandom>;

/** Lowercase, hyphenated, v4-shaped UUID derived from a string. */
export function stableUuid(input: string): string {
  const hex = [0, 1, 2, 3]
    .map((salt) => hashString(`${salt}:${input}`).toString(16).padStart(8, "0"))
    .join("");
  const variant = ((parseInt(hex[16] ?? "0", 16) & 0x3) | 0x8).toString(16);
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-4${hex.slice(13, 16)}-${variant}${hex.slice(17, 20)}-${hex.slice(20, 32)}`;
}

/* ---------------------------------------------------------------- item trees */

interface Node {
  title: string;
  /** Leaf size in bytes; folders have children instead. */
  size?: number;
  children?: Node[];
}

const GB = 1024 ** 3;
const MB = 1024 ** 2;

const EN_ADJ = ["Silent", "Golden", "Distant", "Hidden", "Broken", "Northern", "Last", "Crimson", "Quiet", "Electric", "Paper", "Midnight", "Wild", "Hollow", "Bright", "Velvet"];
const EN_NOUN = ["Harbor", "Garden", "Signal", "River", "Orchard", "Lighthouse", "Frontier", "Archive", "Summer", "Mirror", "Station", "Horizon", "Kingdom", "Atlas", "Echo", "Voyage"];
const ZH_A = ["山海", "长夜", "春潮", "远方", "白昼", "微光", "归途", "雾港", "星河", "旧城", "风起", "云深", "落日", "晚风", "初雪", "长街"];
const ZH_B = ["之间", "来信", "往事", "笔记", "旅人", "故事", "时代", "之歌", "回声", "少年", "边境", "列车", "日记", "群像", "迷城", "人间"];
const JA = ["夏の庭", "青い街", "風の便り", "月夜の舟", "雨上がり", "遠い灯", "海辺の駅", "星降る夜"];

function enTitle(random: Random) {
  return `The ${random.pick(EN_ADJ)} ${random.pick(EN_NOUN)}`;
}
function zhTitle(random: Random) {
  return `${random.pick(ZH_A)}${random.pick(ZH_B)}`;
}

function uniqueTitles(count: number, make: () => string): string[] {
  const seen = new Set<string>();
  const titles: string[] = [];
  let guard = 0;
  while (titles.length < count && guard++ < count * 50) {
    const title = make();
    if (seen.has(title)) continue;
    seen.add(title);
    titles.push(title);
  }
  return titles;
}

function movies(random: Random, scale: number): Node[] {
  const year = () => random.int(1962, 2025);
  const film = (title: string): Node => ({ title: `${title} (${year()})`, size: random.int(1, 24) * GB + random.int(0, 900) * MB });
  const group = (title: string, count: number, make: () => string): Node => ({
    title,
    children: uniqueTitles(Math.max(1, Math.round(count * scale)), make).map(film),
  });
  return [
    group("华语电影", 48, () => zhTitle(random)),
    group("欧美电影", 64, () => enTitle(random)),
    group("日韩电影", 28, () => random.pick(JA) + (random.next() > 0.5 ? ` ${random.int(2, 3)}` : "")),
    group("动画电影", 26, () => (random.next() > 0.5 ? zhTitle(random) : enTitle(random))),
    group("纪录片", 18, () => `${random.pick(ZH_A)}纪事`),
    ...uniqueTitles(Math.max(1, Math.round(9 * scale)), () => enTitle(random)).map(film),
  ];
}

function shows(random: Random, scale: number): Node[] {
  return uniqueTitles(Math.max(1, Math.round(26 * scale)), () => (random.next() > 0.4 ? zhTitle(random) : enTitle(random))).map(
    (show) => ({
      title: show,
      children: Array.from({ length: random.int(1, 4) }, (_, season) => ({
        title: `Season ${String(season + 1).padStart(2, "0")}`,
        children: Array.from({ length: random.int(6, 13) }, (_, episode) => ({
          title: `${show} S${String(season + 1).padStart(2, "0")}E${String(episode + 1).padStart(2, "0")}`,
          size: random.int(400, 3200) * MB,
        })),
      })),
    }),
  );
}

function music(random: Random, scale: number): Node[] {
  const artists = uniqueTitles(Math.max(1, Math.round(42 * scale)), () =>
    random.next() > 0.5 ? `${random.pick(ZH_A)}乐队` : `${random.pick(EN_ADJ)} ${random.pick(EN_NOUN)}s`,
  );
  return artists.map((artist) => ({
    title: artist,
    children: uniqueTitles(random.int(1, 4), () => (random.next() > 0.5 ? zhTitle(random) : `${random.pick(EN_NOUN)} ${random.pick(["Sessions", "Tapes", "Live", "Songs", "EP"])}`)).map(
      (album) => ({
        title: album,
        children: uniqueTitles(random.int(7, 14), () => (random.next() > 0.5 ? zhTitle(random) : `${random.pick(EN_ADJ)} ${random.pick(EN_NOUN)}`)).map(
          (track, index) => ({ title: `${String(index + 1).padStart(2, "0")} ${track}`, size: random.int(4, 62) * MB }),
        ),
      }),
    ),
  }));
}

function photos(random: Random, scale: number): Node[] {
  const photo = (date: string, index: number): Node => ({
    title: `IMG_${date}_${String(index).padStart(4, "0")}`,
    size: random.int(1500, 9800) * 1024,
  });
  const years: Node[] = [];
  for (let year = 2019; year <= 2025; year++) {
    years.push({
      title: String(year),
      children: Array.from({ length: 12 }, (_, month) => {
        const ym = `${year}${String(month + 1).padStart(2, "0")}`;
        return {
          title: `${year}-${String(month + 1).padStart(2, "0")}`,
          children: Array.from({ length: Math.max(1, Math.round(random.int(20, 90) * scale)) }, (_, index) => photo(ym, index + 1)),
        };
      }),
    });
  }
  // One very large flat folder to exercise pagination and virtualisation.
  years.push({
    title: "相机胶卷",
    children: Array.from({ length: Math.max(1, Math.round(20_000 * scale)) }, (_, index) => photo("CAMERA", index + 1)),
  });
  return years;
}

function books(random: Random, scale: number): Node[] {
  const categories = ["小说", "历史", "科普", "技术", "艺术与设计", "传记", "诗歌", "English"];
  return categories.map((category) => ({
    title: category,
    children: uniqueTitles(Math.max(1, Math.round(random.int(14, 48) * scale)), () =>
      category === "English" ? enTitle(random) : `${zhTitle(random)}${random.pick(["", "（修订版）", "：全本", "简史"])}`,
    ).map((title) => ({ title, size: random.int(300, 26_000) * 1024 })),
  }));
}

function comics(random: Random, scale: number): Node[] {
  return uniqueTitles(Math.max(1, Math.round(18 * scale)), () => `${random.pick(ZH_A)}${random.pick(["物语", "战记", "少女", "食堂", "侦探", "日和"])}`).map((series) => ({
    title: series,
    children: Array.from({ length: random.int(4, 28) }, (_, volume) => ({
      title: `${series} 第${String(volume + 1).padStart(2, "0")}卷`,
      size: random.int(18, 130) * MB,
    })),
  }));
}

const GENERATORS: Record<LibraryKind, (random: Random, scale: number) => Node[]> = {
  video: movies,
  music,
  audiobook: (random, scale) => spokenFiles(random, scale, "Chapter"),
  podcast: (random, scale) => spokenFiles(random, scale, "Episode"),
  photo: photos,
  book: books,
  comic: comics,
};

function spokenFiles(random: Random, scale: number, unit: string): Node[] {
  return uniqueTitles(Math.max(1, Math.round(5 * scale)), () => enTitle(random)).map((title) => ({
    title,
    children: Array.from({ length: 3 }, (_, i) => ({ title: `${unit} ${i + 1}`, size: random.int(4, 12) * MB })),
  }));
}

const LEAF_KIND: Record<LibraryKind, ItemKind> = {
  video: "video",
  music: "track",
  audiobook: "chapter",
  podcast: "episode",
  photo: "photo",
  book: "book",
  comic: "comic",
};

const EXTENSIONS: Record<LibraryKind, string[]> = {
  video: [".mkv", ".mp4"],
  music: [".flac", ".m4a", ".mp3"],
  audiobook: [".m4b", ".mp3"],
  podcast: [".m4a", ".mp3"],
  photo: [".jpg", ".heic"],
  book: [".epub", ".pdf"],
  comic: [".cbz"],
};

/**
 * Flattens a generated tree into contract items. `variant` picks a generator flavour so two
 * video libraries (movies, series) don't look alike.
 */
export function generateItems(options: {
  libraryId: string;
  kind: LibraryKind;
  root: string;
  scale: number;
  scannedAt: number;
  variant?: "series";
}): Item[] {
  const { libraryId, kind, root, scale, scannedAt } = options;
  const random = createRandom(hashString(`${libraryId}:${options.variant ?? kind}`));
  const tree = options.variant === "series" ? shows(random, scale) : GENERATORS[kind](random, scale);
  const items: Item[] = [];
  const indexed = new Date(scannedAt).toISOString();

  const walk = (nodes: Node[], parentId: string | null, dir: string) => {
    for (const node of nodes) {
      const isFolder = Boolean(node.children);
      const path = isFolder ? `${dir}/${node.title}` : `${dir}/${node.title}${random.pick(EXTENSIONS[kind])}`;
      const id = stableUuid(`${libraryId}:${path}`);
      const modified = new Date(scannedAt - random.int(1, 1400) * 86_400_000 - random.int(0, 86_399) * 1000).toISOString();
      items.push({
        id,
        libraryId,
        parentId,
        kind: isFolder ? "folder" : LEAF_KIND[kind],
        title: node.title,
        path,
        sizeBytes: isFolder ? null : (node.size ?? 0),
        modifiedAt: modified,
        createdAt: indexed,
        updatedAt: indexed,
      });
      if (node.children) walk(node.children, id, path);
    }
  };
  walk(tree, null, root);
  return items;
}

/* ---------------------------------------------------------------- hardware */

/** A typical Intel N100 NAS with the bundled jellyfin-ffmpeg. */
export function sampleHardwareReport(probedAt: string): HardwareReport {
  return {
    perfTier: "standard",
    cpu: { model: "Intel(R) N100", cores: 4, arch: "x86_64" },
    memoryBytes: 16 * GB,
    ffmpeg: { found: true, path: "/usr/lib/jellyfin-ffmpeg/ffmpeg", version: "7.1.2-Jellyfin", source: "bundled" },
    ffprobe: { found: true, path: "/usr/lib/jellyfin-ffmpeg/ffprobe", version: "7.1.2-Jellyfin", source: "bundled" },
    hwaccels: ["vaapi", "qsv", "drm", "opencl", "vulkan"],
    encoders: [
      { id: "libx264", codec: "h264", method: "software", listed: true, verified: true, error: null },
      { id: "libx265", codec: "hevc", method: "software", listed: true, verified: true, error: null },
      { id: "libsvtav1", codec: "av1", method: "software", listed: true, verified: true, error: null },
      { id: "h264_vaapi", codec: "h264", method: "vaapi", listed: true, verified: true, error: null },
      { id: "hevc_vaapi", codec: "hevc", method: "vaapi", listed: true, verified: true, error: null },
      {
        id: "av1_vaapi",
        codec: "av1",
        method: "vaapi",
        listed: true,
        verified: false,
        error: "No usable encoding entrypoint found for profile VAProfileAV1Profile0 (Function not implemented)",
      },
      { id: "h264_qsv", codec: "h264", method: "qsv", listed: true, verified: true, error: null },
      { id: "hevc_qsv", codec: "hevc", method: "qsv", listed: true, verified: true, error: null },
      {
        id: "av1_qsv",
        codec: "av1",
        method: "qsv",
        listed: true,
        verified: false,
        error: "Error initializing the encoder: unsupported (-3)",
      },
      { id: "h264_nvenc", codec: "h264", method: "nvenc", listed: true, verified: false, error: "Cannot load libcuda.so.1" },
      { id: "hevc_nvenc", codec: "hevc", method: "nvenc", listed: true, verified: false, error: "Cannot load libcuda.so.1" },
      { id: "h264_amf", codec: "h264", method: "amf", listed: true, verified: false, error: "DLL libamfrt64.so.1 failed to open" },
      { id: "h264_rkmpp", codec: "h264", method: "rkmpp", listed: false, verified: false, error: null },
      { id: "hevc_rkmpp", codec: "hevc", method: "rkmpp", listed: false, verified: false, error: null },
      { id: "h264_v4l2m2m", codec: "h264", method: "v4l2m2m", listed: true, verified: false, error: "Could not find a valid device" },
    ],
    probedAt,
  };
}
