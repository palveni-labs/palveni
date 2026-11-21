import { buttonVariants } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { FolderCog, FolderPlus, Library as LibraryIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useFeature } from "@/modules/access";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import type { Library, LibraryKind } from "@/api/types";
import { LIBRARY_KINDS } from "@/lib/kinds";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { LibraryRow } from "./library-card";
import { useItemCountLabel } from "./use-item-count-label";
import { useScanningLibraries } from "./use-scanning";

function KindSection({ kind, libraries, covers, scanning }: { kind: LibraryKind; libraries: Library[]; covers: Record<string, string[]>; scanning: ReadonlySet<string> }) {
  const { t } = useTranslation();
  const itemCount = useItemCountLabel();
  const total = libraries.reduce((sum, library) => sum + library.itemCount, 0);
  return (
    <Section
      title={t(`kinds.${kind}`)}
      count={libraries.length}
      actions={
        <span data-num className="text-sm text-fg-muted tabular-nums">
          {itemCount(total)}
        </span>
      }
    >
      <SectionRows>
        {libraries.map((library) => (
          <li key={library.id}>
            <LibraryRow library={library} covers={covers[library.id] ?? []} scanning={scanning.has(library.id)} />
          </li>
        ))}
      </SectionRows>
    </Section>
  );
}

/** 资料: every library, grouped by kind; each row opens the library for browsing. */
export function LibrariesPage() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const itemCount = useItemCountLabel();
  const query = useLibraries();
  const subscriptions = useFeature("broadcast");
  const storage = useFeature("connections");
  const scanning = useScanningLibraries();
  const libraries = query.data?.libraries ?? [];
  const groups = LIBRARY_KINDS.map((kind) => ({
    kind,
    libraries: libraries.filter((library) => library.kind === kind).sort((a, b) => a.name.localeCompare(b.name)),
  })).filter((group) => group.libraries.length > 0);
  const total = libraries.reduce((sum, library) => sum + library.itemCount, 0);
  const scanningCount = libraries.filter((library) => scanning.has(library.id)).length;

  const summary =
    libraries.length > 0
      ? [
          t("libraries.summary", { count: libraries.length, items: itemCount(total) }),
          scanningCount > 0 ? t("libraries.summaryScanning", { count: scanningCount }) : null,
        ]
          .filter(Boolean)
          .join(" · ")
      : undefined;

  const actions =
    isAdmin && libraries.length > 0 ? (
      <>
        <Link to="/system/libraries" className={buttonVariants({ variant: "ghost", size: "sm" })}>
          <FolderCog aria-hidden />
          {t("nav.manageLibraries")}
        </Link>
        <Link to="/system/libraries" search={{ create: true }} className={buttonVariants({ variant: "outline", size: "sm" })}>
          <FolderPlus aria-hidden />
          {t("adminLibraries.create")}
        </Link>
      </>
    ) : null;

  return (
    <PageFrame>
      <PageHeader title={t("nav.libraries")} summary={summary} actions={actions} />
      {(subscriptions || storage) && (
        <nav aria-label={t("nav.libraries")} className="flex flex-wrap gap-2">
          {subscriptions && (["radio", "podcast", "iptv"] as const).map((kind) => (
            <Link key={kind} to="/broadcast" search={{ kind }} className={buttonVariants({ variant: "outline", size: "sm" })}>
              {t(`broadcast.${kind}`)}
            </Link>
          ))}
          {storage && <Link to="/library/connections" className={buttonVariants({ variant: "outline", size: "sm" })}>{t("connections.title")}</Link>}
        </nav>
      )}
      {query.isPending ? (
        <LoadingRows rows={4} />
      ) : query.isError ? (
        <ErrorState error={query.error} onRetry={() => void query.refetch()} />
      ) : groups.length === 0 ? (
        <EmptyState
          icon={LibraryIcon}
          title={isAdmin ? t("libraries.emptyAdmin") : t("libraries.emptyUser")}
          detail={isAdmin ? t("libraries.emptyAdminDetail") : t("libraries.emptyUserDetail")}
          action={
            isAdmin ? (
              <Link to="/system/libraries" search={{ create: true }} className={buttonVariants({ variant: "primary" })}>
                <FolderPlus aria-hidden />
                {t("adminLibraries.create")}
              </Link>
            ) : null
          }
        />
      ) : (
        <div className={cn("grid items-start gap-(--pv-gap-section)", groups.length > 1 && "lg:grid-cols-2")}>
          {groups.map((group) => (
            <KindSection key={group.kind} kind={group.kind} libraries={group.libraries} covers={{}} scanning={scanning} />
          ))}
        </div>
      )}
    </PageFrame>
  );
}
