/**
 * Covers and work colour for host components.
 *
 * A region about one specific work asks `useWorkColor` for its colours; they exist only when the
 * active skin's `itemColor` slot is on, the work has a cover, and the caller names the work (the
 * colour never appears without the name). The same cover always yields the same colour.
 */
import type { LucideIcon } from "lucide-react";
import { Image as ImageIcon } from "lucide-react";
import type { CSSProperties } from "react";
import { cn } from "@/lib/cn";
import { useArtwork } from "@/provisional/queries/artwork";
import { useActiveSkin } from "@/skins/scope";
import { renderWorkColor, type WorkColors } from "@/skins/work-color";

export interface WorkColor {
  colors: WorkColors;
  /** Custom properties for the region: --work-field, --work-field-fg, --work-field-sub, --work-wash, --work-bar. */
  style: CSSProperties;
}

export function useWorkColor(artworkId: string | null | undefined, name: string | null | undefined): WorkColor | null {
  const { skin, mode } = useActiveSkin();
  const { data } = useArtwork(skin.slots.itemColor === "on" ? artworkId : null);
  if (skin.slots.itemColor !== "on" || !name?.trim() || !data?.hue) return null;
  const tokens = skin.colors[mode];
  const colors = renderWorkColor(data.hue, skin.work[mode], { fg: tokens.fg, track: tokens["progress-rest"], mode });
  return {
    colors,
    style: {
      "--work-field": colors.field,
      "--work-field-fg": colors.fieldFg,
      "--work-field-sub": colors.fieldSub,
      "--work-wash": colors.wash,
      "--work-bar": colors.bar,
    } as CSSProperties,
  };
}

/** A cover image, or a calm placeholder with the kind's icon when there is none. */
export function Cover({
  artworkId,
  title,
  icon: Icon = ImageIcon,
  className,
  decorative = false,
}: {
  artworkId: string | null | undefined;
  title: string;
  icon?: LucideIcon;
  className?: string;
  /** The title is already shown next to the cover. */
  decorative?: boolean;
}) {
  const { data } = useArtwork(artworkId);
  if (data?.url) {
    return <img src={data.url} alt={decorative ? "" : title} draggable={false} className={cn("block object-cover", className)} />;
  }
  return (
    <span
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : title}
      className={cn("grid place-items-center bg-sunken text-fg-subtle", className)}
    >
      <Icon className="size-[38%] max-h-6 max-w-6" strokeWidth={1.6} aria-hidden />
    </span>
  );
}

/** Up to four covers in a 2×2 square (library thumbnails, previews of a batch). */
export function CoverMosaic({ artworkIds, title, icon, className }: { artworkIds: string[]; title: string; icon?: LucideIcon; className?: string }) {
  const ids = artworkIds.slice(0, 4);
  if (ids.length === 0) return <Cover artworkId={null} title={title} icon={icon} className={className} decorative />;
  return (
    <span className={cn("grid gap-0.5 overflow-hidden bg-sunken", ids.length > 1 ? "grid-cols-2" : "grid-cols-1", className)} aria-hidden>
      {ids.map((id) => (
        <Cover key={id} artworkId={id} title="" decorative className="size-full" />
      ))}
    </span>
  );
}
