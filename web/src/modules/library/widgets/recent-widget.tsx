import { useNavigate } from "@tanstack/react-router";
import { BookOpen, Clapperboard, Disc3, Image, Tv } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useCatalog } from "@/provisional/queries/catalog";
import type { CatalogWork, WorkKind } from "@/provisional/types";
import { useSlot } from "@/skins/scope";
import { EmptyState, LoadingRows } from "@/ui/states";
import { Cover, useWorkColor } from "@/ui/work";

const KIND_ICON: Record<WorkKind, typeof Clapperboard> = {
  movie: Clapperboard,
  series: Tv,
  album: Disc3,
  book: BookOpen,
  photo: Image,
};

/** Every medium keeps its own proportions; covers stand on one baseline. */
const SHAPE: Record<WorkKind, string> = {
  movie: "w-full aspect-[2/3] rounded-md",
  series: "w-full aspect-[2/3] rounded-md",
  album: "w-[92%] aspect-square rounded-md",
  book: "w-[86%] aspect-[0.7] rounded-[2px_6px_6px_2px]",
  photo: "w-full aspect-[4/3] rounded-md",
};

/** In a tile the stand has a fixed height and each cover keeps its own proportions within it. */
const TILE_SHAPE: Record<WorkKind, string> = {
  movie: "h-[10.25rem] max-w-full aspect-[2/3] rounded-md",
  series: "h-[10.25rem] max-w-full aspect-[2/3] rounded-md",
  album: "w-[90%] max-w-[9.5rem] aspect-square rounded-lg",
  book: "h-[9.75rem] max-w-full aspect-[0.7] rounded-[2px_6px_6px_2px]",
  photo: "w-full aspect-[4/3] rounded-md",
};

function Work({ work }: { work: CatalogWork }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const itemColor = useSlot("itemColor");
  const color = useWorkColor(work.artworkId, work.title);
  const meta = [t(`works.${work.kind}`), work.meta].filter(Boolean).join(" · ");
  const tile = itemColor === "on";
  const centeredMedia = work.kind === "album" || work.kind === "photo";

  const openWork = () => {
    if ((work.kind === "movie" || work.kind === "series") && work.libraryId) {
      void navigate({
        to: "/video/$libraryId/works/$workId",
        params: { libraryId: work.libraryId, workId: work.id },
      });
    } else if (work.kind === "album" && work.libraryId) {
      void navigate({
        to: "/music/albums/$libraryId/$albumId",
        params: { libraryId: work.libraryId, albumId: work.id },
      });
    } else if (work.kind === "book") {
      void navigate({
        to: "/reading/book/$itemId",
        params: { itemId: work.id },
      });
    } else if (work.kind === "photo") {
      void navigate({ to: "/photos" });
    } else {
      void navigate({ to: "/library" });
    }
  };

  return (
    <button
      type="button"
      onClick={openWork}
      className="block h-full w-full min-w-0 text-start outline-none focus-visible:ring-2 focus-visible:ring-focus rounded-2xl"
    >
      <figure
        className={cn(
          "group flex h-full min-w-0 flex-col transition-all duration-200 hover:-translate-y-1",
          tile && "rounded-2xl border border-white/10 px-4 pt-[1.125rem] pb-4 shadow-panel hover:shadow-overlay",
          tile && (color ? "work-field" : "pv-glass-panel"),
        )}
        style={tile && color ? color.style : undefined}
      >
        <div
          className={cn(
            "flex justify-center",
            centeredMedia ? "items-center" : "items-end",
            tile ? "h-[10.75rem]" : "aspect-[2/3]",
          )}
        >
          <Cover
            artworkId={work.artworkId}
            title={work.title}
            icon={KIND_ICON[work.kind]}
            className={cn(
              tile ? TILE_SHAPE[work.kind] : SHAPE[work.kind],
              "shadow-art",
              tile && "outline-1 -outline-offset-1 outline-(--work-field-fg)/15",
            )}
          />
        </div>
        <figcaption className={cn("grid gap-0.5", tile ? "mt-3.5" : "mt-2.5")}>
          <span
            className={cn(
              "truncate font-strong group-hover:underline",
              tile ? "text-md font-title" : "text-base",
              !(tile && color) && "text-fg",
            )}
          >
            {work.title}
          </span>
          <span
            className={cn(
              "truncate text-xs",
              tile && color ? "text-(--work-field-sub)" : "text-fg-muted",
            )}
          >
            {meta}
          </span>
        </figcaption>
      </figure>
    </button>
  );
}

/** 最近入库: the newest works, each in its own colour where the skin uses work colour. */
export default function RecentWidget() {
  const { t } = useTranslation();
  const { data, isPending } = useCatalog(true);
  if (isPending) return <LoadingRows rows={2} className="px-5 pb-4" />;
  const recent = data?.recent ?? [];
  if (recent.length === 0) return <EmptyState compact icon={Image} title={t("recent.empty")} />;
  return (
    <ul
      className={cn(
        "no-scrollbar grid auto-cols-[9.5rem] grid-flow-col gap-3 overflow-x-auto pb-1 md:grid-flow-row md:grid-cols-6 md:overflow-visible",
      )}
    >
      {recent.slice(0, 6).map((work) => (
        <li key={work.id} className="min-w-0">
          <Work work={work} />
        </li>
      ))}
    </ul>
  );
}
