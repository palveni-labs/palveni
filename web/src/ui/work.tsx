import type { LucideIcon } from "lucide-react";
import { Image as ImageIcon } from "lucide-react";
import { cn } from "@/lib/cn";

/** A cover image, or a calm placeholder with the kind's icon when there is none. */
export function Cover({
  title,
  icon: Icon = ImageIcon,
  className,
  decorative = false,
}: {
  artworkId?: string | null;
  title: string;
  icon?: LucideIcon;
  className?: string;
  /** The title is already shown next to the cover. */
  decorative?: boolean;
}) {
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
