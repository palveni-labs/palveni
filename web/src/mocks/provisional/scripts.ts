/**
 * Scripted assistant behaviour for the mock: each script turns a prompt into human-readable
 * steps and result blocks. Real assistant runs will come from the server.
 */
import type { Block, ChangeRow } from "@/blocks/types";
import type { AssistantContext } from "@/provisional/types";
import type { MockDb } from "../db";

export interface ScriptStep {
  /** Shown while the step runs. */
  running: string;
  /** Shown once it is done. */
  done: string;
  /** Second line once done. */
  detail?: string;
  ms: number;
}

export interface ScriptOutcome {
  reply: string;
  /** What the run is doing, in a few words. */
  title?: string;
  subject?: { title: string; artworkId: string | null };
  /** One-line approval request for the inbox. */
  request?: string;
  previews?: string[];
  steps: ScriptStep[];
  blocks: Block[];
  /** Needs a decision before anything changes. */
  awaitsApproval: boolean;
  /** What happens after approval, given the rows the user kept. */
  approve?: (selectedRowIds: string[]) => { steps: ScriptStep[]; blocks: Block[] };
}

export interface ScriptInput {
  text: string;
  context?: AssistantContext;
  db: MockDb;
  activateScene: (id: string) => void;
}

const PHOTOS = ["ph-beach", "ph-park", "ph-cake", "ph-peak", "ph-street", "ph-beach", "ph-peak", "ph-cake", "ph-street", "ph-park", "ph-beach"];

/** 23 groups: 21 identical copies to move, 2 look-alikes to keep. */
export function duplicateRows(): ChangeRow[] {
  const rows: ChangeRow[] = [];
  for (let index = 0; index < 21; index++) {
    const month = String(1 + (index % 12)).padStart(2, "0");
    const number = String(10 + index * 7).padStart(4, "0");
    const name = `IMG_2024${month}_${number}`;
    const thumb = PHOTOS[index % PHOTOS.length] ?? "ph-beach";
    rows.push({
      id: `d${index}`,
      before: `2024/2024-${month}/${name} (1).jpg`,
      after: "回收站",
      detail: `与 ${name}.jpg 完全相同`,
      selected: true,
      group: "move",
      sizeBytes: Math.round((2.1 + (index % 7) * 0.9) * 1024 * 1024),
      thumbs: [thumb, thumb],
      relation: "identical",
    });
  }
  const similar: Array<[string, string, string, number, number]> = [
    ["2024/2024-08/IMG_202408_0102.jpg", "IMG_202408_0103.jpg", "ph-cat", 94, 4.6],
    ["2024/2024-08/IMG_202408_0117.jpg", "IMG_202408_0118.jpg", "ph-park", 91, 5.2],
  ];
  similar.forEach(([path, other, thumb, score, size], index) => {
    rows.push({
      id: `k${index}`,
      before: path,
      after: "保留在原处",
      detail: `与 ${other} 相似度 ${score}%`,
      note: "只是相似，建议保留",
      selected: false,
      group: "keep",
      sizeBytes: Math.round(size * 1024 * 1024),
      thumbs: [thumb, `${thumb}2`],
      relation: "similar",
    });
  });
  return rows;
}

export function duplicateBlocks(): Block[] {
  return [
    { type: "stat", label: "重复", value: 23, unit: "组", icon: "duplicate" },
    { type: "stat", label: "可释放", value: "186", unit: "MB", icon: "storage" },
    { type: "stat", label: "建议保留", value: 2, unit: "组", hint: "只是相似，不是完全相同", icon: "warning", tone: "warning" },
    {
      type: "change-preview",
      id: "dup-preview",
      title: "将移到回收站的文件",
      summary: "每组保留最早的一张，其余移到回收站",
      rows: duplicateRows(),
      groups: [
        { id: "move", label: "完全相同", collapseAfter: 5 },
        { id: "keep", label: "只是相似，建议保留" },
      ],
      columns: { item: "照片", change: "改动", reason: "原因" },
    },
    {
      type: "approval",
      id: "dup-approval",
      title: "把选中的 {count} 张重复照片移到回收站",
      effects: [
        { text: "移动到", path: "/srv/photos/.palveni-trash", icon: "move" },
        { text: "30 天内可以随时恢复", icon: "restore" },
        { text: "不会修改保留下来的原图", icon: "shield" },
      ],
      risk: "medium",
      previewId: "dup-preview",
      confirmLabel: "移到回收站",
      confirmIcon: "trash",
    },
  ];
}

function duplicates(): ScriptOutcome {
  return {
    reply: "好的，我把「家庭相册」从头比对了一遍。",
    title: "整理「家庭相册」的重复照片",
    subject: { title: "家庭相册", artworkId: "ph-beach" },
    request: "助手建议移除 21 张重复照片，可释放 186 MB",
    previews: ["ph-beach", "ph-park", "ph-cake", "ph-peak", "ph-street", "ph-cat", "ph-beach2", "ph-park2"],
    steps: [
      { running: "正在检查「家庭相册」…", done: "已检查 24,736 张照片", detail: "按画面内容比对，不只看文件名", ms: 1100 },
      {
        running: "正在比对内容…",
        done: "找到 23 组重复，其中 2 组只是相似",
        detail: "21 组完全相同，多出来的那一张建议移到回收站，能腾出 186 MB；另外 2 组像是连拍，有细微差别，先替你留着",
        ms: 1300,
      },
    ],
    blocks: duplicateBlocks(),
    awaitsApproval: true,
    approve: (selected) => ({
      steps: [{ running: `正在移动 ${selected.length} 个文件…`, done: `已移动 ${selected.length} 个文件`, ms: 1400 }],
      blocks: [
        {
          type: "text",
          title: "整理完成",
          text: `已把 ${selected.length} 个重复文件移到回收站，释放了 ${Math.round(selected.length * 8.4)} MB。30 天内可以从回收站恢复。`,
          tone: "success",
        },
      ],
    }),
  };
}

function playlist(db: MockDb): ScriptOutcome {
  const tracks = db.libraries.find((library) => library.kind === "music");
  const items = tracks
    ? Array.from({ length: 10 }, (_, index) => ({
        id: `p${index}`,
        title: ["Blue Station", "Quiet Harbor", "夜行列车", "Velvet Rain", "长街慢板", "Midnight Atlas", "初雪", "Paper Moon", "雾港", "Hollow Echo"][index] ?? "",
        subtitle: ["Golden Harbors", "春潮乐队", "Paper Horizons", "白昼乐队"][index % 4],
        kind: "music" as const,
      }))
    : [];
  return {
    reply: "来一份适合周五晚上的爵士歌单。",
    steps: [
      { running: "正在搜索音乐库…", done: "找到 132 首爵士和相近风格的曲目", ms: 1000 },
      { running: "正在挑选节奏舒缓的曲目…", done: "选出 24 首，约 1 小时 38 分钟", ms: 1100 },
    ],
    blocks: [
      { type: "collection", id: "playlist", title: "周五晚上的爵士 · 24 首", layout: "grid", items, more: 14 },
      {
        type: "approval",
        id: "playlist-approval",
        title: "创建歌单「周五晚上的爵士」",
        effects: ["新建包含 24 首曲目的歌单", "Primuse 和其他客户端会同步看到它"],
        risk: "low",
        confirmLabel: "创建歌单",
      },
    ],
    awaitsApproval: true,
    approve: () => ({
      steps: [{ running: "正在创建歌单…", done: "歌单已创建", ms: 700 }],
      blocks: [{ type: "text", text: "歌单「周五晚上的爵士」已创建，Primuse 下次同步时就能看到。", tone: "success" }],
    }),
  };
}

function storage(db: MockDb): ScriptOutcome {
  const GB = 1024 ** 3;
  const sizes: Record<string, number> = { 电影: 2150 * GB, 剧集: 1430 * GB, 家庭相册: 512 * GB, 无损音乐: 410 * GB, 电子书: 18 * GB };
  return {
    reply: "我统计了一下各个媒体库的空间占用。",
    steps: [
      { running: "正在统计各媒体库占用…", done: `统计了 ${db.libraries.length} 个媒体库`, ms: 1200 },
      { running: "正在找出最大的文件…", done: "找到 5 个超过 40 GB 的文件", ms: 900 },
    ],
    blocks: [
      { type: "chart", title: "按媒体库", kind: "donut", unit: "bytes", series: Object.entries(sizes).map(([label, value]) => ({ label, value })) },
      {
        type: "table",
        title: "最大的文件",
        columns: [
          { key: "name", label: "文件" },
          { key: "library", label: "媒体库" },
          { key: "size", label: "大小", align: "end", format: "bytes" },
        ],
        rows: [
          { name: "The Velvet Horizon (2021).mkv", library: "电影", size: 78.4 * GB },
          { name: "The Golden Atlas (1991).mkv", library: "电影", size: 64.1 * GB },
          { name: "山海之间 (2020).mkv", library: "电影", size: 58.9 * GB },
          { name: "星河之歌 S01 Remux", library: "剧集", size: 52.2 * GB },
          { name: "The Silent Archive (2019).mkv", library: "电影", size: 41.7 * GB },
        ],
      },
      { type: "text", text: "电影占了大约 60% 的空间。需要的话，我可以找出没人看过的高码率版本。", tone: "muted" },
    ],
    awaitsApproval: false,
  };
}

function movieScene(activateScene: (id: string) => void): ScriptOutcome {
  return {
    reply: "准备好观影模式了，确认后我来调整客厅的设备。",
    steps: [{ running: "正在读取客厅设备状态…", done: "找到 4 台可以联动的设备", ms: 900 }],
    blocks: [
      {
        type: "approval",
        id: "scene-approval",
        title: "开启观影模式",
        effects: ["主灯关闭，氛围灯带调到 15%", "电视切换到 Viozi", "空调保持 24°C"],
        risk: "low",
        confirmLabel: "开启",
      },
    ],
    awaitsApproval: true,
    approve: () => {
      activateScene("movie");
      return {
        steps: [{ running: "正在调整客厅灯光…", done: "观影模式已开启", ms: 800 }],
        blocks: [{ type: "text", text: "观影模式已开启，祝观影愉快。", tone: "success" }],
      };
    },
  };
}

function rename(input: ScriptInput): ScriptOutcome {
  const titles = (input.context?.itemIds ?? [])
    .map((id) => input.db.getItem(id))
    .filter((item) => item !== undefined)
    .slice(0, 30);
  const rows: ChangeRow[] = titles.map((item, index) => {
    const match = /IMG_(\d{4})(\d{2})_(\d{4})/.exec(item.title);
    const after = match ? `${match[1]}-${match[2]} 家庭相册 ${match[3]}` : `${item.title.replace(/\s*\(\d{4}\)$/, "")} (整理后)`;
    return { id: `r${index}`, before: item.title, after, selected: true };
  });
  return {
    reply: titles.length > 0 ? `收到你选的 ${input.context?.itemIds?.length ?? 0} 个文件，我先给出重命名方案。` : "先在「资料」里选几个文件交给我吧。",
    steps: [{ running: "正在读取文件信息…", done: `读取了 ${titles.length} 个文件的拍摄日期和标题`, ms: 900 }],
    blocks:
      rows.length > 0
        ? [
            { type: "change-preview", id: "rename-preview", title: "重命名方案", rows },
            {
              type: "approval",
              id: "rename-approval",
              title: `重命名 ${rows.length} 个文件`,
              effects: ["只修改文件名，不改动内容", "改名后会重新扫描所在文件夹"],
              risk: "medium",
              previewId: "rename-preview",
              confirmLabel: "重命名",
            },
          ]
        : [],
    awaitsApproval: rows.length > 0,
    approve: (selected) => ({
      steps: [{ running: `正在重命名 ${selected.length} 个文件…`, done: `已重命名 ${selected.length} 个文件`, ms: 1000 }],
      blocks: [{ type: "text", text: `已重命名 ${selected.length} 个文件。`, tone: "success" }],
    }),
  };
}

function fallback(): ScriptOutcome {
  return {
    reply: "我可以帮你整理重复照片、按心情生成歌单、看看存储空间，或者控制家里的设备。",
    steps: [{ running: "正在理解你的问题…", done: "我理解了", ms: 700 }],
    blocks: [],
    awaitsApproval: false,
  };
}

export function pickScript(input: ScriptInput): ScriptOutcome {
  const text = input.text.toLowerCase();
  if (input.context?.itemIds?.length || /重命名|命名|rename/.test(text)) return rename(input);
  if (/重复|duplicate|整理/.test(text)) return duplicates();
  if (/歌单|音乐|爵士|playlist|jazz|music/.test(text)) return playlist(input.db);
  if (/空间|存储|storage|disk|space/.test(text)) return storage(input.db);
  if (/观影|灯|scene|movie|light/.test(text)) return movieScene(input.activateScene);
  return fallback();
}
