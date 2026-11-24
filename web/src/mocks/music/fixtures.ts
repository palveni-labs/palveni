/**
 * Mock music catalogue: fictional artists, albums and tracks with
 * deterministic metadata, play state, lyrics and loudness. Names and lyrics are original.
 */
import type { MusicAlbum, MusicArtist, MusicTrack } from "@/modules/music/api";
import { hashString } from "@/lib/format";
import { stableUuid } from "../fixtures";

/** Small deterministic PRNG (mulberry32). */
export function rng(seed: number) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

interface ArtistSeed {
  name: string;
  genres: string[];
  albums: Array<[name: string, year: number, cover?: string]>;
  zh: boolean;
}

const ARTISTS: ArtistSeed[] = [
  { name: "林间信号", genres: ["民谣", "独立"], zh: true, albums: [["松针与电波", 2016, "album1"], ["午后的收音机", 2018], ["雾中的站台", 2021], ["回声练习", 2024]] },
  { name: "Harbor Lights", genres: ["流行"], zh: false, albums: [["Paper Satellites", 2014, "album5"], ["Low Tide Letters", 2017], ["Neon Orchard", 2020], ["Glass Weather", 2023]] },
  { name: "北纬三十度", genres: ["摇滚"], zh: true, albums: [["长江以南", 2009, "album6"], ["失眠城市", 2013], ["铁轨尽头", 2019]] },
  { name: "The Quiet Engines", genres: ["电子"], zh: false, albums: [["Signal Bloom", 2015, "album3"], ["Night Freight", 2018], ["Soft Machines", 2022], ["Afterimage", 2025]] },
  { name: "夏夜有风", genres: ["流行", "R&B"], zh: true, albums: [["晚风邮局", 2019, "album2"], ["橘子汽水", 2021], ["慢一点也没关系", 2024]] },
  { name: "Mira Okafor", genres: ["爵士"], zh: false, albums: [["Blue Hour Sessions", 2011, "album4"], ["Velvet Corners", 2016], ["Late Train Home", 2022]] },
  { name: "灰鲸乐团", genres: ["后摇"], zh: true, albums: [["深海以北", 2012, "album7"], ["冰川移动的声音", 2017], ["没有名字的岛", 2023]] },
  { name: "Copper & Salt", genres: ["民谣"], zh: false, albums: [["Kitchen Table Hymns", 2010, "album8"], ["Wild Barley", 2015], ["The Long Field", 2020]] },
  { name: "城市漫游者", genres: ["嘻哈"], zh: true, albums: [["末班地铁", 2018], ["霓虹方言", 2022]] },
  { name: "云端弦乐团", genres: ["古典"], zh: true, albums: [["四季素描", 1998], ["夜曲集", 2005], ["山水组曲", 2014]] },
  { name: "Neon Tide", genres: ["电子", "流行"], zh: false, albums: [["Chrome Lagoon", 2019], ["Pulse Garden", 2024]] },
  { name: "白昼计划", genres: ["原声"], zh: true, albums: [["远行的人 原声带", 2020], ["归途 原声带", 2023]] },
  { name: "Ada Lindqvist", genres: ["流行", "独立"], zh: false, albums: [["Kites Over Malmö", 2012], ["Small Hours", 2016], ["Northern Static", 2021]] },
  { name: "老街唱片社", genres: ["爵士", "流行"], zh: true, albums: [["一九八七年的夏天", 1987], ["霞飞路", 1993]] },
];

const ZH_WORDS = ["晚风", "灯塔", "旧信", "雨季", "远方", "海岸", "月台", "星河", "纸飞机", "黄昏", "侧影", "回音", "白昼", "长夜", "山谷", "候鸟", "微光", "潮汐", "街角", "窗台", "秋千", "蓝调", "落日", "银杏", "雾港", "风铃"];
const ZH_TAILS = ["", "之歌", "（现场）", "练习曲", "以后", "来信", "慢板", "", "", "序曲"];
const EN_A = ["Silver", "Quiet", "Golden", "Northern", "Paper", "Electric", "Hollow", "Wild", "Slow", "Velvet", "Distant", "Midnight", "Amber", "Open", "Little", "Broken"];
const EN_B = ["Rivers", "Signals", "Harbor", "Lanterns", "Weather", "Garden", "Highway", "Letters", "Stars", "Window", "Echoes", "Morning", "Engines", "Shoreline", "Static", "Light"];

const LYRIC_ZH = [
  "把晚风折成一只纸船", "送你到看不见的对岸", "街灯一盏一盏地亮起来", "像谁在远处轻轻地喊",
  "我们走过雨后的月台", "影子比心事还要长", "如果明天还是这样晴朗", "就把旧信摊开晾在窗台",
  "听见潮水退回了海洋", "听见时间落在肩膀", "你说远方并不遥远", "只是需要多一点勇敢",
  "我把名字写在沙滩上", "等一场不急不慢的浪", "城市睡着了 只有我们醒着", "数着星星 一颗 两颗",
];
const LYRIC_EN = [
  "Fold the evening into paper wings", "Carry me across the quiet things", "Every streetlight hums a different tune",
  "We were dancing underneath the moon", "Hold the window open just a while", "Let the weather settle in your smile",
  "All the rivers running to the sea", "Every road is leading back to me", "Say it softly, say it once again",
  "Like the summer never had an end", "Static on the radio tonight", "Turn it up and drive into the light",
];

export interface MockAlbum extends MusicAlbum {
  /** Bundled cover file name, or null for a generated cover. */
  cover: string | null;
  coverSeed: number;
}

export interface MockCatalogue {
  libraryId: string;
  artists: MusicArtist[];
  albums: MockAlbum[];
  tracks: MusicTrack[];
  /** Synced or plain lyrics by track id (absent: none). */
  lyrics: Map<string, { format: "lrc" | "plain"; text: string }>;
  loudness: Map<string, number | null>;
  /** Track id → album, for the artwork endpoint. */
  albumOfTrack: Map<string, MockAlbum>;
}

function title(random: () => number, zh: boolean, used: Set<string>): string {
  for (let attempt = 0; attempt < 20; attempt++) {
    const name = zh
      ? `${ZH_WORDS[Math.floor(random() * ZH_WORDS.length)]}${ZH_TAILS[Math.floor(random() * ZH_TAILS.length)]}`
      : random() < 0.35
        ? `${EN_B[Math.floor(random() * EN_B.length)]}`
        : `${EN_A[Math.floor(random() * EN_A.length)]} ${EN_B[Math.floor(random() * EN_B.length)]}`;
    if (!used.has(name)) {
      used.add(name);
      return name;
    }
  }
  return `${zh ? "无题" : "Untitled"} ${used.size + 1}`;
}

function iso(ms: number) {
  return new Date(ms).toISOString().replace(/\.\d{3}Z$/, "Z");
}

function lrc(lines: string[], durationMs: number, random: () => number): string {
  const out: string[] = [];
  let time = 9000 + random() * 6000;
  const step = (durationMs - time - 12000) / (lines.length + 4);
  for (let index = 0; index < lines.length; index++) {
    const minutes = Math.floor(time / 60000);
    const seconds = (time % 60000) / 1000;
    out.push(`[${String(minutes).padStart(2, "0")}:${seconds.toFixed(2).padStart(5, "0")}]${lines[index]}`);
    time += step * (0.7 + random() * 0.6);
  }
  return out.join("\n");
}

const CODECS = [
  { codec: "flac", bitrate: 980, rate: 44100, depth: 16 },
  { codec: "flac", bitrate: 2400, rate: 96000, depth: 24 },
  { codec: "alac", bitrate: 1050, rate: 44100, depth: 16 },
  { codec: "mp3", bitrate: 320, rate: 44100, depth: null },
  { codec: "aac", bitrate: 256, rate: 44100, depth: null },
] as const;

export function buildCatalogue(libraryId: string, now: number): MockCatalogue {
  const artists: MusicArtist[] = [];
  const albums: MockAlbum[] = [];
  const tracks: MusicTrack[] = [];
  const lyrics = new Map<string, { format: "lrc" | "plain"; text: string }>();
  const loudness = new Map<string, number | null>();
  const albumOfTrack = new Map<string, MockAlbum>();
  const DAY = 86_400_000;

  ARTISTS.forEach((seed, artistIndex) => {
    const artistId = `ar-${hashString(`${libraryId}:${seed.name}`).toString(36)}`;
    const artistAlbums: MockAlbum[] = [];
    seed.albums.forEach(([name, year, cover], albumIndex) => {
      const random = rng(hashString(`${seed.name}/${name}`));
      const albumId = `al-${hashString(`${libraryId}:${seed.name}:${name}`).toString(36)}`;
      const count = 7 + Math.floor(random() * 7);
      const discs = count > 11 && random() < 0.3 ? 2 : 1;
      const format = CODECS[Math.floor(random() * CODECS.length)]!;
      const addedAt = now - Math.floor((artistIndex * 7 + albumIndex * 13 + random() * 30) * DAY * 2.2);
      const genres = random() < 0.25 && seed.genres.length > 1 ? seed.genres.slice(0, 1) : seed.genres;
      const used = new Set<string>();
      const albumTracks: MusicTrack[] = [];
      for (let n = 0; n < count; n++) {
        const id = stableUuid(`music-track:${libraryId}:${albumId}:${n}`);
        const durationMs = Math.round((150 + random() * 230) * 1000);
        const plays = random() < 0.35 ? 0 : Math.floor(random() ** 2 * 60);
        const disc = discs === 2 && n >= Math.ceil(count / 2) ? 2 : 1;
        const trackTitle = title(random, seed.zh, used);
        const feature = !seed.zh && random() < 0.12 ? " (feat. Ada Lindqvist)" : "";
        albumTracks.push({
          id,
          libraryId,
          title: trackTitle + feature,
          artist: seed.name,
          albumArtist: seed.name,
          album: name,
          albumId,
          artistId,
          trackNumber: disc === 2 ? n - Math.ceil(count / 2) + 1 : n + 1,
          discNumber: disc,
          year,
          genres,
          durationMs,
          artworkItemId: null,
          codec: format.codec,
          bitrateKbps: format.bitrate + Math.round((random() - 0.5) * (format.codec.startsWith("f") || format.codec === "alac" ? 200 : 0)),
          sampleRateHz: format.rate,
          bitDepth: format.depth,
          channels: 2,
          sizeBytes: Math.round((durationMs / 1000) * format.bitrate * 125),
          addedAt: iso(addedAt + n * 1000),
          state: {
            favorite: random() < 0.1,
            stars: random() < 0.3 ? 1 + Math.floor(random() * 5) : 0,
            playCount: plays,
            lastPlayedAt: plays > 0 ? iso(now - Math.floor(random() * 60 * DAY)) : null,
          },
        });
        const roll = random();
        if (roll < 0.55) {
          const pool = seed.zh ? LYRIC_ZH : LYRIC_EN;
          const start = Math.floor(random() * pool.length);
          const lines = Array.from({ length: 14 }, (_, index) => pool[(start + index) % pool.length]!);
          lyrics.set(id, roll < 0.45 ? { format: "lrc", text: lrc(lines, durationMs, random) } : { format: "plain", text: lines.join("\n") });
        }
        loudness.set(id, random() < 0.8 ? -6 - random() * 10 : null);
      }
      const artworkItemId = albumTracks[0]!.id;
      for (const track of albumTracks) track.artworkItemId = artworkItemId;
      const rated = albumTracks.filter((track) => track.state.stars > 0);
      const lastPlayed = albumTracks.map((track) => track.state.lastPlayedAt).filter((value): value is string => value !== null).sort().at(-1) ?? null;
      const album: MockAlbum = {
        id: albumId,
        libraryId,
        name,
        artist: seed.name,
        artistId,
        year,
        genres,
        trackCount: count,
        discCount: discs,
        durationMs: albumTracks.reduce((sum, track) => sum + (track.durationMs ?? 0), 0),
        artworkItemId,
        addedAt: albumTracks[0]!.addedAt,
        favorite: random() < 0.18,
        stars: rated.length ? Math.round(rated.reduce((sum, track) => sum + track.state.stars, 0) / rated.length) : 0,
        playCount: albumTracks.reduce((sum, track) => sum + track.state.playCount, 0),
        lastPlayedAt: lastPlayed,
        cover: cover ?? null,
        coverSeed: hashString(albumId),
      };
      for (const track of albumTracks) albumOfTrack.set(track.id, album);
      albums.push(album);
      artistAlbums.push(album);
      tracks.push(...albumTracks);
    });
    const latest = [...artistAlbums].sort((a, b) => (b.year ?? 0) - (a.year ?? 0))[0];
    artists.push({
      id: artistId,
      libraryId,
      name: seed.name,
      albumCount: artistAlbums.length,
      trackCount: artistAlbums.reduce((sum, album) => sum + album.trackCount, 0),
      artworkItemId: latest?.artworkItemId ?? null,
      favorite: artistIndex % 5 === 1,
      playCount: artistAlbums.reduce((sum, album) => sum + album.playCount, 0),
    });
  });

  return { libraryId, artists, albums, tracks, lyrics, loudness, albumOfTrack };
}
