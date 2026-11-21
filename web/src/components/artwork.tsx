import { cn } from "@/lib/cn";
import type { Item, LibraryKind } from "@/api/types";
import { ITEM_KIND_ICON } from "@/lib/kinds";
import { Cover } from "@/ui/work";

export function ItemArtwork({
  item,
  artworkId,
  className,
  iconClassName,
  bare = false,
}: {
  item: Pick<Item, "id" | "kind"> & { title?: string };
  /** Kept for callers that still pass it; the tile no longer depends on the library's kind. */
  libraryKind?: LibraryKind;
  artworkId?: string | null;
  className?: string;
  iconClassName?: string;
  /** Only the icon, without the tile (small sizes on rows that have their own ground). */
  bare?: boolean;
}) {
  const Icon = ITEM_KIND_ICON[item.kind];
  if (artworkId) {
    return <Cover artworkId={artworkId} title={item.title ?? ""} icon={Icon} decorative className={className} />;
  }
  return (
    <span aria-hidden className={cn("grid place-items-center text-fg-subtle", !bare && "bg-sunken", className)}>
      <Icon className={iconClassName ?? "size-[28%] max-h-8 max-w-8 min-h-4 min-w-4"} strokeWidth={1.5} />
    </span>
  );
}
