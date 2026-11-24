/**
 * Declarative UI blocks — PROVISIONAL UI CONTRACT.
 *
 * Assistant results, plugin dashboard widgets and smart-home cards are all described as data
 * using these types and drawn by the host's <BlockRenderer>. Senders (the assistant, plugins)
 * never ship markup or scripts: they send blocks, the host decides how they look and which
 * actions they can trigger. Field names may still change before the server publishes this
 * contract in its OpenAPI document.
 */

/** Small, fixed icon vocabulary so senders cannot reference arbitrary assets. */
export type BlockIcon =
  | "library"
  | "video"
  | "music"
  | "photo"
  | "book"
  | "comic"
  | "folder"
  | "storage"
  | "duplicate"
  | "sparkles"
  | "clock"
  | "download"
  | "transcode"
  | "light"
  | "thermometer"
  | "scene"
  | "check"
  | "warning"
  | "move"
  | "restore"
  | "shield"
  | "trash";

export type BlockTone = "default" | "muted" | "success" | "warning" | "danger";

interface BlockBase {
  /** Stable within one result; required for blocks that carry state (selection, forms). */
  id?: string;
  title?: string;
}

export interface TextBlock extends BlockBase {
  type: "text";
  text: string;
  tone?: BlockTone;
}

export interface StatBlock extends BlockBase {
  type: "stat";
  label: string;
  value: string | number;
  unit?: string;
  hint?: string;
  icon?: BlockIcon;
  tone?: BlockTone;
}

export interface CollectionItem {
  id: string;
  title: string;
  subtitle?: string;
  /** Picks the placeholder artwork style. */
  kind: "video" | "music" | "photo" | "book" | "comic" | "folder";
  /** The item's cover, fetched like any other artwork; without one the placeholder is drawn. */
  artworkId?: string | null;
  badge?: string;
}

export interface CollectionBlock extends BlockBase {
  type: "collection";
  layout?: "grid" | "row";
  items: CollectionItem[];
  /** Shown when more items exist than are listed. */
  more?: number;
}

export type CellFormat = "text" | "number" | "bytes" | "date" | "duration";

export interface TableColumn {
  key: string;
  label: string;
  align?: "start" | "end";
  format?: CellFormat;
}

export interface TableBlock extends BlockBase {
  type: "table";
  columns: TableColumn[];
  rows: Array<Record<string, string | number | null>>;
}

export interface ChangeRow {
  id: string;
  /** What changes: a path or a name. */
  before: string;
  /** Where it goes or what it becomes. */
  after: string;
  /** Secondary line, e.g. why the change is proposed. */
  detail?: string;
  /** Initial checkbox state. */
  selected: boolean;
  /** Group id from the block's `groups`. */
  group?: string;
  sizeBytes?: number;
  /** Artwork ids of the item and of the item it is compared with (host-resolved thumbnails). */
  thumbs?: string[];
  /** How the two thumbnails relate. */
  relation?: "identical" | "similar";
  /** A short label shown before the detail, e.g. "只是相似，建议保留". */
  note?: string;
}

export interface ChangeGroup {
  id: string;
  label: string;
  /** Show this many rows and offer the rest behind a disclosure. */
  collapseAfter?: number;
}

/** Proposed changes, before → after, each row individually selectable. */
export interface ChangePreviewBlock extends BlockBase {
  type: "change-preview";
  id: string;
  summary?: string;
  rows: ChangeRow[];
  /** Row groups in display order; rows without a group come first. */
  groups?: ChangeGroup[];
  /** Column labels for the item, the change and the reason. */
  columns?: { item?: string; change?: string; reason?: string };
}

export interface ApprovalEffect {
  text: string;
  icon?: BlockIcon;
  /** A file system path the effect names (drawn as a path). */
  path?: string;
}

/** Asks the user to confirm an action before anything happens. */
export interface ApprovalBlock extends BlockBase {
  type: "approval";
  id: string;
  /** May contain "{count}", replaced by the number of rows selected in the linked preview. */
  title: string;
  /** Plain-language consequences, one per line. */
  effects: Array<string | ApprovalEffect>;
  risk: "low" | "medium" | "high";
  /** The change preview whose selected rows the confirmation applies to. */
  previewId?: string;
  confirmLabel?: string;
  confirmIcon?: BlockIcon;
  /** Once decided, the block is shown read-only. */
  status?: "pending" | "approved" | "cancelled";
}

export interface ProgressBlock extends BlockBase {
  type: "progress";
  label: string;
  /** 0..1, or null when unknown. */
  value: number | null;
  detail?: string;
  state?: "running" | "succeeded" | "failed";
}

export interface ChartBlock extends BlockBase {
  type: "chart";
  kind: "bar" | "donut";
  unit?: "bytes" | "count";
  series: Array<{ label: string; value: number }>;
}

export type FormField =
  | { kind: "text"; name: string; label: string; value?: string; placeholder?: string }
  | { kind: "number"; name: string; label: string; value?: number; min?: number; max?: number; unit?: string }
  | { kind: "toggle"; name: string; label: string; value?: boolean }
  | { kind: "select"; name: string; label: string; value?: string; options: Array<{ value: string; label: string }> }
  | { kind: "secret"; name: string; label: string; configured?: boolean };

export interface FormBlock extends BlockBase {
  type: "form";
  id: string;
  fields: FormField[];
  submitLabel?: string;
}

/* ---------------------------------------------------------------- devices */

export type DeviceState =
  | { type: "light"; on: boolean; brightness: number; colorTemp?: number }
  | { type: "switch"; on: boolean; power?: number }
  | { type: "climate"; on: boolean; mode: "cool" | "heat" | "auto" | "dry" | "fan"; current: number; target: number; humidity?: number }
  | { type: "sensor"; readings: Array<{ kind: "temperature" | "humidity" | "contact" | "motion" | "pm25"; value: number | boolean; unit?: string }> }
  | {
      type: "media_player";
      status: "playing" | "paused" | "idle" | "off";
      title: string | null;
      subtitle: string | null;
      app: string | null;
      volume: number;
      progress?: number;
      /** Cover of what is playing (host-resolved). */
      artworkId?: string | null;
    };

export interface DeviceInfo {
  id: string;
  name: string;
  room?: string;
  online: boolean;
  state: DeviceState;
}

export interface DeviceBlock extends BlockBase {
  type: "device";
  device: DeviceInfo;
}

export type Block =
  | TextBlock
  | StatBlock
  | CollectionBlock
  | TableBlock
  | ChangePreviewBlock
  | ApprovalBlock
  | ProgressBlock
  | ChartBlock
  | FormBlock
  | DeviceBlock;

export type BlockType = Block["type"];

/** Device state patches a device card may request. */
export type DevicePatch =
  | { on: boolean }
  | { brightness: number }
  | { target: number }
  | { volume: number }
  | { status: "playing" | "paused" };

/** What a rendered block asks the host to do. The host maps these to real API calls. */
export type BlockAction =
  | { type: "approve"; blockId: string; selectedRowIds: string[] }
  | { type: "cancel"; blockId: string }
  | { type: "edit"; blockId: string }
  | { type: "submit"; blockId: string; values: Record<string, string | number | boolean> }
  | { type: "device"; deviceId: string; patch: DevicePatch }
  | { type: "open-item"; itemId: string };
