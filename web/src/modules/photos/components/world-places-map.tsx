import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { PhotoPlaceCluster } from "../api";

// Simplified offline equirectangular continent polygons (720x360: x = (lon+180)*2, y = (90-lat)*2).
// Zero external tiles or network requests at runtime.
const CONTINENT_PATHS = [
  // North America
  "M 84 44 L 166 42 L 228 62 L 236 96 L 204 128 L 178 148 L 152 134 L 122 104 L 82 72 Z",
  // South America
  "M 206 164 L 248 176 L 266 212 L 244 268 L 222 292 L 210 250 L 198 196 Z",
  // Greenland
  "M 248 26 L 296 24 L 304 52 L 262 60 Z",
  // Europe
  "M 338 56 L 408 48 L 436 74 L 418 106 L 356 108 L 336 86 Z",
  // Africa
  "M 336 116 L 414 116 L 444 156 L 424 238 L 392 252 L 368 202 L 332 158 Z",
  // Asia
  "M 422 48 L 578 44 L 664 68 L 638 124 L 584 158 L 516 162 L 468 136 L 434 104 Z",
  // Japan & Maritime SE Asia
  "M 632 104 L 646 108 L 640 124 L 628 118 Z M 556 168 L 614 176 L 618 194 L 560 190 Z",
  // Australia & New Zealand
  "M 584 216 L 646 216 L 658 256 L 612 268 L 580 246 Z M 684 268 L 698 274 L 690 288 Z",
];

function projectCoord(lat: number, lon: number): { x: number; y: number } {
  const clampedLon = Math.min(180, Math.max(-180, lon));
  const clampedLat = Math.min(90, Math.max(-90, lat));
  return {
    x: Number(((clampedLon + 180) * 2).toFixed(1)),
    y: Number(((90 - clampedLat) * 2).toFixed(1)),
  };
}

export function WorldPlacesMap({
  places,
  selectedKey,
  onSelectPlace,
}: {
  places: readonly PhotoPlaceCluster[];
  selectedKey: string | null;
  onSelectPlace: (key: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="overflow-hidden rounded-panel border border-line bg-sunken">
      <svg
        role="img"
        aria-label={t("photosApp.places.mapLabel")}
        viewBox="0 0 720 360"
        className="block h-auto w-full select-none"
      >
        {/* Graticule grid lines every 60° lon / 30° lat */}
        {[60, 120, 180, 240, 300].map((y) => (
          <line
            key={`lat-${y}`}
            x1={0}
            y1={y}
            x2={720}
            y2={y}
            stroke="currentColor"
            strokeDasharray={y === 180 ? undefined : "3 4"}
            className={y === 180 ? "text-line-strong opacity-65" : "text-line opacity-55"}
          />
        ))}
        {[120, 240, 360, 480, 600].map((x) => (
          <line
            key={`lon-${x}`}
            x1={x}
            y1={0}
            x2={x}
            y2={360}
            stroke="currentColor"
            strokeDasharray={x === 360 ? undefined : "3 4"}
            className={x === 360 ? "text-line-strong opacity-65" : "text-line opacity-55"}
          />
        ))}

        {/* Offline continent silhouettes */}
        {CONTINENT_PATHS.map((d, idx) => (
          <path
            key={idx}
            d={d}
            fill="currentColor"
            stroke="currentColor"
            strokeWidth={1}
            className="text-fg-subtle/22"
          />
        ))}

        {/* Interactive photo cluster pins */}
        {places.map((cluster) => {
          const { x, y } = projectCoord(cluster.latitude, cluster.longitude);
          const active = selectedKey === cluster.key;
          const radius = Math.min(20, Math.round(8 + Math.log2(cluster.count + 1) * 3));
          const pinLabel = t("photosApp.places.clusterPin", {
            label: cluster.label,
            count: cluster.count,
          });
          return (
            <g
              key={cluster.key}
              role="button"
              tabIndex={0}
              aria-label={pinLabel}
              aria-pressed={active}
              onClick={() => onSelectPlace(cluster.key)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelectPlace(cluster.key);
                }
              }}
              className="cursor-pointer outline-none focus-visible:opacity-100"
            >
              <circle
                cx={x}
                cy={y}
                r={active ? radius + 4 : radius}
                className={cn(
                  "transition-all duration-150 motion-reduce:transition-none",
                  active ? "fill-fg stroke-surface" : "fill-fg/80 stroke-surface hover:fill-fg",
                )}
                strokeWidth={2.5}
              />
              <text
                x={x}
                y={y + 3.5}
                textAnchor="middle"
                className="pointer-events-none fill-surface text-[10px] font-bold tabular-nums"
              >
                {cluster.count}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}
