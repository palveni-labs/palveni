import { Button, Spinner, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ChevronRight, ScanSearch } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { useScanLibrary } from "@/api/queries/libraries";
import type { Library } from "@/api/types";
import { errorText } from "@/lib/errors";
import { LIBRARY_KIND_META } from "@/lib/kinds";
import { useSlot } from "@/skins/scope";
import { ProgressBar, RunProgress } from "@/ui/progress";
import { useRowInset, useSectionVariant } from "@/ui/section";
import { CoverMosaic } from "@/ui/work";
import { LibraryMeta } from "./library-meta";

/**
 * One library as a row of the 资料 page: what it is (covers where the skin uses work colour, the
 * kind's icon otherwise), how much it holds, when it was scanned. The whole row opens it.
 */
export function LibraryRow({ library, covers, scanning }: { library: Library; covers: string[]; scanning: boolean }) {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const itemColor = useSlot("itemColor");
  const progress = useSlot("progress");
  const variant = useSectionVariant();
  const inset = useRowInset();
  const scan = useScanLibrary();
  const Icon = LIBRARY_KIND_META[library.kind].icon;
  const unscanned = library.lastScanAt === null && library.itemCount === 0;
  const scanLabel = t("libraries.scanNamed", { name: library.name });

  const lead =
    itemColor === "on" ? (
      unscanned ? (
        <span className="grid size-14 shrink-0 place-items-center rounded-tile border-[1.5px] border-dashed border-line-strong text-fg-subtle">
          <Icon className="size-5" aria-hidden />
        </span>
      ) : (
        <CoverMosaic artworkIds={covers} title={library.name} icon={Icon} className={cn("size-14 shrink-0 rounded-tile", covers.length > 0 && "shadow-art")} />
      )
    ) : (
      <span className="grid size-9 shrink-0 place-items-center text-icon">
        <Icon className="size-[1.1rem]" aria-hidden />
      </span>
    );

  return (
    <RunProgress value={null} label={scanLabel} running={scanning}>
      <div
        className={cn(
          "group relative flex min-h-(--pv-row-h) items-center gap-3.5 py-3 transition-colors",
          inset,
          variant !== "rule" && "hover:bg-hover",
        )}
      >
        {lead}
        <Link
          to="/library/$libraryId"
          params={{ libraryId: library.id }}
          className="grid min-w-0 flex-1 gap-0.5 outline-none after:absolute after:inset-0 after:rounded-[inherit] focus-visible:after:outline-2 focus-visible:after:-outline-offset-2 focus-visible:after:outline-(--pv-focus)"
        >
          <span className={cn("truncate text-md font-strong text-fg", variant === "rule" && "group-hover:underline")}>{library.name}</span>
          <LibraryMeta library={library} scanning={scanning} className="truncate text-sm text-fg-muted" />
          {isAdmin && library.paths.length > 0 ? (
            <span className="truncate text-xs text-fg-muted">
              <span className="sr-only">{t("libraries.folders")}: </span>
              {library.paths.map((path, index) => (
                <span key={path}>
                  {index > 0 ? <span aria-hidden> · </span> : null}
                  <span className="font-mono">{path}</span>
                </span>
              ))}
            </span>
          ) : null}
        </Link>
        {scanning && progress === "bar" ? <ProgressBar value={null} label={scanLabel} className="w-20 shrink-0 max-sm:hidden" /> : null}
        {unscanned && isAdmin && !scanning ? (
          <Button
            size="sm"
            variant="outline"
            className="relative z-10 shrink-0"
            isPending={scan.isPending}
            onPress={() =>
              scan.mutate(library.id, {
                onSuccess: () => toast.success(t("adminLibraries.scanStarted", { name: library.name })),
                onError: (error) => toast.danger(errorText(t, error)),
              })
            }
          >
            {scan.isPending ? <Spinner size="sm" color="current" /> : <ScanSearch aria-hidden />}
            <span className="max-sm:sr-only">{t("libraries.scanNow")}</span>
          </Button>
        ) : (
          <ChevronRight className="size-4 shrink-0 text-fg-subtle" aria-hidden />
        )}
      </div>
    </RunProgress>
  );
}
