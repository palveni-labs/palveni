import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { useMusicArtworkUrl } from "../artwork-cache";

/**
 * Blurred artwork backdrop hero for album, artist and playlist detail pages.
 * Renders a heavy dark gradient scrim over the blurred artwork so white text always maintains
 * high contrast (> 7:1) regardless of how light the cover image is.
 */
export function MusicHero({
  artworkItemId,
  artwork,
  title,
  subtitle,
  meta,
  actions,
  extra,
  className,
}: {
  artworkItemId?: string | null;
  artwork: ReactNode;
  title: ReactNode;
  subtitle?: ReactNode;
  meta?: ReactNode;
  actions?: ReactNode;
  extra?: ReactNode;
  className?: string;
}) {
  const backdropUrl = useMusicArtworkUrl(artworkItemId, 480, Boolean(artworkItemId));

  return (
    <section
      className={cn(
        "relative overflow-hidden rounded-panel bg-[#14161a] text-white shadow-raised ring-1 ring-white/10",
        className,
      )}
    >
      <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
        {backdropUrl ? (
          <img
            src={backdropUrl}
            alt=""
            className="size-full scale-125 object-cover opacity-65 blur-3xl saturate-140"
          />
        ) : null}
        <div className="absolute inset-0 bg-gradient-to-tr from-[#0e1014]/90 via-[#121418]/74 to-[#121418]/55" />
      </div>

      <div className="relative z-10 flex flex-col gap-5 p-5 sm:flex-row sm:items-end sm:gap-7 sm:p-7">
        <div className="shrink-0 self-center sm:self-end">{artwork}</div>
        <div className="flex min-w-0 flex-1 flex-col gap-2.5 text-center sm:text-start">
          <h1 className="font-heading text-2xl leading-tight font-title text-white sm:text-3xl">
            {title}
          </h1>
          {subtitle ? <div className="text-base font-mid text-white/90">{subtitle}</div> : null}
          {meta ? <div className="flex flex-wrap items-center justify-center gap-x-2.5 gap-y-1 text-sm text-white/80 sm:justify-start">{meta}</div> : null}
          {extra ? <div className="flex flex-wrap items-center justify-center gap-1.5 sm:justify-start">{extra}</div> : null}
          {actions ? <div className="mt-1 flex flex-wrap items-center justify-center gap-2 sm:justify-start">{actions}</div> : null}
        </div>
      </div>
    </section>
  );
}
