import { Link, useParams } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";
import { cn } from "@/lib/cn";
import { useLibraries } from "@/api/queries/libraries";
import { formatNumber } from "@/lib/format";
import { LIBRARY_KIND_META, LIBRARY_KINDS } from "@/lib/kinds";
import { useTranslation } from "react-i18next";
import { useScanningLibraries } from "./use-scanning";

/** The libraries as a list in a sidebar navigation. */
export default function SidebarLibraries() {
  const { i18n } = useTranslation();
  const { data } = useLibraries();
  const params = useParams({ strict: false });
  const scanning = useScanningLibraries();
  const libraries = [...(data?.libraries ?? [])].sort((a, b) => LIBRARY_KINDS.indexOf(a.kind) - LIBRARY_KINDS.indexOf(b.kind) || a.name.localeCompare(b.name));
  return (
    <ul className="grid gap-0.5">
      {libraries.map((library) => {
        const Icon = LIBRARY_KIND_META[library.kind].icon;
        const active = params.libraryId === library.id;
        return (
          <li key={library.id}>
            <Link
              to="/library/$libraryId"
              params={{ libraryId: library.id }}
              aria-current={active ? "page" : undefined}
              className={cn(
                "flex h-[1.875rem] items-center gap-2.5 rounded-tile px-2.5 text-base outline-none focus-visible:ring-2 focus-visible:ring-focus",
                active ? "bg-nav-active text-nav-active-fg" : "text-shell-fg hover:bg-hover",
              )}
            >
              <Icon className={cn("size-4 shrink-0", active ? "text-nav-active-fg" : "text-icon")} aria-hidden />
              <span className="min-w-0 flex-1 truncate">{library.name}</span>
              {scanning.has(library.id) ? <Loader2 className="pv-spin size-3.5 text-active-mark" aria-hidden /> : null}
              <span data-num className={cn("text-sm tabular-nums", active ? "text-nav-active-fg" : "text-shell-fg-muted")}>
                {formatNumber(library.itemCount, i18n.resolvedLanguage ?? "en")}
              </span>
            </Link>
          </li>
        );
      })}
    </ul>
  );
}
