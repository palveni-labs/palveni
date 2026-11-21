import { AlertDialog, Button, Dropdown, Label, Separator, Spinner, toast } from "@heroui/react";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Ellipsis, FolderOpen, FolderPlus, Library as LibraryIcon, ScanSearch, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { jobDetail, scanGaps, scanGapsText } from "@/api/job-text";
import { useDeleteLibrary, useLibraries, useScanLibrary } from "@/api/queries/libraries";
import type { Library } from "@/api/types";
import { RelativeTime } from "@/components/relative-time";
import { errorText } from "@/lib/errors";
import { formatNumber, formatPercent } from "@/lib/format";
import { LIBRARY_KIND_META, LIBRARY_KINDS } from "@/lib/kinds";
import { useLibraryScan } from "@/modules/library/use-library-scan";
import { useItemCountLabel } from "@/modules/library/use-item-count-label";
import { secureOverrides, SecureSurface } from "@/skins/secure";
import { useActiveSkin } from "@/skins/scope";
import { PageFrame } from "@/ui/frame";
import { Measure, StatusLabel } from "@/ui/marks";
import { PageHeader } from "@/ui/page";
import { ProgressBar } from "@/ui/progress";
import { Section, useSectionVariant } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { CreateLibraryDialog } from "./create-library-dialog";

function ScanStatus({ library }: { library: Library }) {
  const { t, i18n } = useTranslation();
  const { job, active, running } = useLibraryScan(library);
  if (running) {
    const progress = active?.progress ?? null;
    const queued = active?.state === "queued";
    const detail = active ? jobDetail(t, active) : null;
    return (
      <div className="grid min-w-0 gap-1.5">
        <div className="flex items-baseline justify-between gap-3 text-sm">
          <StatusLabel tone="active">{queued ? t("adminLibraries.queued") : t("adminLibraries.scanning")}</StatusLabel>
          {progress !== null ? (
            <Measure value={formatPercent(progress, i18n.resolvedLanguage ?? "en")} className="text-fg" />
          ) : (
            <span className="text-fg-muted">{t("tasks.totalUnknown")}</span>
          )}
        </div>
        <ProgressBar value={progress} label={t("adminLibraries.scanNamed", { name: library.name })} />
        {detail ? <span className="truncate text-xs text-fg-muted">{detail}</span> : null}
      </div>
    );
  }
  if (job?.state === "failed") {
    return (
      <div className="grid min-w-0 gap-0.5 text-sm">
        <StatusLabel tone="failed">{t("adminLibraries.scanFailed")}</StatusLabel>
        {job.error ? (
          <span className="truncate text-xs text-fg-muted" title={job.error}>
            {job.error}
          </span>
        ) : null}
      </div>
    );
  }
  // A finished scan that could not read some paths kept their items: say so, with the reason.
  const gaps = job?.state === "succeeded" ? scanGaps(job.step) : null;
  if (gaps && library.lastScanAt) {
    return (
      <div className="grid min-w-0 gap-0.5 text-sm">
        <StatusLabel tone="degraded">{t("jobSteps.incomplete")}</StatusLabel>
        <span className="text-xs text-fg-muted">
          <RelativeTime value={library.lastScanAt} />
          <span aria-hidden> · </span>
          {scanGapsText(t, gaps)}
        </span>
      </div>
    );
  }
  return library.lastScanAt ? (
    <RelativeTime value={library.lastScanAt} className="text-sm text-fg" />
  ) : (
    <StatusLabel tone="idle" className="text-sm">
      {t("libraries.neverScanned")}
    </StatusLabel>
  );
}

function LibraryRow({ library, index, onDelete }: { library: Library; index: number; onDelete: (library: Library) => void }) {
  const { t, i18n } = useTranslation();
  const variant = useSectionVariant();
  const itemCount = useItemCountLabel();
  const scan = useScanLibrary();
  const navigate = useNavigate();
  const { running } = useLibraryScan(library);
  const busy = scan.isPending || running;
  const Icon = LIBRARY_KIND_META[library.kind].icon;
  const edge = variant === "panel" ? "md:first:ps-(--pv-pad-panel) md:last:pe-(--pv-pad-panel)" : variant === "plain" ? "md:first:ps-3 md:last:pe-3" : "";
  const cell = cn("align-top md:py-3.5 md:pe-5", edge);

  return (
    <tr
      data-library-row={library.id}
      className={cn(
        "grid grid-cols-[auto_minmax(0,1fr)_auto] gap-x-2 gap-y-1.5 py-3.5 md:table-row md:py-0",
        variant === "panel" && "max-md:px-(--pv-pad-panel)",
        variant === "plain" ? cn("max-md:px-3", index % 2 === 0 && "bg-sunken") : "border-b border-line last:border-b-0",
      )}
    >
      <th scope="row" className={cn(cell, "col-span-2 row-start-1 min-w-0 text-start font-body")}>
        <div className="flex min-w-0 items-start gap-3">
          <span className="mt-0.5 grid size-8 shrink-0 place-items-center text-icon">
            <Icon className="size-[1.1rem]" aria-hidden />
          </span>
          <div className="grid min-w-0 gap-0.5">
            <span className="flex min-w-0 flex-wrap items-baseline gap-x-2">
              <Link to="/library/$libraryId" params={{ libraryId: library.id }} className="truncate text-md font-strong text-fg hover:underline">
                {library.name}
              </Link>
              <span className="text-sm text-fg-muted">{t(`kinds.${library.kind}`)}</span>
            </span>
            <ul className="grid gap-0.5">
              {library.paths.map((path) => (
                <li key={path} className="font-mono text-sm break-all text-fg-muted">
                  {path}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </th>
      <td className={cn(cell, "col-start-1 row-start-2 ps-11 text-sm whitespace-nowrap md:ps-0 md:text-end")}>
        <span data-num className="text-fg tabular-nums max-md:hidden">
          {formatNumber(library.itemCount, i18n.resolvedLanguage ?? "en")}
        </span>
        <span data-num className="text-fg tabular-nums md:hidden">
          {itemCount(library.itemCount)}
          <span aria-hidden className="text-fg-muted">
            {" "}
            ·
          </span>
        </span>
      </td>
      <td className={cn(cell, "col-span-2 col-start-2 row-start-2 min-w-0 md:w-56")}>
        <ScanStatus library={library} />
      </td>
      <td className={cn(cell, "col-start-3 row-start-1 md:w-px md:whitespace-nowrap")}>
        <div className="flex items-center justify-end gap-1">
          <Button
            variant="outline"
            size="sm"
            isDisabled={busy}
            aria-label={busy ? undefined : t("adminLibraries.scanNamed", { name: library.name })}
            onPress={() =>
              scan.mutate(library.id, {
                onSuccess: () => toast.success(t("adminLibraries.scanStarted", { name: library.name })),
                onError: (error) => toast.danger(errorText(t, error)),
              })
            }
          >
            {busy ? <Spinner size="sm" color="current" /> : <ScanSearch aria-hidden />}
            <span className="max-sm:sr-only">{busy ? t("adminLibraries.scanning") : t("adminLibraries.scan")}</span>
          </Button>
          <Dropdown>
            <Dropdown.Trigger
              aria-label={t("a11y.more")}
              className="grid size-(--pv-control-h-sm) place-items-center rounded-control text-fg-muted outline-none hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
            >
              <Ellipsis className="size-4" aria-hidden />
            </Dropdown.Trigger>
            <Dropdown.Popover placement="bottom end" className="min-w-44">
              <Dropdown.Menu
                aria-label={t("libraries.actionsFor", { name: library.name })}
                onAction={(key) => {
                  if (key === "browse") void navigate({ to: "/library/$libraryId", params: { libraryId: library.id } });
                  else if (key === "delete") onDelete(library);
                }}
              >
                <Dropdown.Item id="browse" textValue={t("adminLibraries.browse")}>
                  <FolderOpen className="size-4 text-fg-muted" aria-hidden />
                  <Label>{t("adminLibraries.browse")}</Label>
                </Dropdown.Item>
                <Separator />
                <Dropdown.Item id="delete" textValue={t("common.delete")} variant="danger">
                  <Trash2 className="size-4" aria-hidden />
                  <Label>{t("common.delete")}</Label>
                </Dropdown.Item>
              </Dropdown.Menu>
            </Dropdown.Popover>
          </Dropdown>
        </div>
      </td>
    </tr>
  );
}

function LibraryTable({ libraries, onDelete }: { libraries: Library[]; onDelete: (library: Library) => void }) {
  const { t } = useTranslation();
  const variant = useSectionVariant();
  const edge = variant === "panel" ? "first:ps-(--pv-pad-panel) last:pe-(--pv-pad-panel)" : variant === "plain" ? "first:ps-3 last:pe-3" : "";
  const head = cn("py-2 pe-5 text-start text-xs font-mid whitespace-nowrap text-fg-muted", edge);
  return (
    <table className="block w-full border-collapse md:table">
      <thead className="max-md:sr-only">
        <tr className={cn(variant !== "plain" && "border-y border-line")}>
          <th scope="col" className={head}>
            {t("adminLibraries.columns.library")}
          </th>
          <th scope="col" className={cn(head, "text-end")}>
            {t("adminLibraries.columns.items")}
          </th>
          <th scope="col" className={head}>
            {t("adminLibraries.columns.lastScan")}
          </th>
          <th scope="col" className={head}>
            <span className="sr-only">{t("adminLibraries.columns.actions")}</span>
          </th>
        </tr>
      </thead>
      <tbody className={cn("block md:table-row-group", variant !== "plain" && "max-md:border-t max-md:border-line")}>
        {libraries.map((library, index) => (
          <LibraryRow key={library.id} library={library} index={index} onDelete={onDelete} />
        ))}
      </tbody>
    </table>
  );
}

/** A destructive confirmation: the surface keeps the host's safe colours if the skin's fail. */
function DeleteLibraryDialog({ library, open, onClose }: { library: Library | null; open: boolean; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  const { skin, mode } = useActiveSkin();
  const remove = useDeleteLibrary();
  const target = library;
  return (
    <AlertDialog.Backdrop isOpen={open} onOpenChange={(next) => (next ? undefined : onClose())} isDismissable={!remove.isPending}>
      <AlertDialog.Container size="sm">
        <AlertDialog.Dialog style={secureOverrides(skin, mode)}>
          <AlertDialog.Header>
            <AlertDialog.Icon status="danger">
              <Trash2 className="size-5" aria-hidden />
            </AlertDialog.Icon>
            <AlertDialog.Heading className="font-heading text-lg font-title text-heading">{t("adminLibraries.delete.title", { name: target?.name ?? "" })}</AlertDialog.Heading>
          </AlertDialog.Header>
          <AlertDialog.Body>
            <SecureSurface className="grid gap-3 text-base text-fg">
              <p>{t("adminLibraries.delete.description")}</p>
              {target ? (
                <>
                  <p className="text-sm text-fg-muted">
                    {t("adminLibraries.delete.removes", { count: target.itemCount, formatted: formatNumber(target.itemCount, i18n.resolvedLanguage ?? "en"), name: target.name })}
                  </p>
                  <div className="grid gap-1 text-sm">
                    <span className="text-fg-muted">{t("adminLibraries.delete.keeps")}</span>
                    <ul className="grid gap-0.5 rounded-tile bg-sunken px-3 py-2">
                      {target.paths.map((path) => (
                        <li key={path} className="font-mono break-all text-fg">
                          {path}
                        </li>
                      ))}
                    </ul>
                  </div>
                </>
              ) : null}
            </SecureSurface>
          </AlertDialog.Body>
          <AlertDialog.Footer>
            <Button variant="secondary" onPress={onClose} isDisabled={remove.isPending}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              isPending={remove.isPending}
              onPress={() => {
                if (!library) return;
                remove.mutate(library.id, {
                  onSuccess: () => {
                    toast.success(t("adminLibraries.delete.done", { name: library.name }));
                    onClose();
                  },
                  onError: (error) => toast.danger(errorText(t, error)),
                });
              }}
            >
              {remove.isPending ? <Spinner size="sm" color="current" /> : <Trash2 aria-hidden />}
              {t("common.delete")}
            </Button>
          </AlertDialog.Footer>
        </AlertDialog.Dialog>
      </AlertDialog.Container>
    </AlertDialog.Backdrop>
  );
}

/** 系统 › 媒体库管理: every library with its folders, size and scan state; create, scan, delete. */
export function AdminLibrariesPage() {
  const { t } = useTranslation();
  const search = useSearch({ from: "/_app/system/libraries" });
  const navigate = useNavigate({ from: "/system/libraries" });
  const itemCount = useItemCountLabel();
  const query = useLibraries();
  // The library stays set after closing so the dialog keeps its text while it animates out.
  const [deleting, setDeleting] = useState<Library | null>(null);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const createOpen = search.create === true;
  const setCreateOpen = (open: boolean) => void navigate({ search: { create: open ? true : undefined }, replace: true });

  const libraries = [...(query.data?.libraries ?? [])].sort(
    (a, b) => LIBRARY_KINDS.indexOf(a.kind) - LIBRARY_KINDS.indexOf(b.kind) || a.name.localeCompare(b.name),
  );
  const total = libraries.reduce((sum, library) => sum + library.itemCount, 0);

  const createButton = (
    <Button variant="primary" size="sm" onPress={() => setCreateOpen(true)}>
      <FolderPlus aria-hidden />
      {t("adminLibraries.create")}
    </Button>
  );

  return (
    <PageFrame>
      <PageHeader
        title={t("nav.manageLibraries")}
        summary={libraries.length > 0 ? t("libraries.summary", { count: libraries.length, items: itemCount(total) }) : undefined}
        actions={createButton}
      />
      {query.isPending ? (
        <LoadingRows rows={4} />
      ) : query.isError ? (
        <ErrorState error={query.error} onRetry={() => void query.refetch()} />
      ) : libraries.length === 0 ? (
        <EmptyState
          icon={LibraryIcon}
          title={t("adminLibraries.empty")}
          detail={t("adminLibraries.emptyDetail")}
          action={
            <Button variant="primary" onPress={() => setCreateOpen(true)}>
              <FolderPlus aria-hidden />
              {t("adminLibraries.create")}
            </Button>
          }
        />
      ) : (
        <Section title={t("nav.libraries")} count={libraries.length}>
          <LibraryTable
            libraries={libraries}
            onDelete={(library) => {
              setDeleting(library);
              setDeleteOpen(true);
            }}
          />
        </Section>
      )}

      <CreateLibraryDialog open={createOpen} onOpenChange={setCreateOpen} />
      <DeleteLibraryDialog library={deleting} open={deleteOpen} onClose={() => setDeleteOpen(false)} />
    </PageFrame>
  );
}
