import { Star } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";

/** Interactive 5-star rating control. Clicking the active star value clears the rating to 0. */
export function StarRating({
  value,
  onChange,
  size = "sm",
  dark = false,
  className,
}: {
  value: number;
  onChange?: (stars: number) => void;
  size?: "xs" | "sm" | "md";
  dark?: boolean;
  className?: string;
}) {
  const { t } = useTranslation();
  const [hovered, setHovered] = useState<number | null>(null);
  const shown = hovered ?? value;
  const iconClass = size === "xs" ? "size-3" : size === "md" ? "size-4" : "size-3.5";

  return (
    <span
      role="group"
      aria-label={t("musicApp.actions.rating")}
      onMouseLeave={() => setHovered(null)}
      className={cn("inline-flex items-center gap-0.5", className)}
    >
      {[1, 2, 3, 4, 5].map((star) => {
        const active = star <= shown;
        const label = value === star ? t("musicApp.actions.clearRating") : t("musicApp.actions.rateStars", { stars: star });
        if (!onChange) {
          return (
            <Star
              key={star}
              aria-hidden
              className={cn(
                iconClass,
                active ? (dark ? "fill-amber-400 text-amber-400" : "fill-degraded text-degraded") : dark ? "text-white/30" : "text-fg-subtle/40",
              )}
            />
          );
        }
        return (
          <button
            key={star}
            type="button"
            aria-label={label}
            aria-pressed={value === star}
            onMouseEnter={() => setHovered(star)}
            onClick={(event) => {
              event.stopPropagation();
              onChange(value === star ? 0 : star);
            }}
            className={cn(
              "grid place-items-center rounded-sm p-0.5 transition-transform hover:scale-110 focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
              dark ? "focus-visible:outline-white" : "",
            )}
          >
            <Star
              aria-hidden
              className={cn(
                iconClass,
                active
                  ? dark
                    ? "fill-amber-400 text-amber-400"
                    : "fill-degraded text-degraded"
                  : dark
                    ? "text-white/35 hover:text-white/75"
                    : "text-fg-subtle/50 hover:text-fg-muted",
              )}
            />
          </button>
        );
      })}
    </span>
  );
}
