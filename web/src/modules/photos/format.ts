import type { PhotoTimelineItem } from "./api";

/** Computes display aspect ratio after EXIF auto-orientation (orientations 5..8 swap width and height). */
export function orientedAspectRatio(photo: Pick<PhotoTimelineItem, "width" | "height" | "orientation">): number {
  const w = photo.width ?? 4;
  const h = photo.height ?? 3;
  if (w <= 0 || h <= 0) return 4 / 3;
  const swapped = photo.orientation >= 5 && photo.orientation <= 8;
  const ratio = swapped ? h / w : w / h;
  return Math.min(2.8, Math.max(0.45, ratio));
}

export function formatCameraLabel(make: string | null | undefined, model: string | null | undefined): string | null {
  const mk = make?.trim() || "";
  const md = model?.trim() || "";
  if (mk && md) {
    return md.toLowerCase().startsWith(mk.toLowerCase()) ? md : `${mk} ${md}`;
  }
  return md || mk || null;
}

export function formatBytesCompact(bytes: number | null | undefined): string {
  if (bytes == null || bytes < 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  return `${(mb / 1024).toFixed(2)} GB`;
}

export function groupPhotosByDay(items: readonly PhotoTimelineItem[]): Array<{ day: string; items: PhotoTimelineItem[] }> {
  const groups: Array<{ day: string; items: PhotoTimelineItem[] }> = [];
  for (const item of items) {
    const last = groups[groups.length - 1];
    if (last && last.day === item.dayBucket) {
      last.items.push(item);
    } else {
      groups.push({ day: item.dayBucket, items: [item] });
    }
  }
  return groups;
}
