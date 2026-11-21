import {
  BookImage,
  BookOpen,
  Clapperboard,
  Folder,
  Image,
  Music,
  type LucideIcon,
} from "lucide-react";
import type { ItemKind, LibraryKind } from "@/api/types";

export const LIBRARY_KINDS: readonly LibraryKind[] = ["video", "music", "photo", "book", "comic"];

export function isLibraryKind(value: unknown): value is LibraryKind {
  return typeof value === "string" && (LIBRARY_KINDS as readonly string[]).includes(value);
}

interface KindMeta {
  icon: LucideIcon;
  /** Leaf item kind produced by a library of this kind. */
  itemKind: ItemKind;
  /** Placeholder tile aspect ratio (width / height) and the matching utility class. */
  aspect: number;
  aspectClass: string;
}

export const LIBRARY_KIND_META: Record<LibraryKind, KindMeta> = {
  video: { icon: Clapperboard, itemKind: "video", aspect: 2 / 3, aspectClass: "aspect-[2/3]" },
  music: { icon: Music, itemKind: "track", aspect: 1, aspectClass: "aspect-square" },
  photo: { icon: Image, itemKind: "photo", aspect: 1, aspectClass: "aspect-square" },
  book: { icon: BookOpen, itemKind: "book", aspect: 2 / 3, aspectClass: "aspect-[2/3]" },
  comic: { icon: BookImage, itemKind: "comic", aspect: 2 / 3, aspectClass: "aspect-[2/3]" },
};

export const ITEM_KIND_ICON: Record<ItemKind, LucideIcon> = {
  folder: Folder,
  video: Clapperboard,
  track: Music,
  photo: Image,
  book: BookOpen,
  comic: BookImage,
};
