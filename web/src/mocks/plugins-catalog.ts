import type { PluginCatalogEntry } from "@/api/types";

export const OFFICIAL_PLUGIN_CATALOG: readonly Omit<PluginCatalogEntry, "installed" | "installedVersion">[] = [
  {
    id: "palveni.recipe.tmdb",
    name: "TMDB Metadata Recipe",
    version: "1.2.0",
    previousVersion: "1.1.0",
    runtime: "recipe",
    provides: ["metadata"],
    permissions: ["network:api.themoviedb.org", "library:write_metadata"],
    riskLevel: "medium",
    source: "official",
    description: {
      "zh-CN": "声明式 TMDB 影视元数据刮削配方，自动补齐电影与剧集简介、演职表与海报。",
      en: "Declarative TMDB metadata recipe for movie and series synopses, cast and artwork.",
    },
    settingsSchema: {
      type: "object",
      properties: {
        language: {
          type: "string",
          title: "Metadata Language",
          enum: ["zh-CN", "en-US", "ja-JP"],
          default: "zh-CN",
        },
        includeAdult: {
          type: "boolean",
          title: "Include Adult Titles",
          default: false,
        },
        autoDownloadPoster: {
          type: "boolean",
          title: "Auto Download Poster",
          default: true,
        },
      },
    },
    defaultSettings: {
      language: "zh-CN",
      includeAdult: false,
      autoDownloadPoster: true,
    },
  },
  {
    id: "palveni.recipe.bangumi",
    name: "Bangumi Anime & Book Recipe",
    version: "1.1.0",
    previousVersion: "1.0.0",
    runtime: "recipe",
    provides: ["metadata"],
    permissions: ["network:api.bgm.tv", "library:write_metadata"],
    riskLevel: "medium",
    source: "official",
    description: {
      "zh-CN": "面向番剧、漫画与轻小说的 Bangumi 刮削配方，支持中文译名优先与角色资料补全。",
      en: "Bangumi metadata recipe tailored for anime, manga and books with Chinese title preference.",
    },
    settingsSchema: {
      type: "object",
      properties: {
        preferChineseTitle: {
          type: "boolean",
          title: "Prefer Chinese Title",
          default: true,
        },
        fetchCharacters: {
          type: "boolean",
          title: "Fetch Characters",
          default: true,
        },
      },
    },
    defaultSettings: {
      preferChineseTitle: true,
      fetchCharacters: true,
    },
  },
  {
    id: "palveni.recipe.lrclib",
    name: "LRCLIB Synced Lyrics Recipe",
    version: "1.0.2",
    previousVersion: "1.0.0",
    runtime: "recipe",
    provides: ["lyrics"],
    permissions: ["network:lrclib.net"],
    riskLevel: "low",
    source: "official",
    description: {
      "zh-CN": "从 LRCLIB 拉取精准逐行同步 LRC 歌词，并在无时间轴歌词时平滑回退纯文本歌词。",
      en: "Fetches time-synced LRC lyrics from LRCLIB with optional plain-text fallback.",
    },
    settingsSchema: {
      type: "object",
      properties: {
        preferSynced: {
          type: "boolean",
          title: "Prefer Synced LRC",
          default: true,
        },
        fallbackPlain: {
          type: "boolean",
          title: "Fallback to Plain Lyrics",
          default: true,
        },
      },
    },
    defaultSettings: {
      preferSynced: true,
      fallbackPlain: true,
    },
  },
  {
    id: "palveni.recipe.opensubtitles",
    name: "OpenSubtitles Subtitle Recipe",
    version: "1.0.1",
    previousVersion: "1.0.0",
    runtime: "recipe",
    provides: ["subtitles"],
    permissions: ["network:api.opensubtitles.com", "library:write_subtitles"],
    riskLevel: "medium",
    source: "official",
    description: {
      "zh-CN": "按文件哈希与片名自动匹配多语言外挂字幕，并保存至对应视频旁路目录。",
      en: "Matches multilingual subtitles via OpenSubtitles and saves sidecar tracks beside videos.",
    },
    settingsSchema: {
      type: "object",
      properties: {
        preferredLanguages: {
          type: "string",
          title: "Preferred Languages",
          minLength: 2,
          maxLength: 64,
          default: "zh-CN,en",
        },
        hearingImpaired: {
          type: "boolean",
          title: "Include Hearing Impaired (SDH)",
          default: false,
        },
      },
    },
    defaultSettings: {
      preferredLanguages: "zh-CN,en",
      hearingImpaired: false,
    },
  },
  {
    id: "palveni.widget.clock_weather",
    name: "Home Overview Clock & Media Summary Widget",
    version: "1.0.0",
    previousVersion: null,
    runtime: "widget",
    provides: ["widget"],
    permissions: [],
    riskLevel: "low",
    source: "official",
    description: {
      "zh-CN": "零权限概览小组件，提供本地时钟与家庭媒体库概览状态徽标。",
      en: "Zero-permission overview widget showing a local clock and media storage summary badge.",
    },
    settingsSchema: {
      type: "object",
      properties: {
        use24Hour: {
          type: "boolean",
          title: "24-Hour Clock",
          default: true,
        },
        showStorageBadge: {
          type: "boolean",
          title: "Show Storage Badge",
          default: true,
        },
      },
    },
    defaultSettings: {
      use24Hour: true,
      showStorageBadge: true,
    },
  },
  {
    id: "palveni.widget.media_quote",
    name: "Daily Film & Book Quote Widget",
    version: "1.0.0",
    previousVersion: null,
    runtime: "widget",
    provides: ["widget"],
    permissions: ["library:read"],
    riskLevel: "low",
    source: "official",
    description: {
      "zh-CN": "只读概览小组件，每日从本地影视与书籍资料中精选一句台词或摘录。",
      en: "Read-only overview widget surfacing a daily quote from your film and book catalog.",
    },
    settingsSchema: {
      type: "object",
      properties: {
        sourceDomain: {
          type: "string",
          title: "Quote Source",
          enum: ["all", "video", "book"],
          default: "all",
        },
      },
    },
    defaultSettings: {
      sourceDomain: "all",
    },
  },
];
