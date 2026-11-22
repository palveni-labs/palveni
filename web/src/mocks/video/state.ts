import type {
  MediaInfo,
  MediaState,
  SubtitleTrack,
  VideoEntry,
  VideoGenre,
  VideoHome,
  VideoOrder,
  VideoPerson,
  VideoUserState,
  VideoWorkDetail,
} from "@/modules/video/api";
import { stableUuid } from "../fixtures";
import { ART_PEOPLE, ART_WORKS, type ArtPersonSpec, type ArtWorkSpec } from "./artwork";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;
const MIN = 60_000;
const GB = 1024 ** 3;

export const MOVIES_LIBRARY_ID = stableUuid("seed-library:movies");
export const SERIES_LIBRARY_ID = stableUuid("seed-library:series");

interface CastEntry {
  key: string;
  name: string;
  role: string | null;
  kind: VideoPerson["kind"];
  personSpec?: ArtPersonSpec;
}

interface EpisodeRecord {
  id: string;
  itemId: string;
  seriesId: string;
  seriesTitle: string;
  libraryId: string;
  seasonNumber: number;
  episodeNumber: number;
  title: string;
  originalTitle: string | null;
  overview: string;
  runtimeMs: number;
  premiered: string;
  addedAt: string;
  art: ArtWorkSpec;
  thumbVariant: number;
  mediaInfo: MediaInfo;
}

interface SeasonRecord {
  id: string;
  seriesId: string;
  seriesTitle: string;
  libraryId: string;
  seasonNumber: number;
  title: string;
  overview: string;
  year: number;
  premiered: string;
  addedAt: string;
  posterItemId: string;
  backdropItemId: string;
  episodes: EpisodeRecord[];
}

interface WorkRecord {
  id: string;
  libraryId: string;
  kind: "movie" | "series";
  art: ArtWorkSpec;
  title: string;
  originalTitle: string;
  tagline: string;
  overview: string;
  year: number;
  premiered: string;
  addedAt: string;
  genres: string[];
  runtimeMs: number;
  communityRating: number;
  officialRating: string;
  studios: string[];
  cast: CastEntry[];
  /** Movie primary itemId (null for series). */
  itemId: string | null;
  posterItemId: string;
  backdropItemId: string;
  files: { itemId: string; title: string; sizeBytes: number | null }[];
  seasons: SeasonRecord[];
  mediaInfoByItem: Map<string, MediaInfo>;
}

interface ItemProgressState {
  played: boolean;
  positionMs: number;
  playCount: number;
  favorite: boolean;
  playbackVersion: number;
  lastPlayedAt: string | null;
}

const CAST_POOL: Array<{ key: string; name: string; spec: ArtPersonSpec }> = [
  { key: "p01", name: "陈砚舟", spec: ART_PEOPLE[0]! },
  { key: "p02", name: "林知夏", spec: ART_PEOPLE[1]! },
  { key: "p03", name: "周沉", spec: ART_PEOPLE[2]! },
  { key: "p04", name: "高远山", spec: ART_PEOPLE[3]! },
  { key: "p05", name: "苏清岚", spec: ART_PEOPLE[4]! },
  { key: "p06", name: "陆明川", spec: ART_PEOPLE[5]! },
  { key: "p07", name: "许安宁", spec: ART_PEOPLE[6]! },
  { key: "p08", name: "程亦凡", spec: ART_PEOPLE[7]! },
  { key: "p09", name: "叶微", spec: ART_PEOPLE[8]! },
  { key: "p10", name: "梁颂", spec: ART_PEOPLE[9]! },
  { key: "p11", name: "沈伯言", spec: ART_PEOPLE[10]! },
  { key: "p12", name: "方南枝", spec: ART_PEOPLE[11]! },
];

const MOVIE_META: Record<
  string,
  {
    genres: string[];
    runtimeMin: number;
    rating: number;
    official: string;
    tagline: string;
    overview: string;
    studios: string[];
    roles: [string, string, string, string];
    hdr?: "sdr" | "hdr10" | "dolby-vision";
    uhd?: boolean;
    extraVersion?: string;
    addedDaysAgo: number;
  }
> = {
  "velvet-horizon": {
    genres: ["剧情", "悬疑"],
    runtimeMin: 134,
    rating: 8.6,
    official: "PG-13",
    tagline: "潮水退去之前，海平面仍藏着最后的回信。",
    overview: "一艘失去航志的科考船在深秋漂回东海旧港。档案修复员在受潮胶片与声呐日志中，逐步拼出十年前那次远洋失联背后被尘封的选择。",
    studios: ["海风影业", "南岸制作"],
    roles: ["林海潮", "沈雾", "老船长", "宋律师"],
    hdr: "hdr10",
    uhd: true,
    extraVersion: "1080p 导演剪辑版",
    addedDaysAgo: 4,
  },
  "after-the-city": {
    genres: ["科幻", "剧情"],
    runtimeMin: 118,
    rating: 8.2,
    official: "PG-13",
    tagline: "当最后一座高架熄灯，夜色才真正属于行走的人。",
    overview: "近未来的滨海都市在电网大迁移后陷入半停摆。两位夜间值守工程师在空旷街区间修补通信基站，重新辨认城市记忆的轮廓。",
    studios: ["白昼工作室"],
    roles: ["江野", "陶然", "调度员", "钟博士"],
    hdr: "sdr",
    addedDaysAgo: 11,
  },
  "blue-hour": {
    genres: ["剧情", "爱情"],
    runtimeMin: 108,
    rating: 8.4,
    official: "PG",
    tagline: "日落后的二十分钟，所有未说出口的话都有了回声。",
    overview: "在海拔三千米的观测站轮值期间，气象录音师与重返故乡的建筑师在每天黄昏短暂交会，记录下山谷四季交替的细微声响。",
    studios: ["云端映画"],
    roles: ["顾辰", "温书", "站长", "阿宁"],
    hdr: "sdr",
    addedDaysAgo: 18,
  },
  "golden-atlas": {
    genres: ["冒险", "剧情"],
    runtimeMin: 142,
    rating: 8.8,
    official: "PG-13",
    tagline: "旧地图上标着空白的地方，曾有人走过整整一生。",
    overview: "测绘学家顺着祖父留下的铜版海图穿越河西走廊与塔克拉玛干边缘，在流沙掩埋的驿站遗迹中寻找失落百年的商队手记。",
    studios: ["西风影业", "丝路视界"],
    roles: ["裴行舟", "阿依娜", "向导老马", "馆长"],
    hdr: "hdr10",
    uhd: true,
    addedDaysAgo: 25,
  },
  "night-route": {
    genres: ["悬疑", "犯罪"],
    runtimeMin: 115,
    rating: 8.1,
    official: "R",
    tagline: "暴雨夜的每一站停靠，都离真相更近一步。",
    overview: "一趟跨省夜班长途车因山体滑坡改走旧国道。随行刑警在七名素不相识的乘客之间，逐步抽丝剥茧一桩跨年失踪案。",
    studios: ["暗房影业"],
    roles: ["唐骁", "夏青", "司机老周", "李明"],
    hdr: "sdr",
    addedDaysAgo: 7,
  },
  northbound: {
    genres: ["冒险", "纪录片"],
    runtimeMin: 98,
    rating: 8.5,
    official: "PG",
    tagline: "沿着冬日的冻土与浮冰，一路向北。",
    overview: "历时十四个月实地跟拍破冰补给船与极地气象队，记录寒潮来袭前人类与自然彼此试探的安静瞬间。",
    studios: ["北纬制作"],
    roles: ["领航员", "观测员", "轮机长", "摄影师"],
    hdr: "hdr10",
    uhd: true,
    addedDaysAgo: 30,
  },
  "paper-lanterns": {
    genres: ["家庭", "剧情"],
    runtimeMin: 112,
    rating: 8.3,
    official: "PG",
    tagline: "一盏手糊灯笼，照亮老街三代人的团圆夜。",
    overview: "江南水乡的传统竹编灯笼铺面临搬迁，常年在外漂泊的小女儿回到老宅，在元宵灯会前的七天里重新理解父亲的固执与温柔。",
    studios: ["青瓦影业"],
    roles: ["许墨", "小棠", "老许", "邻家阿婆"],
    hdr: "sdr",
    addedDaysAgo: 42,
  },
  "tide-archive": {
    genres: ["科幻", "悬疑"],
    runtimeMin: 128,
    rating: 8.7,
    official: "PG-13",
    tagline: "每逢大潮，灯塔地下室的磁带就会多出一段尚未录制的声音。",
    overview: "声学档案局接管了一座废弃离岛观测站，研究员们发现库房里的时间标记出现错位，潮汐周期竟与一段来自未来的预警完全重合。",
    studios: ["深海视界", "海风影业"],
    roles: ["陆屿", "白汐", "首席档案官", "守塔人"],
    hdr: "dolby-vision",
    uhd: true,
    extraVersion: "1080p 收藏版",
    addedDaysAgo: 1,
  },
  "above-snowline": {
    genres: ["动作", "冒险"],
    runtimeMin: 121,
    rating: 7.9,
    official: "PG-13",
    tagline: "在海拔六千米以上，呼吸本身就是全部的勇气。",
    overview: "突如其来的暴风雪困住了南坡科考营地，高山救援队必须在七十二小时气象窗口关闭前完成高海拔绳索横渡营救。",
    studios: ["雪岭电影"],
    roles: ["秦岭", "赵雪", "队长老韩", "队医"],
    hdr: "sdr",
    addedDaysAgo: 55,
  },
  "seventh-summer": {
    genres: ["动画", "家庭"],
    runtimeMin: 102,
    rating: 8.9,
    official: "G",
    tagline: "蝉鸣最响的那个暑假，云朵低得伸手就能碰到。",
    overview: "转学到乡间小镇的少年在旧谷仓发现一只会发光的纸飞机，与新结识的伙伴们度过了漫长而清澈的第七个夏天。",
    studios: ["晴空动画"],
    roles: ["小夏", "阿悠", "外婆", "邮差叔叔"],
    hdr: "sdr",
    addedDaysAgo: 62,
  },
  "end-of-line": {
    genres: ["剧情", "历史"],
    runtimeMin: 126,
    rating: 8.0,
    official: "PG-13",
    tagline: "铁轨尽头不是终点，是下一段岁月的起点。",
    overview: "上世纪八十年代末，一条支线蒸汽铁路即将停运。值守小站三十年的老站长与年轻调度员在最后一次春运中送别无数归人。",
    studios: ["长风制片厂"],
    roles: ["周站长", "陈小舟", "售票员", "老司机"],
    hdr: "sdr",
    addedDaysAgo: 48,
  },
  "neon-rain": {
    genres: ["科幻", "动作"],
    runtimeMin: 136,
    rating: 8.4,
    official: "R",
    tagline: "雨夜霓虹之下，记忆是可以被复制的证物。",
    overview: "义体审计师在追查一起神经接口非法改写事件时，意外发现自己三年前的执勤影像被人篡改，整座下城区的地下交易网随之浮出水面。",
    studios: ["赤道影业", "暗房影业"],
    roles: ["韩烬", "安澜", "情报商", "检察官"],
    hdr: "dolby-vision",
    uhd: true,
    addedDaysAgo: 2,
  },
  "lighthouse-keeper": {
    genres: ["悬疑", "剧情"],
    runtimeMin: 114,
    rating: 8.5,
    official: "PG-13",
    tagline: "守住这束光，就是守住夜航者的方向。",
    overview: "孤悬北海的礁石灯塔迎来新旧守塔人交接。随着大雾封岛，两人从彼此戒备到直面各自藏在心底的往事。",
    studios: ["北纬制作"],
    roles: ["老陈", "林木", "补给船长", "记者"],
    hdr: "sdr",
    addedDaysAgo: 70,
  },
  "song-of-dunes": {
    genres: ["科幻", "冒险"],
    runtimeMin: 148,
    rating: 8.6,
    official: "PG-13",
    tagline: "风吹过沙丘的频率，是这颗星球最古老的歌。",
    overview: "干旱星球的地下水脉日渐枯竭，地质勘探小队深入禁区红沙带，寻找传说中能重启生态循环的古代蓄水阵列。",
    studios: ["西风影业"],
    roles: ["亚宁", "洛辰", "族长", "领队"],
    hdr: "hdr10",
    uhd: true,
    addedDaysAgo: 20,
  },
  "last-train": {
    genres: ["悬疑", "犯罪"],
    runtimeMin: 106,
    rating: 7.8,
    official: "R",
    tagline: "零点十五分发车，无人提前下站。",
    overview: "末班环线地铁因信号故障停在江底隧道，车厢内一名乘客的手提箱神秘消失，封闭空间里的心理博弈由此展开。",
    studios: ["暗房影业"],
    roles: ["沈默", "林薇", "列车长", "神秘客"],
    hdr: "sdr",
    addedDaysAgo: 34,
  },
  "forest-letters": {
    genres: ["动画", "剧情"],
    runtimeMin: 96,
    rating: 8.7,
    official: "PG",
    tagline: "森林记得每一片落叶曾去过的远方。",
    overview: "住在林场边缘的小女孩在树洞里捡到一封写给三十年前的信，顺着信中的线索，她和狐狸伙伴穿过雾林寻找失散的收信人。",
    studios: ["晴空动画"],
    roles: ["小禾", "林爷爷", "信使", "母亲"],
    hdr: "sdr",
    addedDaysAgo: 28,
  },
  "deep-echo": {
    genres: ["科幻", "纪录片"],
    runtimeMin: 92,
    rating: 8.3,
    official: "PG",
    tagline: "在三千光年之外，射电望远镜听见了微弱的律动。",
    overview: "结合射电天文观测与空间探测器实景影像，讲述科学家如何从浩瀚宇宙背景噪声中分辨出脉冲星与深空回波。",
    studios: ["星图影像"],
    roles: ["首席天文学家", "数据分析师", "总设计师", "解说"],
    hdr: "hdr10",
    uhd: true,
    addedDaysAgo: 15,
  },
  "salt-flats": {
    genres: ["剧情", "文艺"],
    runtimeMin: 109,
    rating: 8.1,
    official: "PG",
    tagline: "天空落进湖面时，世界只剩下光和风。",
    overview: "两姐妹驾车穿越高原盐湖去归还母亲生前借走的胶片相机，在漫长公路旅途中重新修复早已疏远的亲情。",
    studios: ["云端映画"],
    roles: ["姐姐", "妹妹", "照相馆老板", "旅人"],
    hdr: "sdr",
    addedDaysAgo: 3,
  },
};

const SERIES_META: Record<
  string,
  {
    genres: string[];
    rating: number;
    official: string;
    tagline: string;
    overview: string;
    studios: string[];
    roles: [string, string, string, string];
    seasons: Array<{
      seasonNumber: number;
      year: number;
      overview: string;
      episodes: Array<{ title: string; original: string; runtimeMin: number; overview: string; addedDaysAgo: number }>;
    }>;
  }
> = {
  harbour: {
    genres: ["悬疑", "犯罪"],
    rating: 8.9,
    official: "TV-MA",
    tagline: "每一艘进港的货轮，都带着未写进舱单的秘密。",
    overview: "围绕百年深水集装箱码头展开的双线调查剧。海事稽查与港务调度在迷雾与潮汐之间，追索跨国走私与沉船旧案的真相。",
    studios: ["海风剧场", "北岸制作"],
    roles: ["陈港生", "林岚", "周局长", "老水手"],
    seasons: [
      {
        seasonNumber: 1,
        year: 2022,
        overview: "冬季封航前夕，一具无名集装箱在四号码头被吊起，打破港口多年的平静。",
        episodes: [
          { title: "四号码头", original: "Pier Four", runtimeMin: 52, overview: "夜班吊车发现一只没有报关单的冷链集装箱，海事稽查队长陈港生接手调查。", addedDaysAgo: 180 },
          { title: "潮汐表", original: "Tide Table", runtimeMin: 48, overview: "潮位记录与航道雷达出现十五分钟空白，调度室内部有人刻意隐瞒当晚航迹。", addedDaysAgo: 173 },
          { title: "防波堤", original: "Breakwater", runtimeMin: 50, overview: "老引水员透露十年前同型号货轮曾在外锚地触礁，线索指向港务档案库。", addedDaysAgo: 166 },
          { title: "浓雾警报", original: "Fog Warning", runtimeMin: 54, overview: "大雾锁港之夜，关键证人在修船坞失联，林岚独自追踪可疑拖轮。", addedDaysAgo: 159 },
          { title: "双份舱单", original: "Double Manifest", runtimeMin: 49, overview: "两份相互矛盾的货运提单浮出水面，幕后航运公司浮现轮廓。", addedDaysAgo: 152 },
          { title: "出港", original: "Departure", runtimeMin: 58, overview: "黎明前最后一次涨潮，收网行动在三号泊位展开。", addedDaysAgo: 145 },
        ],
      },
      {
        seasonNumber: 2,
        year: 2024,
        overview: "新航道扩建工程启动，沉入泥沙多年的旧驳船重见天日。",
        episodes: [
          { title: "疏浚", original: "Dredging", runtimeMin: 51, overview: "航道疏浚船打捞出一只密封铁盒，重启停摆已久的跨港调查。", addedDaysAgo: 16 },
          { title: "夜锚地", original: "Night Anchorage", runtimeMin: 53, overview: "外锚地出现未开启 AIS 的补给船，陈港生带队登临核查。", addedDaysAgo: 9 },
          { title: "旧船坞", original: "Dry Dock", runtimeMin: 49, overview: "修船厂老工匠辨认出铁盒上的造船厂钢印，牵出一段陈年合作。", addedDaysAgo: 5 },
          { title: "长笛声", original: "The Horn", runtimeMin: 56, overview: "风暴登陆前夕，港口全线停摆，真相终于在灯塔下揭晓。", addedDaysAgo: 2 },
        ],
      },
    ],
  },
  "star-courier": {
    genres: ["科幻", "冒险"],
    rating: 8.7,
    official: "TV-14",
    tagline: "无论相隔几条悬臂，信件终会送达。",
    overview: "一艘小型星际邮政飞船穿梭于边缘殖民星系之间，船员们运送的不仅是包裹与芯片，更是相隔数光年的人间牵挂。",
    studios: ["星图影像"],
    roles: ["陆航", "苏墨", "机械师阿杰", "领航 AI"],
    seasons: [
      {
        seasonNumber: 1,
        year: 2021,
        overview: "「风信子号」踏上外环航线，接下一单没有署名的加急投递。",
        episodes: [
          { title: "外环第一站", original: "First Stop Outer Rim", runtimeMin: 46, overview: "风信子号在矿业空间站接到一个加密冷藏箱，目的地坐标却不在星图上。", addedDaysAgo: 90 },
          { title: "轨道漂流", original: "Orbital Drift", runtimeMin: 44, overview: "跃迁引擎在冰环带熄火，船员必须借助废弃中继卫星修正航向。", addedDaysAgo: 83 },
          { title: "延迟三年的信", original: "Three Years Late", runtimeMin: 47, overview: "一封来自已撤销观测站的家书，让众人决定绕道五十天文单位。", addedDaysAgo: 76 },
          { title: "静默星港", original: "Silent Port", runtimeMin: 48, overview: "抵达无人值守的旧星港，自动化分拣系统仍在忠实运转。", addedDaysAgo: 69 },
          { title: "尘埃与光", original: "Dust and Light", runtimeMin: 45, overview: "星云尘暴阻断通信，苏墨尝试用光学摩斯信号联系地表定居点。", addedDaysAgo: 62 },
          { title: "返航坐标", original: "Home Vector", runtimeMin: 52, overview: "神秘收件人终于现身，冷藏箱里的种子为荒芜卫星带来生机。", addedDaysAgo: 55 },
        ],
      },
      {
        seasonNumber: 2,
        year: 2023,
        overview: "新开通的深空航道危机四伏，风信子号迎来新的同行者。",
        episodes: [
          { title: "双星驿站", original: "Binary Relay", runtimeMin: 48, overview: "双星引力潮汐引发通信窗口缩短，邮船必须抢在日珥爆发前离港。", addedDaysAgo: 21 },
          { title: "失踪邮袋", original: "Missing Pouch", runtimeMin: 46, overview: "一批历史胶片在转运中丢失，线索指向走私航线。", addedDaysAgo: 14 },
          { title: "白矮星之夜", original: "White Dwarf Night", runtimeMin: 50, overview: "船员在白矮星外围救助了一艘电量耗尽的探测艇。", addedDaysAgo: 7 },
          { title: "下一程", original: "Next Leg", runtimeMin: 54, overview: "邮路向更深处延伸，风信子号启程前往未知星区。", addedDaysAgo: 3 },
        ],
      },
    ],
  },
  "mountain-town": {
    genres: ["剧情", "历史"],
    rating: 8.6,
    official: "TV-PG",
    tagline: "石阶向上蜿蜒，烟火与往事都藏在雾气里。",
    overview: "西南雾都山城三十年变迁史。以一条临江老梯坎上的茶馆、书店与渡口为圆心，讲述普通街坊的悲欢离合。",
    studios: ["青瓦影业"],
    roles: ["廖掌柜", "小满", "江老师", "渡船老刘"],
    seasons: [
      {
        seasonNumber: 1,
        year: 2019,
        overview: "嘉陵江畔的老梯坎迎来大桥通车前的最后一个冬天。",
        episodes: [
          { title: "头班渡船", original: "First Ferry", runtimeMin: 45, overview: "雾气未散的清晨，老渡口迎来返乡探亲的小满，茶馆重新挂起招牌。", addedDaysAgo: 40 },
          { title: "石阶茶香", original: "Stone Steps Tea", runtimeMin: 43, overview: "老街修缮方案公示，街坊们在茶馆里争论着搬迁与留守。", addedDaysAgo: 35 },
          { title: "雨季书店", original: "Rainy Bookshop", runtimeMin: 46, overview: "旧书店清理阁楼，翻出二十年前城建摄影师留下的底片。", addedDaysAgo: 28 },
          { title: "江风晚钟", original: "River Breeze", runtimeMin: 44, overview: "跨江大桥合龙之夜，老渡船完成最后一次摆渡。", addedDaysAgo: 20 },
          { title: "再见梯坎", original: "Farewell Steps", runtimeMin: 50, overview: "新春将至，搬进新居的老邻居再次聚在山顶俯瞰万家灯火。", addedDaysAgo: 12 },
        ],
      },
    ],
  },
  "kitchen-lab": {
    genres: ["纪录片", "生活"],
    rating: 8.4,
    official: "TV-G",
    tagline: "温度、时间与发酵，餐桌背后的微观科学。",
    overview: "食物科学纪录剧集。主厨与食品化学家携手走进地方作坊与现代厨房，用高速微距镜头揭示风味诞生的原理。",
    studios: ["味觉工坊"],
    roles: ["主厨周原", "林博士", "面点师", "酿造匠人"],
    seasons: [
      {
        seasonNumber: 1,
        year: 2024,
        overview: "从火候到菌群，五集探索日常料理最迷人的风味转折。",
        episodes: [
          { title: "美拉德的边界", original: "Edge of Maillard", runtimeMin: 38, overview: "解析焦香的精确温区，对比铁锅猛火与低温慢煎的分子差异。", addedDaysAgo: 15 },
          { title: "面团的呼吸", original: "Living Dough", runtimeMin: 41, overview: "天然酵种在二十四小时低温发酵中如何构筑轻盈气孔与酸香。", addedDaysAgo: 10 },
          { title: "鲜味密码", original: "Umami Code", runtimeMin: 39, overview: "昆布、菌菇与火腿的谷氨酸协同效应，还原清汤吊鲜的秘密。", addedDaysAgo: 6 },
          { title: "油温与酥脆", original: "Crisp Physics", runtimeMin: 37, overview: "水分蒸发与糊化淀粉如何造就咬下瞬间清脆的声响。", addedDaysAgo: 3 },
          { title: "时光的盐", original: "Salt of Time", runtimeMin: 42, overview: "走入百年酱园，观察日晒夜露如何让大豆完成风味蜕变。", addedDaysAgo: 1 },
        ],
      },
    ],
  },
  "under-aurora": {
    genres: ["悬疑", "科幻"],
    rating: 8.5,
    official: "TV-14",
    tagline: "极夜降临后，磁暴会唤醒冰层下的回响。",
    overview: "北极圈内的联合观测站进入为期两个月的极夜。一次罕见地磁风暴切断了对外通信，科研团队在冰芯样本中发现了异常规律。",
    studios: ["北纬制作", "星图影像"],
    roles: ["沈清", "艾瑞克", "站长老顾", "通信员"],
    seasons: [
      {
        seasonNumber: 1,
        year: 2023,
        overview: "极夜第十二天，极光出现异常脉动，冰芯钻探被迫中断。",
        episodes: [
          { title: "极夜初日", original: "First Polar Night", runtimeMin: 50, overview: "最后一班补给直升机起飞后，观测站迎来强地磁预警。", addedDaysAgo: 18 },
          { title: "三号钻孔", original: "Borehole Three", runtimeMin: 49, overview: "深层冰芯中检出不属于这一地质纪元的微弱声波共振。", addedDaysAgo: 11 },
          { title: "绿色天幕", original: "Green Curtain", runtimeMin: 52, overview: "站内备用发电机故障，两人在极光下徒步前往外围气象站。", addedDaysAgo: 5 },
          { title: "破晓之前", original: "Before Dawn", runtimeMin: 55, overview: "短波电台重新接通，所有异常数据指向冰盖深处的天然空腔。", addedDaysAgo: 1 },
        ],
      },
    ],
  },
};

function makeMediaInfo(opts: {
  runtimeMs: number;
  uhd?: boolean;
  hdr?: "sdr" | "hdr10" | "dolby-vision";
  title: string;
}): MediaInfo {
  const uhd = Boolean(opts.uhd);
  const width = uhd ? 3840 : 1920;
  const height = uhd ? 2160 : 1080;
  const bitrateKbps = uhd ? 18_400 : 8_200;
  const sizeBytes = Math.round(((bitrateKbps * 1000) / 8) * (opts.runtimeMs / 1000));
  const third = Math.round(opts.runtimeMs / 3);
  return {
    container: uhd ? "mkv" : "mp4",
    durationMs: opts.runtimeMs,
    sizeBytes: Math.max(sizeBytes, Math.round(1.2 * GB)),
    bitrateKbps,
    video: {
      codec: uhd ? "hevc" : "h264",
      profile: uhd ? "Main 10" : "High",
      width,
      height,
      frameRate: 23.976,
      bitDepth: uhd ? 10 : 8,
      hdr: opts.hdr ?? "sdr",
    },
    audio: [
      { index: 1, codec: uhd ? "eac3" : "aac", language: "zh", title: "中文原声", channels: 6, channelLayout: "5.1", default: true },
      { index: 2, codec: "aac", language: "en", title: "English Stereo", channels: 2, channelLayout: "stereo", default: false },
    ],
    subtitles: [
      { index: 3, codec: "subrip", language: "zh", title: "简体中文", forced: false, default: true, textBased: true },
      { index: 4, codec: "webvtt", language: "en", title: "English", forced: false, default: false, textBased: true },
    ],
    chapters: [
      { startMs: 0, title: `序幕 · ${opts.title}` },
      { startMs: Math.round(opts.runtimeMs * 0.12), title: "第一幕" },
      { startMs: third, title: "转折" },
      { startMs: third * 2, title: "高潮" },
      { startMs: Math.round(opts.runtimeMs * 0.88), title: "尾声" },
    ],
  };
}

/** Leading ASCII initial used by the alphabet jump bar (`A`–`Z` or `#`). */
export function initialLetterFor(title: string, originalTitle: string | null): string {
  const stripArticle = (raw: string) => raw.trim().replace(/^(the|a|an)\s+/i, "");
  const source = originalTitle ? stripArticle(originalTitle) : stripArticle(title);
  const first = source.charAt(0).toUpperCase();
  if (first >= "A" && first <= "Z") return first;
  const fallback = stripArticle(title).charAt(0).toUpperCase();
  return fallback >= "A" && fallback <= "Z" ? fallback : "#";
}

function seededShuffle<T>(items: T[], seed: number): T[] {
  const out = [...items];
  let s = (seed || 1) >>> 0;
  const next = () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(next() * (i + 1));
    [out[i], out[j]] = [out[j]!, out[i]!];
  }
  return out;
}

export class VideoMockState {
  readonly works: WorkRecord[] = [];
  readonly worksById = new Map<string, WorkRecord>();
  readonly episodesByItemId = new Map<string, EpisodeRecord>();
  readonly workByItemId = new Map<string, WorkRecord>();
  readonly artByItemId = new Map<string, { art: ArtWorkSpec; seasonNumber?: number; thumbVariant?: number }>();
  readonly mediaInfoByItemId = new Map<string, MediaInfo>();
  readonly peopleByKey = new Map<string, ArtPersonSpec>();
  private itemState = new Map<string, ItemProgressState>();
  private workFavorite = new Map<string, boolean>();
  readonly sessions = new Map<string, { itemId: string; startMs: number; createdAt: number }>();
  private readonly now: () => number;

  constructor(now: () => number = () => Date.now()) {
    this.now = now;
    for (const p of CAST_POOL) this.peopleByKey.set(p.key, p.spec);
    this.seed();
  }

  private iso(offsetMs = 0): string {
    return new Date(this.now() + offsetMs).toISOString().replace(/\.\d{3}Z$/, "Z");
  }

  private seed() {
    let castCursor = 0;
    const pickCast = (roles: [string, string, string, string]): CastEntry[] => {
      const result: CastEntry[] = [];
      const director = CAST_POOL[(castCursor + 10) % CAST_POOL.length]!;
      const writer = CAST_POOL[(castCursor + 11) % CAST_POOL.length]!;
      result.push({ key: director.key, name: director.name, role: null, kind: "director", personSpec: director.spec });
      result.push({ key: writer.key, name: writer.name, role: null, kind: "writer", personSpec: writer.spec });
      for (let i = 0; i < 4; i++) {
        const actor = CAST_POOL[(castCursor + i) % CAST_POOL.length]!;
        result.push({ key: actor.key, name: actor.name, role: roles[i]!, kind: "actor", personSpec: actor.spec });
      }
      castCursor = (castCursor + 3) % CAST_POOL.length;
      return result;
    };

    for (const art of ART_WORKS) {
      const movieMeta = MOVIE_META[art.id];
      if (movieMeta) {
        const workId = stableUuid(`video-work:${art.id}`);
        const itemId = stableUuid(`video-item:${art.id}:main`);
        const runtimeMs = movieMeta.runtimeMin * MIN;
        const addedAt = this.iso(-movieMeta.addedDaysAgo * DAY);
        const mediaInfo = makeMediaInfo({ runtimeMs, uhd: movieMeta.uhd, hdr: movieMeta.hdr, title: art.title });
        const files: WorkRecord["files"] = [
          {
            itemId,
            title: movieMeta.uhd ? `${art.title} · 4K ${movieMeta.hdr === "dolby-vision" ? "Dolby Vision" : "HDR10"}` : `${art.title} · 1080p`,
            sizeBytes: mediaInfo.sizeBytes,
          },
        ];
        const mediaMap = new Map<string, MediaInfo>([[itemId, mediaInfo]]);
        if (movieMeta.extraVersion) {
          const altId = stableUuid(`video-item:${art.id}:alt`);
          const altInfo = makeMediaInfo({ runtimeMs, uhd: false, hdr: "sdr", title: art.title });
          files.push({ itemId: altId, title: `${art.title} · ${movieMeta.extraVersion}`, sizeBytes: altInfo.sizeBytes });
          mediaMap.set(altId, altInfo);
          this.mediaInfoByItemId.set(altId, altInfo);
          this.artByItemId.set(altId, { art });
        }

        const work: WorkRecord = {
          id: workId,
          libraryId: MOVIES_LIBRARY_ID,
          kind: "movie",
          art,
          title: art.title,
          originalTitle: art.original,
          tagline: movieMeta.tagline,
          overview: movieMeta.overview,
          year: art.year,
          premiered: `${art.year}-09-14`,
          addedAt,
          genres: movieMeta.genres,
          runtimeMs,
          communityRating: movieMeta.rating,
          officialRating: movieMeta.official,
          studios: movieMeta.studios,
          cast: pickCast(movieMeta.roles),
          itemId,
          posterItemId: itemId,
          backdropItemId: itemId,
          files,
          seasons: [],
          mediaInfoByItem: mediaMap,
        };
        this.works.push(work);
        this.worksById.set(work.id, work);
        this.workByItemId.set(itemId, work);
        if (files[1]) this.workByItemId.set(files[1].itemId, work);
        this.artByItemId.set(itemId, { art });
        this.mediaInfoByItemId.set(itemId, mediaInfo);
        continue;
      }

      const seriesMeta = SERIES_META[art.id];
      if (!seriesMeta) continue;
      const workId = stableUuid(`video-work:${art.id}`);
      const seasons: SeasonRecord[] = [];
      const mediaMap = new Map<string, MediaInfo>();
      let firstEpItemId = stableUuid(`video-item:${art.id}:s1e1`);
      let latestAdded = "";
      let firstRuntime = 48 * MIN;

      for (const s of seriesMeta.seasons) {
        const seasonId = stableUuid(`video-season:${art.id}:s${s.seasonNumber}`);
        const episodes: EpisodeRecord[] = [];
        for (let idx = 0; idx < s.episodes.length; idx++) {
          const ep = s.episodes[idx]!;
          const epNum = idx + 1;
          const epItemId = stableUuid(`video-item:${art.id}:s${s.seasonNumber}e${epNum}`);
          if (s.seasonNumber === 1 && epNum === 1) {
            firstEpItemId = epItemId;
            firstRuntime = ep.runtimeMin * MIN;
          }
          const addedAt = this.iso(-ep.addedDaysAgo * DAY);
          if (!latestAdded || addedAt > latestAdded) latestAdded = addedAt;
          const runtimeMs = ep.runtimeMin * MIN;
          const info = makeMediaInfo({ runtimeMs, uhd: s.seasonNumber >= 2 || art.id === "under-aurora", hdr: s.seasonNumber >= 2 ? "hdr10" : "sdr", title: ep.title });
          const epRecord: EpisodeRecord = {
            id: epItemId,
            itemId: epItemId,
            seriesId: workId,
            seriesTitle: art.title,
            libraryId: SERIES_LIBRARY_ID,
            seasonNumber: s.seasonNumber,
            episodeNumber: epNum,
            title: ep.title,
            originalTitle: ep.original,
            overview: ep.overview,
            runtimeMs,
            premiered: `${s.year}-${String(Math.min(12, epNum + 2)).padStart(2, "0")}-10`,
            addedAt,
            art,
            thumbVariant: s.seasonNumber * 10 + epNum,
            mediaInfo: info,
          };
          episodes.push(epRecord);
          this.episodesByItemId.set(epItemId, epRecord);
          this.artByItemId.set(epItemId, { art, seasonNumber: s.seasonNumber, thumbVariant: epRecord.thumbVariant });
          this.mediaInfoByItemId.set(epItemId, info);
          mediaMap.set(epItemId, info);
        }

        const seasonPosterId = stableUuid(`video-poster:${art.id}:s${s.seasonNumber}`);
        this.artByItemId.set(seasonPosterId, { art, seasonNumber: s.seasonNumber });
        seasons.push({
          id: seasonId,
          seriesId: workId,
          seriesTitle: art.title,
          libraryId: SERIES_LIBRARY_ID,
          seasonNumber: s.seasonNumber,
          title: `第 ${s.seasonNumber} 季`,
          overview: s.overview,
          year: s.year,
          premiered: `${s.year}-03-01`,
          addedAt: episodes[episodes.length - 1]?.addedAt ?? this.iso(-30 * DAY),
          posterItemId: seasonPosterId,
          backdropItemId: firstEpItemId,
          episodes,
        });
      }

      const work: WorkRecord = {
        id: workId,
        libraryId: SERIES_LIBRARY_ID,
        kind: "series",
        art,
        title: art.title,
        originalTitle: art.original,
        tagline: seriesMeta.tagline,
        overview: seriesMeta.overview,
        year: art.year,
        premiered: `${art.year}-03-01`,
        addedAt: latestAdded || this.iso(-10 * DAY),
        genres: seriesMeta.genres,
        runtimeMs: firstRuntime,
        communityRating: seriesMeta.rating,
        officialRating: seriesMeta.official,
        studios: seriesMeta.studios,
        cast: pickCast(seriesMeta.roles),
        itemId: null,
        posterItemId: firstEpItemId,
        backdropItemId: firstEpItemId,
        files: [],
        seasons,
        mediaInfoByItem: mediaMap,
      };
      this.works.push(work);
      this.worksById.set(work.id, work);
      for (const s of seasons) {
        for (const ep of s.episodes) this.workByItemId.set(ep.itemId, work);
      }
    }

    this.seedPersonalStates();
  }

  private seedPersonalStates() {
    const workBySlug = (slug: string) => this.works.find((w) => w.art.id === slug);

    // Continue watching movies
    const velvet = workBySlug("velvet-horizon");
    if (velvet?.itemId) {
      this.itemState.set(velvet.itemId, {
        played: false,
        positionMs: Math.round(velvet.runtimeMs * 0.46),
        playCount: 0,
        favorite: true,
        playbackVersion: 2,
        lastPlayedAt: this.iso(-2 * HOUR),
      });
      this.workFavorite.set(velvet.id, true);
    }

    const nightRoute = workBySlug("night-route");
    if (nightRoute?.itemId) {
      this.itemState.set(nightRoute.itemId, {
        played: false,
        positionMs: Math.round(nightRoute.runtimeMs * 0.27),
        playCount: 0,
        favorite: false,
        playbackVersion: 1,
        lastPlayedAt: this.iso(-18 * HOUR),
      });
    }

    // Played & favorite movies
    for (const slug of ["blue-hour", "golden-atlas", "paper-lanterns", "seventh-summer", "lighthouse-keeper"]) {
      const m = workBySlug(slug);
      if (m?.itemId) {
        this.itemState.set(m.itemId, {
          played: true,
          positionMs: 0,
          playCount: 1,
          favorite: slug === "golden-atlas" || slug === "seventh-summer" || slug === "paper-lanterns",
          playbackVersion: 1,
          lastPlayedAt: this.iso(-10 * DAY),
        });
        if (slug === "golden-atlas" || slug === "seventh-summer" || slug === "paper-lanterns") {
          this.workFavorite.set(m.id, true);
        }
      }
    }
    for (const slug of ["after-the-city", "neon-rain", "tide-archive"]) {
      const m = workBySlug(slug);
      if (m) this.workFavorite.set(m.id, true);
    }

    // Series progress: Harbour (S1 all played, S2E1 played, S2E2 in progress -> continueWatching + favorite)
    const harbour = workBySlug("harbour");
    if (harbour) {
      this.workFavorite.set(harbour.id, true);
      for (const ep of harbour.seasons[0]?.episodes ?? []) {
        this.itemState.set(ep.itemId, { played: true, positionMs: 0, playCount: 1, favorite: false, playbackVersion: 1, lastPlayedAt: this.iso(-14 * DAY) });
      }
      const s2 = harbour.seasons[1]?.episodes ?? [];
      if (s2[0]) {
        this.itemState.set(s2[0].itemId, { played: true, positionMs: 0, playCount: 1, favorite: false, playbackVersion: 1, lastPlayedAt: this.iso(-3 * DAY) });
      }
      if (s2[1]) {
        this.itemState.set(s2[1].itemId, { played: false, positionMs: Math.round(s2[1].runtimeMs * 0.62), playCount: 0, favorite: false, playbackVersion: 3, lastPlayedAt: this.iso(-5 * HOUR) });
      }
    }

    // Star Courier: S1E1–E3 played -> S1E4 is Next Up
    const courier = workBySlug("star-courier");
    if (courier) {
      this.workFavorite.set(courier.id, true);
      for (const ep of (courier.seasons[0]?.episodes ?? []).slice(0, 3)) {
        this.itemState.set(ep.itemId, { played: true, positionMs: 0, playCount: 1, favorite: false, playbackVersion: 1, lastPlayedAt: this.iso(-2 * DAY) });
      }
    }

    // Mountain Town: S1E1–E2 played -> S1E3 is Next Up
    const town = workBySlug("mountain-town");
    if (town) {
      for (const ep of (town.seasons[0]?.episodes ?? []).slice(0, 2)) {
        this.itemState.set(ep.itemId, { played: true, positionMs: 0, playCount: 1, favorite: false, playbackVersion: 1, lastPlayedAt: this.iso(-4 * DAY) });
      }
    }

    // Kitchen Lab: S1E1 in progress -> Continue Watching
    const kitchen = workBySlug("kitchen-lab");
    if (kitchen) {
      const ep1 = kitchen.seasons[0]?.episodes[0];
      if (ep1) {
        this.itemState.set(ep1.itemId, { played: false, positionMs: Math.round(ep1.runtimeMs * 0.36), playCount: 0, favorite: false, playbackVersion: 1, lastPlayedAt: this.iso(-9 * HOUR) });
      }
    }

    // Under Aurora: S1E1 played -> S1E2 is Next Up
    const aurora = workBySlug("under-aurora");
    if (aurora) {
      const ep1 = aurora.seasons[0]?.episodes[0];
      if (ep1) {
        this.itemState.set(ep1.itemId, { played: true, positionMs: 0, playCount: 1, favorite: false, playbackVersion: 1, lastPlayedAt: this.iso(-1 * DAY) });
      }
    }
  }

  getItemRawState(itemId: string): ItemProgressState {
    return (
      this.itemState.get(itemId) ?? {
        played: false,
        positionMs: 0,
        playCount: 0,
        favorite: false,
        playbackVersion: 0,
        lastPlayedAt: null,
      }
    );
  }

  getPersonalMediaState(itemId: string): MediaState {
    const s = this.getItemRawState(itemId);
    return {
      favorite: s.favorite,
      stars: 0,
      positionMs: s.positionMs,
      completed: s.played,
      playbackVersion: s.playbackVersion,
      lastPlayedAt: s.lastPlayedAt,
      playCount: s.playCount,
    };
  }

  private userStateForItem(itemId: string, favoriteOverride?: boolean): VideoUserState {
    const s = this.getItemRawState(itemId);
    return {
      played: s.played,
      positionMs: s.positionMs,
      playCount: s.playCount,
      favorite: favoriteOverride ?? s.favorite,
      lastPlayedAt: s.lastPlayedAt,
      unplayedCount: null,
    };
  }

  private userStateForSeason(season: SeasonRecord, workFav: boolean): VideoUserState {
    let unplayed = 0;
    let playCount = 0;
    let lastPlayedAt: string | null = null;
    for (const ep of season.episodes) {
      const s = this.getItemRawState(ep.itemId);
      if (!s.played) unplayed++;
      playCount += s.playCount;
      if (s.lastPlayedAt && (!lastPlayedAt || s.lastPlayedAt > lastPlayedAt)) lastPlayedAt = s.lastPlayedAt;
    }
    return {
      played: season.episodes.length > 0 && unplayed === 0,
      positionMs: 0,
      playCount,
      favorite: workFav,
      lastPlayedAt,
      unplayedCount: unplayed,
    };
  }

  private userStateForWork(work: WorkRecord): VideoUserState {
    const fav = this.workFavorite.get(work.id) ?? false;
    if (work.kind === "movie" && work.itemId) {
      return this.userStateForItem(work.itemId, fav);
    }
    let unplayed = 0;
    let totalEps = 0;
    let playCount = 0;
    let lastPlayedAt: string | null = null;
    for (const season of work.seasons) {
      for (const ep of season.episodes) {
        totalEps++;
        const s = this.getItemRawState(ep.itemId);
        if (!s.played) unplayed++;
        playCount += s.playCount;
        if (s.lastPlayedAt && (!lastPlayedAt || s.lastPlayedAt > lastPlayedAt)) lastPlayedAt = s.lastPlayedAt;
      }
    }
    return {
      played: totalEps > 0 && unplayed === 0,
      positionMs: 0,
      playCount,
      favorite: fav,
      lastPlayedAt,
      unplayedCount: unplayed,
    };
  }

  toWorkEntry(work: WorkRecord): VideoEntry {
    const fileCount = work.kind === "series" ? work.seasons.reduce((acc, s) => acc + s.episodes.length, 0) : work.files.length;
    return {
      id: work.id,
      kind: work.kind,
      title: work.title,
      itemId: work.itemId,
      posterItemId: work.posterItemId,
      overview: work.overview,
      year: work.year,
      seasonNumber: null,
      episodeNumber: null,
      fileCount,
      libraryId: work.libraryId,
      originalTitle: work.originalTitle,
      tagline: work.tagline,
      genres: work.genres,
      runtimeMs: work.runtimeMs,
      communityRating: work.communityRating,
      officialRating: work.officialRating,
      premiered: work.premiered,
      backdropItemId: work.backdropItemId,
      thumbItemId: null,
      addedAt: work.addedAt,
      state: this.userStateForWork(work),
      seriesId: null,
      seriesTitle: null,
    };
  }

  toSeasonEntry(season: SeasonRecord, work: WorkRecord): VideoEntry {
    const fav = this.workFavorite.get(work.id) ?? false;
    return {
      id: season.id,
      kind: "season",
      title: season.title,
      itemId: null,
      posterItemId: season.posterItemId,
      overview: season.overview,
      year: season.year,
      seasonNumber: season.seasonNumber,
      episodeNumber: null,
      fileCount: season.episodes.length,
      libraryId: season.libraryId,
      originalTitle: null,
      tagline: null,
      genres: work.genres,
      runtimeMs: null,
      communityRating: work.communityRating,
      officialRating: work.officialRating,
      premiered: season.premiered,
      backdropItemId: season.backdropItemId,
      thumbItemId: null,
      addedAt: season.addedAt,
      state: this.userStateForSeason(season, fav),
      seriesId: work.id,
      seriesTitle: work.title,
    };
  }

  toEpisodeEntry(ep: EpisodeRecord, fileCountOverride?: number): VideoEntry {
    const work = this.worksById.get(ep.seriesId);
    return {
      id: ep.id,
      kind: "episode",
      title: ep.title,
      itemId: ep.itemId,
      posterItemId: work?.posterItemId ?? ep.itemId,
      overview: ep.overview,
      year: work?.year ?? null,
      seasonNumber: ep.seasonNumber,
      episodeNumber: ep.episodeNumber,
      fileCount: fileCountOverride ?? 1,
      libraryId: ep.libraryId,
      originalTitle: ep.originalTitle,
      tagline: null,
      genres: work?.genres ?? [],
      runtimeMs: ep.runtimeMs,
      communityRating: work?.communityRating ?? null,
      officialRating: work?.officialRating ?? null,
      premiered: ep.premiered,
      backdropItemId: work?.backdropItemId ?? ep.itemId,
      thumbItemId: ep.itemId,
      addedAt: ep.addedAt,
      state: this.userStateForItem(ep.itemId),
      seriesId: ep.seriesId,
      seriesTitle: ep.seriesTitle,
    };
  }

  /** Next episode to watch for a series: first in-progress episode, else first unplayed after last watched, else S01E01 if unwatched. */
  nextUpForSeries(work: WorkRecord): EpisodeRecord | null {
    const all = work.seasons.flatMap((s) => s.episodes);
    if (all.length === 0) return null;
    const inProgress = all.find((ep) => {
      const s = this.getItemRawState(ep.itemId);
      return !s.played && s.positionMs > 0;
    });
    if (inProgress) return inProgress;
    let lastPlayedIdx = -1;
    for (let i = 0; i < all.length; i++) {
      if (this.getItemRawState(all[i]!.itemId).played) lastPlayedIdx = i;
    }
    if (lastPlayedIdx >= 0) {
      return all.slice(lastPlayedIdx + 1).find((ep) => !this.getItemRawState(ep.itemId).played) ?? null;
    }
    return all[0] ?? null;
  }

  /* ---------------------------------------------------------------- queries */

  list(
    libraryId: string,
    params: {
      kind?: string | null;
      series?: string | null;
      season?: string | null;
      q?: string | null;
      genre?: string | null;
      yearFrom?: string | null;
      yearTo?: string | null;
      played?: string | null;
      favorite?: string | null;
      order?: string | null;
      dir?: string | null;
      seed?: string | null;
      startsWith?: string | null;
      cursor?: string | null;
      limit?: string | null;
    },
  ): { entries: VideoEntry[]; nextCursor: string | null; total: number } {
    const limit = Math.min(200, Math.max(1, Number(params.limit) || 60));
    const offset = params.cursor ? Number(params.cursor) || 0 : 0;

    // Seasons / Episodes drill-down
    if (params.series) {
      const work = this.worksById.get(params.series);
      if (!work || work.libraryId !== libraryId || work.kind !== "series") {
        return { entries: [], nextCursor: null, total: 0 };
      }
      if (params.season != null && params.season !== "") {
        const sNum = Number(params.season);
        const season = work.seasons.find((s) => s.seasonNumber === sNum);
        const all = (season?.episodes ?? []).map((ep) => this.toEpisodeEntry(ep));
        const page = all.slice(offset, offset + limit);
        return { entries: page, nextCursor: offset + limit < all.length ? String(offset + limit) : null, total: all.length };
      }
      const all = work.seasons.map((s) => this.toSeasonEntry(s, work));
      const page = all.slice(offset, offset + limit);
      return { entries: page, nextCursor: offset + limit < all.length ? String(offset + limit) : null, total: all.length };
    }

    // Top-level library works
    let items = this.works.filter((w) => w.libraryId === libraryId);
    if (params.kind === "movie" || params.kind === "series") {
      items = items.filter((w) => w.kind === params.kind);
    }
    if (params.q?.trim()) {
      const needle = params.q.trim().toLowerCase();
      items = items.filter(
        (w) =>
          w.title.toLowerCase().includes(needle) ||
          w.originalTitle.toLowerCase().includes(needle) ||
          w.overview.toLowerCase().includes(needle) ||
          w.genres.some((g) => g.toLowerCase().includes(needle)),
      );
    }
    if (params.genre?.trim()) {
      const g = params.genre.trim();
      items = items.filter((w) => w.genres.includes(g));
    }
    if (params.yearFrom) {
      const y = Number(params.yearFrom);
      if (Number.isFinite(y)) items = items.filter((w) => w.year >= y);
    }
    if (params.yearTo) {
      const y = Number(params.yearTo);
      if (Number.isFinite(y)) items = items.filter((w) => w.year <= y);
    }
    if (params.played === "true" || params.played === "false") {
      const want = params.played === "true";
      items = items.filter((w) => this.userStateForWork(w).played === want);
    }
    if (params.favorite === "true") {
      items = items.filter((w) => this.userStateForWork(w).favorite);
    }
    if (params.startsWith) {
      const target = params.startsWith.toUpperCase();
      items = items.filter((w) => initialLetterFor(w.title, w.originalTitle) === target);
    }

    const entries = items.map((w) => this.toWorkEntry(w));
    const order = (params.order as VideoOrder | null) ?? "added";
    const dir = params.dir === "asc" || params.dir === "desc" ? params.dir : order === "title" || order === "poster" ? "asc" : "desc";
    const mult = dir === "asc" ? 1 : -1;

    let sorted: VideoEntry[];
    if (order === "random") {
      sorted = seededShuffle(entries, Number(params.seed) || 42);
    } else {
      const collator = new Intl.Collator("zh-CN", { numeric: true });
      sorted = [...entries].sort((a, b) => {
        switch (order) {
          case "title":
          case "poster":
            return mult * collator.compare(a.title, b.title);
          case "year":
            return mult * ((a.year ?? 0) - (b.year ?? 0)) || collator.compare(a.title, b.title);
          case "rating":
            return mult * ((a.communityRating ?? 0) - (b.communityRating ?? 0)) || collator.compare(a.title, b.title);
          case "runtime":
            return mult * ((a.runtimeMs ?? 0) - (b.runtimeMs ?? 0)) || collator.compare(a.title, b.title);
          case "added":
          default:
            return mult * a.addedAt.localeCompare(b.addedAt) || collator.compare(a.title, b.title);
        }
      });
    }

    const total = sorted.length;
    const page = sorted.slice(offset, offset + limit);
    return {
      entries: page,
      nextCursor: offset + limit < total ? String(offset + limit) : null,
      total,
    };
  }

  genres(libraryId: string): VideoGenre[] {
    const counts = new Map<string, number>();
    for (const w of this.works) {
      if (w.libraryId !== libraryId) continue;
      for (const g of w.genres) counts.set(g, (counts.get(g) ?? 0) + 1);
    }
    return [...counts.entries()]
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name, "zh-CN"));
  }

  home(libraryId?: string | null): VideoHome {
    const works = libraryId ? this.works.filter((w) => w.libraryId === libraryId) : this.works;

    // Continue watching: partially watched movies & episodes sorted by lastPlayedAt desc
    const cw: VideoEntry[] = [];
    for (const w of works) {
      if (w.kind === "movie" && w.itemId) {
        const s = this.getItemRawState(w.itemId);
        if (!s.played && s.positionMs > 0) cw.push(this.toWorkEntry(w));
      } else if (w.kind === "series") {
        for (const season of w.seasons) {
          for (const ep of season.episodes) {
            const s = this.getItemRawState(ep.itemId);
            if (!s.played && s.positionMs > 0) cw.push(this.toEpisodeEntry(ep));
          }
        }
      }
    }
    cw.sort((a, b) => (b.state?.lastPlayedAt ?? "").localeCompare(a.state?.lastPlayedAt ?? ""));

    // Next up: next unwatched episode for started series (excluding ones already in continueWatching)
    const cwIds = new Set(cw.map((e) => e.itemId));
    const nextUp: VideoEntry[] = [];
    for (const w of works) {
      if (w.kind !== "series") continue;
      const hasAnyProgress = w.seasons.some((s) => s.episodes.some((ep) => {
        const st = this.getItemRawState(ep.itemId);
        return st.played || st.positionMs > 0;
      }));
      if (!hasAnyProgress) continue;
      const next = this.nextUpForSeries(w);
      if (next && !cwIds.has(next.itemId)) nextUp.push(this.toEpisodeEntry(next));
    }

    // Latest movies
    const latestMovies = works
      .filter((w) => w.kind === "movie")
      .sort((a, b) => b.addedAt.localeCompare(a.addedAt))
      .slice(0, 24)
      .map((w) => this.toWorkEntry(w));

    // Latest episodes merged by series (latest episode per series, fileCount = recent unplayed/added count)
    const latestEpisodes: VideoEntry[] = [];
    for (const w of works) {
      if (w.kind !== "series") continue;
      const allEps = w.seasons.flatMap((s) => s.episodes).sort((a, b) => b.addedAt.localeCompare(a.addedAt));
      const newest = allEps[0];
      if (!newest) continue;
      const recentCount = allEps.filter((ep) => ep.seasonNumber === newest.seasonNumber).length;
      latestEpisodes.push(this.toEpisodeEntry(newest, recentCount));
    }
    latestEpisodes.sort((a, b) => b.addedAt.localeCompare(a.addedAt));

    return {
      continueWatching: cw.slice(0, 24),
      nextUp: nextUp.slice(0, 24),
      latestMovies,
      latestEpisodes: latestEpisodes.slice(0, 24),
    };
  }

  workDetail(libraryId: string, workId: string): VideoWorkDetail | null {
    const work = this.worksById.get(workId);
    if (!work || work.libraryId !== libraryId) return null;

    const people: VideoPerson[] = work.cast.map((c) => ({
      name: c.name,
      role: c.role,
      kind: c.kind,
      imageUrl: c.personSpec ? `/api/v1/libraries/${encodeURIComponent(libraryId)}/videos/people/${encodeURIComponent(c.key)}/image` : null,
    }));

    const next = work.kind === "series" ? this.nextUpForSeries(work) : null;
    const similar = this.works
      .filter((other) => other.id !== work.id && other.genres.some((g) => work.genres.includes(g)))
      .sort((a, b) => {
        const overlapA = a.genres.filter((g) => work.genres.includes(g)).length;
        const overlapB = b.genres.filter((g) => work.genres.includes(g)).length;
        return overlapB - overlapA || b.communityRating - a.communityRating;
      })
      .slice(0, 12)
      .map((w) => this.toWorkEntry(w));

    return {
      work: this.toWorkEntry(work),
      studios: work.studios,
      people,
      seasons: work.seasons.map((s) => this.toSeasonEntry(s, work)),
      files: work.files,
      nextUp: next ? this.toEpisodeEntry(next) : null,
      similar,
    };
  }

  /* ---------------------------------------------------------------- mutations */

  setWorkPlayed(libraryId: string, workId: string, played: boolean): boolean {
    const work = this.worksById.get(workId);
    if (!work || work.libraryId !== libraryId) return false;
    const itemIds = work.kind === "movie" ? work.files.map((f) => f.itemId) : work.seasons.flatMap((s) => s.episodes.map((e) => e.itemId));
    for (const id of itemIds) this.setItemPlayed(id, played);
    return true;
  }

  setWorkFavorite(libraryId: string, workId: string, favorite: boolean): boolean {
    const work = this.worksById.get(workId);
    if (!work || work.libraryId !== libraryId) return false;
    this.workFavorite.set(work.id, favorite);
    if (work.itemId) {
      const prev = this.getItemRawState(work.itemId);
      this.itemState.set(work.itemId, { ...prev, favorite });
    }
    return true;
  }

  setItemPlayed(itemId: string, played: boolean): void {
    const prev = this.getItemRawState(itemId);
    this.itemState.set(itemId, {
      ...prev,
      played,
      positionMs: 0,
      playCount: played ? Math.max(1, prev.playCount) : prev.playCount,
      lastPlayedAt: played ? this.iso() : prev.lastPlayedAt,
      playbackVersion: prev.playbackVersion + 1,
    });
  }

  reportProgress(itemId: string, update: { positionMs: number; completed: boolean; expectedVersion: number }): MediaState {
    const prev = this.getItemRawState(itemId);
    const next: ItemProgressState = {
      ...prev,
      played: update.completed ? true : prev.played && update.positionMs === 0 ? prev.played : false,
      positionMs: update.completed ? 0 : Math.max(0, Math.round(update.positionMs)),
      playCount: update.completed ? prev.playCount + 1 : prev.playCount,
      playbackVersion: prev.playbackVersion + 1,
      lastPlayedAt: this.iso(),
    };
    this.itemState.set(itemId, next);
    return this.getPersonalMediaState(itemId);
  }

  mediaInfo(itemId: string): MediaInfo {
    const found = this.mediaInfoByItemId.get(itemId);
    if (found) return found;
    return makeMediaInfo({ runtimeMs: 15_000, uhd: false, hdr: "sdr", title: "Sample" });
  }

  subtitles(): SubtitleTrack[] {
    return [
      { id: "zh.vtt", language: "zh", forced: false, hearingImpaired: false, format: "vtt" },
      { id: "en.vtt", language: "en", forced: false, hearingImpaired: false, format: "vtt" },
    ];
  }

  subtitleContent(trackId: string): string | null {
    if (trackId === "zh.vtt") {
      return [
        "WEBVTT",
        "",
        "00:00:00.500 --> 00:00:04.200",
        "潮水退去之前，海面仍保持着最后的平静。",
        "",
        "00:00:04.600 --> 00:00:09.200",
        "灯塔的光束扫过防波堤，指向远方的锚地。",
        "",
        "00:00:09.600 --> 00:00:14.500",
        "「我们还有足够的时间，在涨潮前找到那段录音。」",
        "",
      ].join("\n");
    }
    if (trackId === "en.vtt") {
      return [
        "WEBVTT",
        "",
        "00:00:00.500 --> 00:00:04.200",
        "Before the tide withdraws, the horizon stays impossibly still.",
        "",
        "00:00:04.600 --> 00:00:09.200",
        "The lighthouse beam sweeps across the breakwater toward the anchorage.",
        "",
        "00:00:09.600 --> 00:00:14.500",
        "“We still have time to recover the tape before high water.”",
        "",
      ].join("\n");
    }
    return null;
  }
}
