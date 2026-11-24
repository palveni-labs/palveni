import type {
  CreatePhotoAlbumRequest,
  CreatePhotoPersonRequest,
  PhotoAlbumSummary,
  PhotoAnalyzeResponse,
  PhotoCameraBucket,
  PhotoDuplicateCluster,
  PhotoDuplicatesResponse,
  PhotoInfo,
  PhotoMemories,
  PhotoMonthBucket,
  PhotoPersonRef,
  PhotoPersonSummary,
  PhotoPlaceCluster,
  PhotoQueryFilter,
  PhotoSearchHit,
  PhotoSearchResponse,
  PhotoTimelineFilter,
  PhotoTimelineItem,
  PhotoTimelinePage,
  UpdatePhotoAlbumRequest,
  UpdatePhotoPersonRequest,
} from "@/modules/photos/api";
import { formatCameraLabel } from "@/modules/photos/format";
import { stableUuid } from "../fixtures";

export const PHOTOS_LIBRARY_ID = stableUuid("seed-library:photos");

interface StoredPhoto extends PhotoInfo {
  userFavorites: Set<string>;
}

interface StoredAlbum {
  id: string;
  userId: string;
  title: string;
  description: string | null;
  coverItemId: string | null;
  itemIds: string[];
  createdAt: string;
  updatedAt: string;
}

interface StoredAiTag {
  dhash: number;
  dominantColor: string;
  dominantHex: string;
  sceneTags: string[];
  timeOfDay: string;
  season: string;
}

interface StoredPerson {
  id: string;
  ownerId: string;
  name: string;
  coverItemId: string | null;
  itemIds: string[];
  createdAt: string;
}

function popcount32(n: number): number {
  let x = n >>> 0;
  let c = 0;
  while (x !== 0) {
    x &= x - 1;
    c += 1;
  }
  return c;
}

function parseMockQuery(raw: string, nowYear: number): PhotoQueryFilter {
  let text = raw.trim().toLowerCase();
  const filter: PhotoQueryFilter = {
    year: null,
    month: null,
    season: null,
    timeOfDay: null,
    color: null,
    orientation: null,
    cameraBrand: null,
    geotagged: null,
    keywords: [],
  };
  if (!text) return filter;

  const strip = (term: string): boolean => {
    if (/^[a-z0-9_ ]+$/.test(term)) {
      const re = new RegExp(`\\b${term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "g");
      if (re.test(text)) {
        text = text.replace(re, " ");
        return true;
      }
      return false;
    }
    if (text.includes(term)) {
      text = text.split(term).join(" ");
      return true;
    }
    return false;
  };

  if (strip("去年") || strip("last year")) filter.year = nowYear - 1;
  else if (strip("今年") || strip("this year")) filter.year = nowYear;
  else {
    const ym = text.match(/(?:^|[^0-9])(20[0-3][0-9])年?/);
    if (ym?.[1]) {
      filter.year = Number(ym[1]);
      text = text.replace(ym[0], " ");
    }
  }

  for (let m = 12; m >= 1; m -= 1) {
    if (strip(`${m}月`)) {
      filter.month = m;
      break;
    }
  }

  const matchCat = (rules: Array<[string, string[]]>): string | null => {
    for (const [canon, syns] of rules) {
      let hit = false;
      for (const s of syns) {
        if (strip(s)) hit = true;
      }
      if (hit) return canon;
    }
    return null;
  };

  filter.season = matchCat([
    ["spring", ["春季", "春天", "春", "spring"]],
    ["summer", ["夏季", "夏天", "夏", "summer"]],
    ["autumn", ["秋季", "秋天", "秋", "autumn", "fall"]],
    ["winter", ["冬季", "冬天", "冬", "winter"]],
  ]);

  filter.timeOfDay = matchCat([
    ["morning", ["清晨", "早上", "早晨", "上午", "morning"]],
    ["afternoon", ["下午", "午后", "中午", "afternoon"]],
    ["sunset", ["日落", "黄昏", "夕阳", "傍晚", "sunset"]],
    ["night", ["夜景", "夜晚", "晚上", "夜间", "夜", "night"]],
  ]);

  filter.color = matchCat([
    ["blue", ["蓝色", "蓝天", "天空", "海洋", "大海", "蓝", "海", "blue", "sky", "ocean", "sea"]],
    ["red", ["红色", "红", "red"]],
    ["orange", ["橙色", "橘色", "橙", "orange"]],
    ["green", ["绿色", "草地", "森林", "绿", "草", "树", "green", "grass", "tree"]],
    ["warm", ["暖色", "暖调", "暖", "warm"]],
    ["yellow", ["黄色", "金色", "黄", "yellow"]],
    ["dark", ["暗调", "深色", "黑色", "暗", "黑", "dark", "black"]],
    ["bright", ["明亮", "白色", "亮", "白", "bright", "white"]],
  ]);

  filter.orientation = matchCat([
    ["portrait", ["竖拍", "竖图", "竖屏", "portrait", "vertical"]],
    ["landscape", ["横拍", "横图", "横屏", "landscape", "horizontal"]],
  ]);

  filter.cameraBrand = matchCat([
    ["sony", ["索尼", "sony"]],
    ["apple", ["苹果", "iphone", "apple"]],
    ["canon", ["佳能", "canon"]],
    ["nikon", ["尼康", "nikon"]],
    ["fuji", ["富士", "fujifilm", "fuji"]],
    ["huawei", ["华为", "huawei"]],
    ["xiaomi", ["小米", "xiaomi"]],
  ]);

  if (strip("带定位") || strip("有定位") || strip("geotagged") || strip("gps")) {
    filter.geotagged = true;
  }

  for (const stop of ["拍的", "拍摄", "照片", "图片", "的", "在", "里", "中"]) {
    strip(stop);
  }
  const enStops = new Set(["in", "of", "the", "with", "from", "a", "an", "photo", "photos"]);
  filter.keywords = text
    .split(/[\s,，、;；|]+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0 && !enStops.has(s));

  return filter;
}

function latCell(lat: number): number {
  return Math.max(-900, Math.min(899, Math.floor(lat * 10)));
}

function lonCell(lon: number): number {
  return Math.max(-1800, Math.min(1799, Math.floor(lon * 10)));
}

function placeKeyOf(lat: number, lon: number): string {
  return `${(latCell(lat) / 10).toFixed(1)},${(lonCell(lon) / 10).toFixed(1)}`;
}

function placeLabelOf(lat: number, lon: number): string {
  const ns = lat >= 0 ? "N" : "S";
  const ew = lon >= 0 ? "E" : "W";
  return `${Math.abs(lat).toFixed(1)}°${ns}, ${Math.abs(lon).toFixed(1)}°${ew}`;
}

export class PhotosMockState {
  private photos: StoredPhoto[] = [];
  private albums: StoredAlbum[] = [];
  private aiTags = new Map<string, StoredAiTag>();
  private people: StoredPerson[] = [];
  private readonly now: () => number;

  constructor(now: () => number = () => Date.now()) {
    this.now = now;
    this.seed();
  }

  private iso(): string {
    return new Date(this.now()).toISOString().replace(/\.\d{3}Z$/, "Z");
  }

  private seed() {
    const today = new Date(this.now());
    const mm = String(today.getUTCMonth() + 1).padStart(2, "0");
    const dd = String(today.getUTCDate()).padStart(2, "0");
    const lastYear = today.getUTCFullYear() - 1;
    const onThisDayDate = `${lastYear}-${mm}-${dd}`;
    const onThisDayMonth = `${lastYear}-${mm}`;

    const familyUserId = stableUuid("user:family");
    const adminUserId = stableUuid("user:admin");

    const seeds: Array<Omit<StoredPhoto, "albums" | "favorite" | "userFavorites"> & { fav?: boolean }> = [
      {
        id: stableUuid("photo:kyoto-arashiyama"),
        libraryId: PHOTOS_LIBRARY_ID,
        title: "IMG_4821",
        fileName: "IMG_4821.JPG",
        path: "2026/11/IMG_4821.JPG",
        sizeBytes: 6_420_000,
        takenAt: "2026-11-18T08:25:00Z",
        dayBucket: "2026-11-18",
        monthBucket: "2026-11",
        width: 7008,
        height: 4672,
        orientation: 1,
        cameraMake: "SONY",
        cameraModel: "ILCE-7M4",
        lensModel: "FE 35mm F1.4 GM",
        iso: 160,
        fNumber: 2.8,
        exposureTime: "1/250s",
        focalLength: 35,
        latitude: 35.0094,
        longitude: 135.6668,
        contentHash: "a1".repeat(32),
        fav: true,
      },
      {
        id: stableUuid("photo:kyoto-kiyomizu"),
        libraryId: PHOTOS_LIBRARY_ID,
        title: "IMG_4822",
        fileName: "IMG_4822.JPG",
        path: "2026/11/IMG_4822.JPG",
        sizeBytes: 5_890_000,
        takenAt: "2026-11-18T06:10:00Z",
        dayBucket: "2026-11-18",
        monthBucket: "2026-11",
        width: 4672,
        height: 7008,
        orientation: 6,
        cameraMake: "SONY",
        cameraModel: "ILCE-7M4",
        lensModel: "FE 24-70mm F2.8 GM II",
        iso: 320,
        fNumber: 4.0,
        exposureTime: "1/125s",
        focalLength: 50,
        latitude: 35.0156,
        longitude: 135.6761,
        contentHash: "b2".repeat(32),
      },
      {
        id: stableUuid("photo:shanghai-bund"),
        libraryId: PHOTOS_LIBRARY_ID,
        title: "DSC_1090",
        fileName: "DSC_1090.HEIC",
        path: "2026/09/DSC_1090.HEIC",
        sizeBytes: 3_210_000,
        takenAt: "2026-09-05T11:45:00Z",
        dayBucket: "2026-09-05",
        monthBucket: "2026-09",
        width: 4032,
        height: 3024,
        orientation: 1,
        cameraMake: "Apple",
        cameraModel: "iPhone 15 Pro",
        lensModel: "24mm f/1.78",
        iso: 80,
        fNumber: 1.78,
        exposureTime: "1/800s",
        focalLength: 6.77,
        latitude: 31.2397,
        longitude: 121.4998,
        contentHash: "c3".repeat(32),
        fav: true,
      },
      {
        id: stableUuid("photo:on-this-day-harbor"),
        libraryId: PHOTOS_LIBRARY_ID,
        title: "HARBOR_2025",
        fileName: "HARBOR_2025.JPG",
        path: `${lastYear}/${mm}/HARBOR_2025.JPG`,
        sizeBytes: 4_750_000,
        takenAt: `${onThisDayDate}T09:30:00Z`,
        dayBucket: onThisDayDate,
        monthBucket: onThisDayMonth,
        width: 6240,
        height: 4160,
        orientation: 1,
        cameraMake: "FUJIFILM",
        cameraModel: "X-T5",
        lensModel: "XF23mmF1.4 R LM WR",
        iso: 200,
        fNumber: 2.0,
        exposureTime: "1/500s",
        focalLength: 23,
        latitude: 31.2341,
        longitude: 121.4822,
        contentHash: "d4".repeat(32),
      },
    ];

    this.photos = seeds.map(({ fav, ...rest }) => {
      const favUsers = new Set<string>();
      if (fav) {
        favUsers.add(familyUserId);
        favUsers.add(adminUserId);
      }
      return {
        ...rest,
        favorite: Boolean(fav),
        albums: [],
        userFavorites: favUsers,
      };
    });

    const kyotoAlbumId = stableUuid("photo-album:kyoto");
    for (const userId of [familyUserId, adminUserId]) {
      this.albums.push({
        id: userId === familyUserId ? kyotoAlbumId : stableUuid(`photo-album:kyoto:${userId}`),
        userId,
        title: "京都秋行",
        description: "岚山与清水寺枫叶记录",
        coverItemId: seeds[0]!.id,
        itemIds: [seeds[0]!.id, seeds[1]!.id],
        createdAt: "2026-11-19T10:00:00Z",
        updatedAt: "2026-11-19T10:00:00Z",
      });
      this.people.push({
        id: stableUuid(`photo-person:xiaoming:${userId}`),
        ownerId: userId,
        name: "小明",
        coverItemId: seeds[0]!.id,
        itemIds: [seeds[0]!.id],
        createdAt: "2026-11-19T10:00:00Z",
      });
    }

    // Seed local AI tags (IMG_4821 & IMG_4822 share similar dHash dist=2 for Cleanup view)
    this.aiTags.set(seeds[0]!.id, {
      dhash: 0b1111_0000_1010_0101,
      dominantColor: "blue",
      dominantHex: "#2B6CB0",
      sceneTags: ["landscape", "geotagged", "sony", "morning", "autumn", "blue"],
      timeOfDay: "morning",
      season: "autumn",
    });
    this.aiTags.set(seeds[1]!.id, {
      dhash: 0b1111_0000_1010_0110,
      dominantColor: "warm",
      dominantHex: "#B87A3D",
      sceneTags: ["portrait", "geotagged", "sony", "sunset", "autumn", "warm"],
      timeOfDay: "sunset",
      season: "autumn",
    });
    this.aiTags.set(seeds[2]!.id, {
      dhash: 0b0000_1111_0011_1100,
      dominantColor: "blue",
      dominantHex: "#3176B8",
      sceneTags: ["landscape", "geotagged", "apple", "afternoon", "autumn", "blue"],
      timeOfDay: "afternoon",
      season: "autumn",
    });
    this.aiTags.set(seeds[3]!.id, {
      dhash: 0b1010_0101_0000_1111,
      dominantColor: "blue",
      dominantHex: "#1F3A56",
      sceneTags: ["landscape", "geotagged", "fuji", "night", "autumn", "blue"],
      timeOfDay: "night",
      season: "autumn",
    });
  }

  hasPhoto(itemId: string): boolean {
    return this.photos.some((p) => p.id === itemId);
  }

  private toTimelineItem(photo: StoredPhoto, userId: string): PhotoTimelineItem {
    return {
      id: photo.id,
      libraryId: photo.libraryId,
      title: photo.title,
      takenAt: photo.takenAt,
      dayBucket: photo.dayBucket,
      monthBucket: photo.monthBucket,
      width: photo.width,
      height: photo.height,
      orientation: photo.orientation,
      cameraMake: photo.cameraMake,
      cameraModel: photo.cameraModel,
      latitude: photo.latitude,
      longitude: photo.longitude,
      favorite: photo.userFavorites.has(userId),
    };
  }

  setFavorite(userId: string, itemId: string, favorite: boolean): boolean {
    const photo = this.photos.find((p) => p.id === itemId);
    if (!photo) return false;
    if (favorite) photo.userFavorites.add(userId);
    else photo.userFavorites.delete(userId);
    return true;
  }

  isFavorite(userId: string, itemId: string): boolean {
    const photo = this.photos.find((p) => p.id === itemId);
    return Boolean(photo?.userFavorites.has(userId));
  }

  timeline(userId: string, filter: PhotoTimelineFilter): PhotoTimelinePage | null {
    let list = [...this.photos];
    if (filter.libraryId) {
      list = list.filter((p) => p.libraryId === filter.libraryId);
    }
    if (filter.albumId) {
      const album = this.albums.find((a) => a.id === filter.albumId && a.userId === userId);
      if (!album) return null;
      const memberSet = new Set(album.itemIds);
      list = list.filter((p) => memberSet.has(p.id));
    }
    if (filter.favorite !== undefined) {
      list = list.filter((p) => p.userFavorites.has(userId) === filter.favorite);
    }
    if (filter.hasLocation !== undefined) {
      list = list.filter((p) => (p.latitude !== null && p.longitude !== null) === filter.hasLocation);
    }
    if (filter.place) {
      list = list.filter(
        (p) =>
          p.latitude !== null &&
          p.longitude !== null &&
          placeKeyOf(p.latitude, p.longitude) === filter.place,
      );
    }
    if (filter.camera) {
      const target = filter.camera.trim().toLowerCase();
      list = list.filter((p) => {
        const label = formatCameraLabel(p.cameraMake, p.cameraModel)?.toLowerCase();
        return (
          label === target ||
          p.cameraMake?.toLowerCase() === target ||
          p.cameraModel?.toLowerCase() === target
        );
      });
    }
    if (filter.q) {
      const q = filter.q.trim().toLowerCase();
      list = list.filter(
        (p) =>
          p.title.toLowerCase().includes(q) ||
          p.fileName.toLowerCase().includes(q) ||
          p.dayBucket.toLowerCase().includes(q),
      );
    }

    list.sort((a, b) => b.takenAt.localeCompare(a.takenAt) || b.id.localeCompare(a.id));

    const monthCounts = new Map<string, number>();
    for (const p of list) {
      monthCounts.set(p.monthBucket, (monthCounts.get(p.monthBucket) ?? 0) + 1);
    }
    const months: PhotoMonthBucket[] = [...monthCounts.entries()]
      .sort((a, b) => b[0].localeCompare(a[0]))
      .map(([month, count]) => ({ month, count }));

    if (filter.month) {
      list = list.filter((p) => p.monthBucket === filter.month);
    }

    const total = list.length;
    const limit = Math.min(200, Math.max(1, filter.limit ?? 60));
    const offset = filter.cursor ? Number(filter.cursor) || 0 : 0;
    const slice = list.slice(offset, offset + limit);
    const nextCursor = offset + limit < total ? String(offset + limit) : null;

    return {
      items: slice.map((p) => this.toTimelineItem(p, userId)),
      months,
      nextCursor,
      total,
    };
  }

  memories(userId: string): PhotoMemories {
    const sorted = [...this.photos].sort(
      (a, b) => b.takenAt.localeCompare(a.takenAt) || b.id.localeCompare(a.id),
    );
    const today = new Date(this.now());
    const mmDd = `-${String(today.getUTCMonth() + 1).padStart(2, "0")}-${String(today.getUTCDate()).padStart(2, "0")}`;
    const yearCutoff = `${today.getUTCFullYear()}-01-01`;

    const onThisDay = sorted
      .filter((p) => p.dayBucket.endsWith(mmDd) && p.dayBucket < yearCutoff)
      .map((p) => this.toTimelineItem(p, userId));

    const favs = sorted.filter((p) => p.userFavorites.has(userId));
    const rest = sorted.filter((p) => !p.userFavorites.has(userId));
    const recentHighlights = [...favs, ...rest]
      .slice(0, 24)
      .map((p) => this.toTimelineItem(p, userId));

    const camMap = new Map<string, number>();
    const placeMap = new Map<
      string,
      { sumLat: number; sumLon: number; count: number; coverItemId: string }
    >();
    let geotaggedCount = 0;

    for (const p of sorted) {
      const cam = formatCameraLabel(p.cameraMake, p.cameraModel);
      if (cam) camMap.set(cam, (camMap.get(cam) ?? 0) + 1);
      if (p.latitude !== null && p.longitude !== null) {
        geotaggedCount += 1;
        const key = placeKeyOf(p.latitude, p.longitude);
        const existing = placeMap.get(key);
        if (existing) {
          existing.sumLat += p.latitude;
          existing.sumLon += p.longitude;
          existing.count += 1;
        } else {
          placeMap.set(key, {
            sumLat: p.latitude,
            sumLon: p.longitude,
            count: 1,
            coverItemId: p.id,
          });
        }
      }
    }

    const cameras: PhotoCameraBucket[] = [...camMap.entries()]
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));

    const places: PhotoPlaceCluster[] = [...placeMap.entries()]
      .map(([key, acc]) => {
        const lat = Number((acc.sumLat / acc.count).toFixed(4));
        const lon = Number((acc.sumLon / acc.count).toFixed(4));
        return {
          key,
          label: placeLabelOf(lat, lon),
          latitude: lat,
          longitude: lon,
          count: acc.count,
          coverItemId: acc.coverItemId,
        };
      })
      .sort((a, b) => b.count - a.count || a.key.localeCompare(b.key));

    return {
      onThisDay,
      recentHighlights,
      places,
      cameras,
      totalPhotos: sorted.length,
      geotaggedCount,
    };
  }

  photoInfo(userId: string, itemId: string, isAdmin: boolean): PhotoInfo | null {
    const photo = this.photos.find((p) => p.id === itemId);
    if (!photo) return null;
    const containing = this.albums
      .filter((a) => a.userId === userId && a.itemIds.includes(itemId))
      .map((a) => ({ id: a.id, title: a.title }));
    return {
      id: photo.id,
      libraryId: photo.libraryId,
      title: photo.title,
      fileName: photo.fileName,
      path: isAdmin ? `/srv/photos/${photo.path}` : photo.path,
      sizeBytes: photo.sizeBytes,
      takenAt: photo.takenAt,
      dayBucket: photo.dayBucket,
      monthBucket: photo.monthBucket,
      width: photo.width,
      height: photo.height,
      orientation: photo.orientation,
      cameraMake: photo.cameraMake,
      cameraModel: photo.cameraModel,
      lensModel: photo.lensModel,
      iso: photo.iso,
      fNumber: photo.fNumber,
      exposureTime: photo.exposureTime,
      focalLength: photo.focalLength,
      latitude: photo.latitude,
      longitude: photo.longitude,
      contentHash: photo.contentHash,
      favorite: photo.userFavorites.has(userId),
      albums: containing,
    };
  }

  private toAlbumSummary(album: StoredAlbum): PhotoAlbumSummary {
    const liveIds = album.itemIds.filter((id) => this.hasPhoto(id));
    const previewItemIds = liveIds.slice(0, 4);
    const coverItemId =
      album.coverItemId && liveIds.includes(album.coverItemId)
        ? album.coverItemId
        : (previewItemIds[0] ?? null);
    return {
      id: album.id,
      title: album.title,
      description: album.description,
      coverItemId,
      previewItemIds,
      itemCount: liveIds.length,
      createdAt: album.createdAt,
      updatedAt: album.updatedAt,
    };
  }

  listAlbums(userId: string): PhotoAlbumSummary[] {
    return this.albums
      .filter((a) => a.userId === userId)
      .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
      .map((a) => this.toAlbumSummary(a));
  }

  getAlbum(userId: string, albumId: string): PhotoAlbumSummary | null {
    const found = this.albums.find((a) => a.id === albumId && a.userId === userId);
    return found ? this.toAlbumSummary(found) : null;
  }

  createAlbum(userId: string, req: CreatePhotoAlbumRequest): PhotoAlbumSummary {
    const now = this.iso();
    const created: StoredAlbum = {
      id: stableUuid(`photo-album:${userId}:${req.title}:${this.now()}:${Math.random()}`),
      userId,
      title: req.title.trim(),
      description: req.description?.trim() ? req.description.trim() : null,
      coverItemId: req.coverItemId ?? null,
      itemIds: [],
      createdAt: now,
      updatedAt: now,
    };
    this.albums.unshift(created);
    return this.toAlbumSummary(created);
  }

  updateAlbum(
    userId: string,
    albumId: string,
    req: UpdatePhotoAlbumRequest,
  ): PhotoAlbumSummary | null {
    const album = this.albums.find((a) => a.id === albumId && a.userId === userId);
    if (!album) return null;
    if (req.title !== undefined && req.title !== null) album.title = req.title.trim();
    if (req.description !== undefined) {
      album.description = req.description?.trim() ? req.description.trim() : null;
    }
    if (req.coverItemId !== undefined) album.coverItemId = req.coverItemId ?? null;
    album.updatedAt = this.iso();
    return this.toAlbumSummary(album);
  }

  deleteAlbum(userId: string, albumId: string): boolean {
    const before = this.albums.length;
    this.albums = this.albums.filter((a) => !(a.id === albumId && a.userId === userId));
    return this.albums.length < before;
  }

  addAlbumItems(userId: string, albumId: string, itemIds: string[]): PhotoAlbumSummary | null {
    const album = this.albums.find((a) => a.id === albumId && a.userId === userId);
    if (!album) return null;
    for (const id of itemIds) {
      if (this.hasPhoto(id) && !album.itemIds.includes(id)) {
        album.itemIds.push(id);
      }
    }
    if (!album.coverItemId && album.itemIds[0]) {
      album.coverItemId = album.itemIds[0];
    }
    album.updatedAt = this.iso();
    return this.toAlbumSummary(album);
  }

  removeAlbumItems(userId: string, albumId: string, itemIds: string[]): PhotoAlbumSummary | null {
    const album = this.albums.find((a) => a.id === albumId && a.userId === userId);
    if (!album) return null;
    const removeSet = new Set(itemIds);
    album.itemIds = album.itemIds.filter((id) => !removeSet.has(id));
    if (album.coverItemId && removeSet.has(album.coverItemId)) {
      album.coverItemId = album.itemIds[0] ?? null;
    }
    album.updatedAt = this.iso();
    return this.toAlbumSummary(album);
  }

  uploadPhoto(
    userId: string,
    isAdmin: boolean,
    libraryId: string,
    fileName: string,
  ): PhotoInfo {
    const now = new Date(this.now());
    const yyyy = String(now.getUTCFullYear());
    const mm = String(now.getUTCMonth() + 1).padStart(2, "0");
    const dd = String(now.getUTCDate()).padStart(2, "0");
    const cleanName = fileName.replace(/[^A-Za-z0-9._-\u4e00-\u9fa5]/g, "_") || "upload.jpg";
    const stem = cleanName.replace(/\.[^.]+$/, "") || cleanName;
    const id = stableUuid(`photo:upload:${this.now()}:${Math.random()}`);
    const iso = this.iso();

    const stored: StoredPhoto = {
      id,
      libraryId,
      title: stem,
      fileName: cleanName,
      path: `Uploads/${yyyy}/${mm}/${cleanName}`,
      sizeBytes: 1_240_000,
      takenAt: iso,
      dayBucket: `${yyyy}-${mm}-${dd}`,
      monthBucket: `${yyyy}-${mm}`,
      width: 3024,
      height: 2016,
      orientation: 1,
      cameraMake: "Apple",
      cameraModel: "iPhone 15 Pro",
      lensModel: "24mm f/1.78",
      iso: 100,
      fNumber: 1.78,
      exposureTime: "1/250s",
      focalLength: 6.77,
      latitude: null,
      longitude: null,
      contentHash: "f0".repeat(32),
      favorite: false,
      albums: [],
      userFavorites: new Set(),
    };
    this.photos.unshift(stored);
    return this.photoInfo(userId, id, isAdmin)!;
  }

  // -------------------------------------------------------------------------
  // Local AI Analysis, Smart Search, People & Duplicate Cleanup
  // -------------------------------------------------------------------------

  analyze(): PhotoAnalyzeResponse {
    let analyzedCount = 0;
    for (const photo of this.photos) {
      if (!this.aiTags.has(photo.id)) {
        analyzedCount += 1;
        this.aiTags.set(photo.id, {
          dhash: 0b0101_1010_1100_0011,
          dominantColor: "blue",
          dominantHex: "#2F6BA8",
          sceneTags: ["landscape", "morning", "autumn", "blue"],
          timeOfDay: "morning",
          season: "autumn",
        });
      }
    }
    return {
      analyzedCount,
      totalTagged: this.aiTags.size,
      remaining: Math.max(0, this.photos.length - this.aiTags.size),
    };
  }

  private peopleForItem(userId: string, itemId: string): PhotoPersonRef[] {
    return this.people
      .filter((p) => p.ownerId === userId && p.itemIds.includes(itemId))
      .map((p) => ({ id: p.id, name: p.name }));
  }

  search(
    userId: string,
    q = "",
    personId?: string,
    limit = 60,
  ): PhotoSearchResponse {
    const nowYear = new Date(this.now()).getUTCFullYear();
    const parsedFilters = parseMockQuery(q, nowYear);

    const hits: PhotoSearchHit[] = [];
    for (const photo of this.photos) {
      const tag = this.aiTags.get(photo.id);
      const itemPeople = this.peopleForItem(userId, photo.id);
      if (personId && !itemPeople.some((p) => p.id === personId)) continue;

      const takenYear = Number(photo.dayBucket.slice(0, 4));
      const takenMonth = Number(photo.dayBucket.slice(5, 7));
      if (parsedFilters.year !== null && takenYear !== parsedFilters.year) continue;
      if (parsedFilters.month !== null && takenMonth !== parsedFilters.month) continue;
      if (parsedFilters.season && tag?.season !== parsedFilters.season) continue;
      if (parsedFilters.timeOfDay && tag?.timeOfDay !== parsedFilters.timeOfDay) continue;
      if (parsedFilters.color) {
        const dc = tag?.dominantColor ?? "";
        const matchColor =
          parsedFilters.color === "warm"
            ? ["warm", "orange", "yellow"].includes(dc)
            : dc === parsedFilters.color;
        if (!matchColor) continue;
      }
      if (parsedFilters.orientation) {
        const effPortrait =
          photo.width && photo.height
            ? photo.orientation >= 5 && photo.orientation <= 8
              ? photo.width > photo.height
              : photo.height > photo.width
            : false;
        const actual = effPortrait ? "portrait" : "landscape";
        if (actual !== parsedFilters.orientation && !tag?.sceneTags.includes(parsedFilters.orientation)) {
          continue;
        }
      }
      if (parsedFilters.cameraBrand) {
        const cam = `${photo.cameraMake ?? ""} ${photo.cameraModel ?? ""}`.toLowerCase();
        const brand = parsedFilters.cameraBrand;
        const ok =
          cam.includes(brand) ||
          (brand === "apple" && cam.includes("iphone")) ||
          (brand === "fuji" && cam.includes("fujifilm"));
        if (!ok) continue;
      }
      if (parsedFilters.geotagged && (photo.latitude === null || photo.longitude === null)) {
        continue;
      }

      const albumTitles = this.albums
        .filter((a) => a.userId === userId && a.itemIds.includes(photo.id))
        .map((a) => a.title.toLowerCase());
      const haystack = [
        photo.title,
        photo.fileName,
        photo.dayBucket,
        photo.cameraMake ?? "",
        photo.cameraModel ?? "",
        ...albumTitles,
        ...itemPeople.map((p) => p.name),
        ...(tag?.sceneTags ?? []),
      ]
        .join(" ")
        .toLowerCase();
      if (parsedFilters.keywords.some((kw) => !haystack.includes(kw.toLowerCase()))) {
        continue;
      }

      hits.push({
        ...this.toTimelineItem(photo, userId),
        dominantColor: tag?.dominantColor ?? null,
        dominantHex: tag?.dominantHex ?? null,
        timeOfDay: tag?.timeOfDay ?? null,
        season: tag?.season ?? null,
        sceneTags: tag?.sceneTags ?? [],
        people: itemPeople,
      });
      if (hits.length >= limit) break;
    }

    return {
      query: q,
      parsedFilters,
      items: hits,
    };
  }

  duplicates(userId: string, threshold = 6): PhotoDuplicatesResponse {
    const clusters: PhotoDuplicateCluster[] = [];
    const exactUsed = new Set<string>();

    const byHash = new Map<string, StoredPhoto[]>();
    for (const p of this.photos) {
      if (!p.contentHash) continue;
      const list = byHash.get(p.contentHash) ?? [];
      list.push(p);
      byHash.set(p.contentHash, list);
    }

    const rankOf = (p: StoredPhoto) =>
      ((p.width ?? 0) * (p.height ?? 0)) * 1000 + (p.sizeBytes ?? 0);

    for (const [hash, group] of byHash.entries()) {
      if (group.length < 2) continue;
      const sorted = [...group].sort((a, b) => rankOf(b) - rankOf(a));
      const keep = sorted[0]!;
      let reclaimable = 0;
      const items = sorted.map((p, idx) => {
        exactUsed.add(p.id);
        if (idx > 0) reclaimable += p.sizeBytes ?? 0;
        return {
          ...this.toTimelineItem(p, userId),
          fileName: p.fileName,
          sizeBytes: p.sizeBytes,
          recommendedKeep: idx === 0,
          distance: 0,
        };
      });
      clusters.push({
        clusterId: `exact:${hash.slice(0, 12)}`,
        kind: "exact",
        keepItemId: keep.id,
        reclaimableBytes: reclaimable,
        maxDistance: 0,
        items,
      });
    }

    const remaining = this.photos.filter((p) => !exactUsed.has(p.id) && this.aiTags.has(p.id));
    const visited = new Set<string>();
    for (let i = 0; i < remaining.length; i += 1) {
      const seed = remaining[i]!;
      if (visited.has(seed.id)) continue;
      const seedHash = this.aiTags.get(seed.id)!.dhash;
      const group = [seed];
      visited.add(seed.id);
      for (let j = i + 1; j < remaining.length; j += 1) {
        const other = remaining[j]!;
        if (visited.has(other.id)) continue;
        const otherHash = this.aiTags.get(other.id)!.dhash;
        if (popcount32(seedHash ^ otherHash) <= threshold) {
          group.push(other);
          visited.add(other.id);
        }
      }
      if (group.length < 2) continue;
      const sorted = [...group].sort((a, b) => rankOf(b) - rankOf(a));
      const keep = sorted[0]!;
      const keepHash = this.aiTags.get(keep.id)!.dhash;
      let reclaimable = 0;
      let maxDistance = 0;
      const items = sorted.map((p, idx) => {
        const dist = idx === 0 ? 0 : popcount32(keepHash ^ this.aiTags.get(p.id)!.dhash);
        maxDistance = Math.max(maxDistance, dist);
        if (idx > 0) reclaimable += p.sizeBytes ?? 0;
        return {
          ...this.toTimelineItem(p, userId),
          fileName: p.fileName,
          sizeBytes: p.sizeBytes,
          recommendedKeep: idx === 0,
          distance: dist,
        };
      });
      clusters.push({
        clusterId: `similar:${keep.id}`,
        kind: "similar",
        keepItemId: keep.id,
        reclaimableBytes: reclaimable,
        maxDistance,
        items,
      });
    }

    const totalDuplicatePhotos = clusters.reduce((acc, c) => acc + Math.max(0, c.items.length - 1), 0);
    const totalReclaimableBytes = clusters.reduce((acc, c) => acc + c.reclaimableBytes, 0);

    return {
      threshold,
      totalClusters: clusters.length,
      totalDuplicatePhotos,
      totalReclaimableBytes,
      clusters,
    };
  }

  private toPersonSummary(person: StoredPerson): PhotoPersonSummary {
    return {
      id: person.id,
      name: person.name,
      coverItemId: person.coverItemId ?? person.itemIds[0] ?? null,
      previewItemIds: person.itemIds.slice(0, 4),
      photoCount: person.itemIds.length,
      createdAt: person.createdAt,
    };
  }

  listPeople(userId: string): PhotoPersonSummary[] {
    return this.people
      .filter((p) => p.ownerId === userId)
      .map((p) => this.toPersonSummary(p));
  }

  createPerson(userId: string, body: CreatePhotoPersonRequest): PhotoPersonSummary {
    const person: StoredPerson = {
      id: stableUuid(`photo-person:${userId}:${body.name}:${this.now()}:${Math.random()}`),
      ownerId: userId,
      name: body.name.trim(),
      coverItemId: body.coverItemId ?? null,
      itemIds: [],
      createdAt: this.iso(),
    };
    this.people.push(person);
    return this.toPersonSummary(person);
  }

  updatePerson(
    userId: string,
    personId: string,
    body: UpdatePhotoPersonRequest,
  ): PhotoPersonSummary | null {
    const person = this.people.find((p) => p.id === personId && p.ownerId === userId);
    if (!person) return null;
    if (body.name !== undefined) person.name = body.name.trim();
    if (body.coverItemId !== undefined) person.coverItemId = body.coverItemId;
    return this.toPersonSummary(person);
  }

  deletePerson(userId: string, personId: string): boolean {
    const before = this.people.length;
    this.people = this.people.filter((p) => !(p.id === personId && p.ownerId === userId));
    return this.people.length < before;
  }

  listItemPeople(userId: string, itemId: string): PhotoPersonRef[] {
    return this.peopleForItem(userId, itemId);
  }

  tagPerson(userId: string, itemId: string, personId: string): PhotoPersonSummary | null {
    if (!this.hasPhoto(itemId)) return null;
    const person = this.people.find((p) => p.id === personId && p.ownerId === userId);
    if (!person) return null;
    if (!person.itemIds.includes(itemId)) {
      person.itemIds.push(itemId);
    }
    if (!person.coverItemId) {
      person.coverItemId = itemId;
    }
    return this.toPersonSummary(person);
  }

  untagPerson(userId: string, itemId: string, personId: string): boolean {
    const person = this.people.find((p) => p.id === personId && p.ownerId === userId);
    if (!person) return false;
    const before = person.itemIds.length;
    person.itemIds = person.itemIds.filter((id) => id !== itemId);
    if (person.coverItemId === itemId) {
      person.coverItemId = person.itemIds[0] ?? null;
    }
    return person.itemIds.length < before;
  }
}
