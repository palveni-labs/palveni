/** Sample data for the PROVISIONAL mock APIs (assistant, unified tasks, home, storage). */
import type { Catalog, CatalogWork, HomeSnapshot, StorageReport, Suggestion, SystemMetrics, Task } from "@/provisional/types";

const GB = 1024 ** 3;
const TB = 1024 ** 4;

export function seedHome(iso: (offsetMs?: number) => string): HomeSnapshot {
  return {
    rooms: [
      { id: "living", name: "客厅" },
      { id: "bedroom", name: "卧室" },
      { id: "study", name: "书房" },
      { id: "kitchen", name: "厨房" },
    ],
    devices: [
      { id: "living-main", roomId: "living", name: "客厅主灯", online: true, state: { type: "light", on: false, brightness: 60 } },
      { id: "living-strip", roomId: "living", name: "氛围灯带", online: true, state: { type: "light", on: true, brightness: 15, colorTemp: 2700 } },
      {
        id: "living-tv",
        roomId: "living",
        name: "客厅电视",
        online: true,
        state: {
          type: "media_player",
          status: "playing",
          title: "The Velvet Horizon",
          subtitle: "1:12:08 / 2:04:30",
          app: "Viozi",
          volume: 32,
          progress: 0.579,
          artworkId: "velvet-horizon",
        },
      },
      { id: "living-ac", roomId: "living", name: "空调", online: true, state: { type: "climate", on: true, mode: "cool", current: 26, target: 24, humidity: 52 } },
      {
        id: "living-climate",
        roomId: "living",
        name: "温湿度计",
        online: true,
        state: { type: "sensor", readings: [{ kind: "temperature", value: 25.8, unit: "°C" }, { kind: "humidity", value: 52, unit: "%" }] },
      },
      { id: "bed-lamp", roomId: "bedroom", name: "床头灯", online: true, state: { type: "light", on: false, brightness: 40 } },
      { id: "bed-humidifier", roomId: "bedroom", name: "加湿器", online: true, state: { type: "switch", on: false } },
      { id: "bed-window", roomId: "bedroom", name: "窗户传感器", online: true, state: { type: "sensor", readings: [{ kind: "contact", value: false }] } },
      { id: "study-lamp", roomId: "study", name: "台灯", online: true, state: { type: "light", on: true, brightness: 80 } },
      { id: "study-plug", roomId: "study", name: "显示器插座", online: true, state: { type: "switch", on: true, power: 46 } },
      {
        id: "study-air",
        roomId: "study",
        name: "空气质量",
        online: true,
        state: { type: "sensor", readings: [{ kind: "pm25", value: 12, unit: "µg/m³" }, { kind: "temperature", value: 23.9, unit: "°C" }] },
      },
      { id: "kitchen-light", roomId: "kitchen", name: "吊灯", online: true, state: { type: "light", on: false, brightness: 70 } },
      { id: "kitchen-plug", roomId: "kitchen", name: "咖啡机插座", online: false, state: { type: "switch", on: false } },
      { id: "kitchen-motion", roomId: "kitchen", name: "人体传感器", online: true, state: { type: "sensor", readings: [{ kind: "motion", value: false }] } },
    ],
    scenes: [
      { id: "movie", name: "观影模式", icon: "movie", active: true },
      { id: "home", name: "回家", icon: "home", active: false },
      { id: "night", name: "晚安", icon: "night", active: false },
      { id: "morning", name: "早安", icon: "morning", active: false },
      { id: "away", name: "离家", icon: "away", active: false },
    ],
    rules: [
      { id: "r1", trigger: "开始播放电影或剧集", action: "主灯关闭，灯带调到 15%", enabled: true, lastRunAt: iso(-40 * 60_000) },
      { id: "r2", trigger: "暂停超过 1 分钟", action: "客厅灯光恢复到 50%", enabled: true, lastRunAt: iso(-26 * 3600_000) },
      { id: "r3", trigger: "片尾字幕出现", action: "过道灯缓慢亮起", enabled: true, lastRunAt: iso(-26 * 3600_000) },
      { id: "r4", trigger: "晚上 11 点后开始播放", action: "电视音量上限 30%", enabled: false, lastRunAt: null },
      { id: "r5", trigger: "书房开始播放音乐", action: "台灯切换为暖色", enabled: true, lastRunAt: iso(-3 * 3600_000) },
    ],
  };
}

export function seedStorage(iso: (offsetMs?: number) => string): StorageReport {
  return {
    volumes: [
      { id: "v1", label: "数据盘", path: "/volume1", totalBytes: 8 * TB, usedBytes: 6.2 * TB },
      { id: "v2", label: "备份盘", path: "/volume2", totalBytes: 4 * TB, usedBytes: 1.1 * TB },
    ],
    byLibrary: [
      { libraryId: "movies", name: "电影", bytes: 2.1 * TB },
      { libraryId: "series", name: "剧集", bytes: 1.4 * TB },
      { libraryId: "photos", name: "家庭相册", bytes: 512 * GB },
      { libraryId: "music", name: "无损音乐", bytes: 410 * GB },
      { libraryId: "books", name: "电子书", bytes: 18 * GB },
    ],
    mounts: [
      { id: "m-115", label: "115 网盘", state: "offline", affectedLibraries: 2, since: iso(-14 * 60_000) },
      { id: "m-usb", label: "USB 归档盘", state: "online", affectedLibraries: 1, since: iso(-6 * 24 * 3600_000) },
    ],
  };
}

export function seedMetrics(): SystemMetrics {
  return { cpu: 0.12, memoryUsedBytes: 3.1 * GB, memoryTotalBytes: 8 * GB, devicesOnline: 2 };
}

/** Recently added works with their (original, drawn-for-the-project) covers. */
export function seedCatalog(iso: (offsetMs?: number) => string, libraryId: (key: string) => string): Catalog {
  const work = (id: string, title: string, kind: CatalogWork["kind"], meta: string | null, library: string, hoursAgo: number): CatalogWork => ({
    id,
    title,
    kind,
    meta,
    artworkId: id,
    libraryId: libraryId(library),
    addedAt: iso(-hoursAgo * 3600_000),
  });
  return {
    recent: [
      work("velvet-horizon", "The Velvet Horizon", "movie", "2021", "movies", 2),
      work("paper-lanterns", "Paper Lanterns", "movie", "2019", "movies", 3),
      work("blue-hour", "Blue Hour", "album", "Primuse Sessions", "music", 5),
      work("after-the-city", "After the City", "album", "Quiet Hours", "music", 5),
      { ...work("night-route", "长夜的航线", "book", null, "books", 30) },
      work("northbound", "Northbound", "series", "第 2 季", "series", 26),
    ],
    libraryCovers: {
      [libraryId("movies")]: ["velvet-horizon", "paper-lanterns", "big-buck-bunny", "golden-atlas"],
      [libraryId("series")]: ["northbound"],
      [libraryId("music")]: ["blue-hour", "after-the-city", "album1", "album7"],
      [libraryId("photos")]: ["ph-beach", "ph-street", "ph-peak", "ph-cake"],
      [libraryId("books")]: ["night-route"],
    },
  };
}

export const SUGGESTIONS: Suggestion[] = [
  { id: "s2", icon: "music", text: "周五晚上了，来一份爵士歌单？", prompt: "生成一个周五晚上听的爵士歌单", actionLabel: "生成歌单" },
  { id: "s3", icon: "storage", text: "「家庭相册」本周多了 3.1 GB，看看空间都去哪了？", prompt: "看看存储空间都被什么占用了", actionLabel: "看看空间" },
  { id: "s1", icon: "duplicate", text: "发现 23 组重复照片，要整理吗？", prompt: "帮我整理家庭相册里的重复照片", actionLabel: "开始整理" },
  { id: "s4", icon: "scene", text: "电影开始前，一键进入观影模式", prompt: "打开客厅的观影模式", actionLabel: "开启" },
];

const ALBUMS = ["album1", "album2", "album3", "album4", "album5", "album6", "album7", "album8"];

/** Background work that is not a library scan (those come from the M0 mock's jobs). */
export function seedTasks(iso: (offsetMs?: number) => string): Task[] {
  const step = (id: string, label: string, state: Task["steps"][number]["state"], at: string | null, detail: string | null = null) => ({
    id,
    label,
    state,
    at,
    detail,
  });
  return [
    {
      id: "t-covers",
      kind: "assistant.run",
      title: "助手想为「无损音乐」补全 8 张专辑封面",
      scope: "无损音乐",
      previews: ALBUMS,
      state: "awaiting_approval",
      progress: null,
      message: "找到 8 张专辑的封面，等你确认后写入",
      error: null,
      createdAt: iso(-22 * 60_000),
      startedAt: iso(-22 * 60_000),
      finishedAt: null,
      origin: { module: "assistant", label: "助手对话" },
      steps: [
        step("a", "检查「无损音乐」里的专辑", "done", iso(-22 * 60_000), "共 136 张专辑，8 张没有封面"),
        step("b", "在 MusicBrainz 查找封面", "done", iso(-21 * 60_000), "8 张全部找到"),
        step("c", "等待确认", "running", iso(-20 * 60_000)),
        step("d", "写入封面文件", "pending", null),
      ],
      result: [
        {
          type: "change-preview",
          id: "covers",
          title: "将写入的封面",
          summary: "写入专辑文件夹里的 cover.jpg，不改动音频文件",
          rows: [
            ["长夜之歌", "春潮乐队"],
            ["Quiet Sessions", "Golden Harbors"],
            ["远方来信", "白昼乐队"],
            ["Station Tapes", "Paper Horizons"],
            ["雾港", "星河乐队"],
            ["Northern Live", "Wild Echoes"],
            ["初雪", "旧城乐队"],
            ["Velvet EP", "Hidden Atlases"],
          ].map(([album, artist], index) => ({
            id: `c${index}`,
            before: `${artist}/${album}/cover.jpg`,
            after: "新建封面",
            detail: "来源：MusicBrainz · 1200×1200",
            selected: true,
            thumbs: [ALBUMS[index] ?? "album1"],
            sizeBytes: 380_000 + index * 41_000,
          })),
          columns: { item: "专辑", change: "改动", reason: "来源" },
        },
        {
          type: "approval",
          id: "covers-approval",
          title: "写入 8 张专辑封面",
          effects: [
            { text: "在每个专辑文件夹里新建 cover.jpg", icon: "folder" },
            { text: "不会修改任何音频文件或标签", icon: "shield" },
          ],
          risk: "low",
          previewId: "covers",
          confirmLabel: "写入封面",
        },
      ],
    },
    {
      id: "t-download",
      kind: "download",
      title: "Big Buck Bunny (2008) 2160p",
      subject: { title: "Big Buck Bunny (2008)", detail: "2160p", artworkId: "big-buck-bunny" },
      rate: "12.4 MB/s",
      etaSeconds: 180,
      state: "running",
      progress: 0.65,
      message: null,
      error: null,
      createdAt: iso(-9 * 60_000),
      startedAt: iso(-9 * 60_000),
      finishedAt: null,
      origin: { module: "apps", label: "qBittorrent 插件" },
      steps: [step("a", "连接 qBittorrent", "done", iso(-9 * 60_000)), step("b", "下载中", "running", iso(-9 * 60_000)), step("c", "移入「电影」并扫描", "pending", null)],
      result: [],
    },
    {
      id: "t-transcode",
      kind: "transcode",
      title: "The Velvet Horizon (2021) → 1080p H.264",
      subject: { title: "The Velvet Horizon (2021)", detail: "1080p H.264", artworkId: "velvet-horizon" },
      rate: "4.2×",
      etaSeconds: 360,
      state: "running",
      progress: 0.37,
      message: "Intel QSV",
      error: null,
      createdAt: iso(-4 * 60_000),
      startedAt: iso(-4 * 60_000),
      finishedAt: null,
      origin: { module: "home", label: "客厅电视正在播放" },
      steps: [step("a", "选择播放方案", "done", iso(-4 * 60_000), "HEVC 10-bit 超出电视能力，转码为 H.264"), step("b", "硬件转码 (Intel QSV)", "running", iso(-4 * 60_000))],
      result: [],
    },
    {
      id: "t-subtitle",
      kind: "ai",
      title: "生成中文字幕：The Golden Atlas (1991)",
      subject: { title: "The Golden Atlas (1991)", detail: null, verb: "生成中文字幕", artworkId: "golden-atlas" },
      state: "succeeded",
      progress: 1,
      message: "1,284 条字幕",
      error: null,
      createdAt: iso(-3 * 3600_000),
      startedAt: iso(-3 * 3600_000),
      finishedAt: iso(-3 * 3600_000 + 14 * 60_000),
      origin: { module: "assistant", label: "助手对话" },
      steps: [
        step("a", "提取音轨", "done", iso(-3 * 3600_000)),
        step("b", "语音转写（英语）", "done", iso(-3 * 3600_000 + 8 * 60_000), "1 小时 52 分钟"),
        step("c", "翻译为简体中文", "done", iso(-3 * 3600_000 + 13 * 60_000)),
        step("d", "保存为 The Golden Atlas (1991).zh-CN.srt", "done", iso(-3 * 3600_000 + 14 * 60_000)),
      ],
      result: [
        { type: "stat", label: "字幕条数", value: "1,284", icon: "check" },
        { type: "stat", label: "用时", value: "14", unit: "分钟", icon: "clock" },
        { type: "text", text: "字幕已保存在影片旁边，Viozi 播放时会自动加载。", tone: "muted" },
      ],
    },
    {
      id: "t-nightly",
      kind: "automation",
      title: "每晚 03:00 扫描全部媒体库",
      state: "succeeded",
      progress: 1,
      message: "6 个媒体库，新增 41 个文件",
      error: null,
      createdAt: iso(-17 * 3600_000),
      startedAt: iso(-17 * 3600_000),
      finishedAt: iso(-17 * 3600_000 + 3 * 60_000),
      origin: { module: "system", label: "定时任务" },
      steps: [step("a", "扫描 6 个媒体库", "done", iso(-17 * 3600_000)), step("b", "新增 41 个文件，移除 3 个", "done", iso(-17 * 3600_000 + 3 * 60_000))],
      result: [],
    },
    {
      id: "t-failed",
      kind: "transcode",
      title: "星河之歌 S01E03 → 720p",
      state: "failed",
      progress: 0.12,
      message: null,
      error: "NVENC 不可用（Cannot load libcuda.so.1），已改用软件编码后仍超时",
      createdAt: iso(-6 * 3600_000),
      startedAt: iso(-6 * 3600_000),
      finishedAt: iso(-6 * 3600_000 + 5 * 60_000),
      origin: { module: "home", label: "卧室平板" },
      steps: [step("a", "选择播放方案", "done", iso(-6 * 3600_000)), step("b", "硬件转码 (NVENC)", "failed", iso(-6 * 3600_000), "Cannot load libcuda.so.1"), step("c", "软件编码", "failed", iso(-6 * 3600_000 + 5 * 60_000), "超过 5 分钟未完成")],
      result: [],
    },
  ];
}
