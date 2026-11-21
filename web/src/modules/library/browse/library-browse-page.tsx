import { Button, Dropdown, Label, Spinner, toast, ToggleButton, ToggleButtonGroup, buttonVariants } from "@heroui/react";
import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { ArrowUp, CheckSquare, ChevronRight, Ellipsis, Folder, FolderOpen, LayoutGrid, Library as LibraryIcon, List, RotateCw, ScanSearch, SearchX, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useIsAdmin } from "@/api/queries/auth";
import { itemAncestorsQuery, itemsInfiniteQuery } from "@/api/queries/items";
import { useLibrary, useScanLibrary } from "@/api/queries/libraries";
import type { Item, Library } from "@/api/types";
import { errorText } from "@/lib/errors";
import { formatPercent } from "@/lib/format";
import { readLocal, writeLocal } from "@/lib/storage";
import { useSlot } from "@/skins/scope";
import { useFeature } from "@/modules/access";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { LibraryMeta } from "../library-meta";
import { useLibraryScan } from "../use-library-scan";
import { ItemDetailsSheet } from "./item-details-sheet";
import { MusicBrowse } from "./music-browse";
import { SpokenBrowse } from "./spoken-browse";
import { VideoBrowse } from "./video-browse";
import { ItemGrid, ItemList } from "./item-views";

type ViewMode = "grid" | "list";
const VIEW_KEY = "palveni.browse.view";

function useViewMode(): [ViewMode, (mode: ViewMode) => void] {
  const [mode, setMode] = useState<ViewMode>(() => (readLocal(VIEW_KEY) === "list" ? "list" : "grid"));
  const update = useCallback((next: ViewMode) => {
    setMode(next);
    writeLocal(VIEW_KEY, next);
  }, []);
  return [mode, update];
}

/** Breadcrumbs from the library root to the open folder; long trails fold the middle into a menu. */
function FolderTrail({ library, parentId, keepFileView }: { library: Library; parentId: string | null; keepFileView: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const navigate = useNavigate({ from: "/library/$libraryId" });
  const { data: chain } = useQuery({ ...itemAncestorsQuery(queryClient, parentId ?? ""), enabled: parentId !== null });
  const folders = parentId ? (chain ?? []) : [];
  const hidden = folders.length > 3 ? folders.slice(0, -2) : [];
  const visible = folders.length > 3 ? folders.slice(-2) : folders;
  const crumb = "truncate rounded-control px-1.5 py-1 text-fg-muted transition-colors hover:bg-hover hover:text-fg";
  const separator = <ChevronRight className="size-3.5 shrink-0 text-fg-subtle" aria-hidden />;

  return (
    <nav aria-label={t("a11y.breadcrumb")} className="min-w-0 flex-1">
      <ol className="flex min-w-0 items-center gap-0.5 text-sm">
        <li className="flex min-w-0 shrink-0 items-center">
          {folders.length === 0 ? (
            <span aria-current="page" className="flex items-center gap-1.5 px-1.5 py-1 font-strong text-fg">
              <FolderOpen className="size-4 shrink-0 text-icon" aria-hidden />
              {t("browse.root")}
            </span>
          ) : (
            <Link to="/library/$libraryId" params={{ libraryId: library.id }} search={keepFileView ? { view: "files" } : {}} className={cn(crumb, "flex items-center gap-1.5")}>
              <FolderOpen className="size-4 shrink-0" aria-hidden />
              {t("browse.root")}
            </Link>
          )}
        </li>
        {hidden.length > 0 ? (
          <li className="flex shrink-0 items-center gap-0.5">
            {separator}
            <Dropdown>
              <Dropdown.Trigger aria-label={t("browse.hiddenFolders")} className="grid size-7 place-items-center rounded-control text-fg-muted outline-none hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)">
                <Ellipsis className="size-4" aria-hidden />
              </Dropdown.Trigger>
              <Dropdown.Popover placement="bottom start">
                <Dropdown.Menu aria-label={t("browse.hiddenFolders")} onAction={(key) => void navigate({ search: { parent: String(key) } })}>
                  {hidden.map((folder) => (
                    <Dropdown.Item key={folder.id} id={folder.id} textValue={folder.title}>
                      <Folder className="size-4 text-fg-muted" aria-hidden />
                      <Label>{folder.title}</Label>
                    </Dropdown.Item>
                  ))}
                </Dropdown.Menu>
              </Dropdown.Popover>
            </Dropdown>
          </li>
        ) : null}
        {visible.map((folder, index) => (
          <li key={folder.id} className="flex min-w-0 items-center gap-0.5">
            {separator}
            {index === visible.length - 1 ? (
              <span aria-current="page" className="truncate px-1.5 py-1 font-strong text-fg">
                {folder.title}
              </span>
            ) : (
              <Link to="/library/$libraryId" params={{ libraryId: library.id }} search={{ parent: folder.id }} className={crumb}>
                {folder.title}
              </Link>
            )}
          </li>
        ))}
      </ol>
    </nav>
  );
}

function ScanControl({ library, size = "sm" }: { library: Library; size?: "sm" | "md" }) {
  const { t, i18n } = useTranslation();
  const scan = useScanLibrary();
  const { active, running } = useLibraryScan(library);
  const progress = active?.progress ?? null;
  const queued = active?.state === "queued";
  const busy = scan.isPending || running;
  const label = running ? (queued ? t("adminLibraries.queued") : t("adminLibraries.scanning")) : t("adminLibraries.scan");
  return (
    <Button
      variant={size === "md" ? "primary" : "outline"}
      size={size}
      isDisabled={busy}
      onPress={() =>
        scan.mutate(library.id, {
          onSuccess: () => toast.success(t("adminLibraries.scanStarted", { name: library.name })),
          onError: (error) => toast.danger(errorText(t, error)),
        })
      }
    >
      {busy ? <Spinner size="sm" color="current" /> : <ScanSearch aria-hidden />}
      {label}
      {running && progress !== null ? (
        <span data-num className="tabular-nums">
          {formatPercent(progress, i18n.resolvedLanguage ?? "en")}
        </span>
      ) : null}
    </Button>
  );
}

function SelectionBar({
  picked,
  loaded,
  onSelectLoaded,
  onDone,
}: {
  picked: ReadonlySet<string>;
  loaded: number;
  onSelectLoaded: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const navPhone = useSlot("navPhone");
  return (
    <div
      className={cn(
        "pointer-events-none sticky bottom-5 z-20 flex justify-center px-2",
        navPhone === "bottom-bar" && "max-md:bottom-[calc(4.75rem+env(safe-area-inset-bottom))]",
      )}
    >
      <div
        role="toolbar"
        aria-label={t("browse.selectionTools")}
        className="pointer-events-auto flex max-w-full flex-wrap items-center gap-x-2 gap-y-1.5 rounded-panel border border-line bg-raised p-1.5 ps-4 shadow-raised"
      >
        <span aria-live="polite" className="me-1 text-sm font-strong text-fg tabular-nums">
          {picked.size > 0 ? t("browse.selected", { count: picked.size }) : <span className="font-body text-fg-muted">{t("browse.selectHint")}</span>}
        </span>
        <Button variant="ghost" size="sm" isDisabled={loaded === 0 || picked.size === loaded} onPress={onSelectLoaded}>
          {t("browse.selectLoaded")}
        </Button>
        <Button variant="ghost" size="sm" isIconOnly onPress={onDone} aria-label={t("browse.cancelSelect")}>
          <X aria-hidden />
        </Button>
      </div>
    </div>
  );
}

function GridSkeleton() {
  const { t } = useTranslation();
  return <LoadingRows rows={6} label={t("common.loading")} />;
}

function BrowseBody({ library, parentId, keepFileView }: { library: Library; parentId: string | null; keepFileView: boolean }) {
  const { t } = useTranslation();
  const navigate = useNavigate({ from: "/library/$libraryId" });
  const isAdmin = useIsAdmin();
  const sectionSlot = useSlot("section");
  const queryClient = useQueryClient();
  const itemsQuery = useInfiniteQuery(itemsInfiniteQuery(library.id, parentId));
  const { data: chain } = useQuery({ ...itemAncestorsQuery(queryClient, parentId ?? ""), enabled: parentId !== null });
  const [view, setView] = useViewMode();
  const [selected, setSelected] = useState<Item | null>(null);
  const [picking, setPicking] = useState<Set<string> | null>(null);
  const { running: scanning } = useLibraryScan(library);

  const items = itemsQuery.data?.pages.flatMap((page) => page.items) ?? [];
  const { fetchNextPage } = itemsQuery;
  const loadMore = useCallback(() => void fetchNextPage(), [fetchNextPage]);

  const open = useCallback(
    (item: Item) => {
      if (picking) {
        setPicking((current) => {
          const next = new Set(current);
          if (next.has(item.id)) next.delete(item.id);
          else next.add(item.id);
          return next;
        });
      } else if (item.kind === "folder") void navigate({ search: { parent: item.id } });
      else setSelected(item);
    },
    [navigate, picking],
  );

  // Escape leaves selection mode (dialogs handle their own Escape first).
  useEffect(() => {
    if (!picking) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) setPicking(null);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [picking]);

  const parentOfCurrent = chain && chain.length > 1 ? (chain[chain.length - 2]?.id ?? null) : null;
  const empty = itemsQuery.isSuccess && items.length === 0;
  const variant = view === "grid" && sectionSlot === "panel" ? "plain" : sectionSlot;

  let content;
  if (itemsQuery.isPending) content = <GridSkeleton />;
  else if (itemsQuery.isError && items.length === 0) content = <ErrorState error={itemsQuery.error} onRetry={() => void itemsQuery.refetch()} />;
  else if (empty) {
    content = parentId ? (
      <EmptyState
        icon={FolderOpen}
        title={t("browse.emptyFolder")}
        action={
          <Link
            to="/library/$libraryId"
            params={{ libraryId: library.id }}
            search={parentOfCurrent ? { parent: parentOfCurrent } : keepFileView ? { view: "files" } : {}}
            className={buttonVariants({ variant: "outline", size: "sm" })}
          >
            <ArrowUp aria-hidden />
            {t("browse.up")}
          </Link>
        }
      />
    ) : library.lastScanAt === null ? (
      <EmptyState
        icon={ScanSearch}
        title={scanning ? t("libraries.scanning") : isAdmin ? t("browse.notScannedAdmin") : t("browse.notScannedUser")}
        detail={isAdmin ? t("browse.notScannedDetailAdmin") : t("browse.notScannedDetailUser")}
        action={isAdmin ? <ScanControl library={library} size="md" /> : null}
      />
    ) : (
      <EmptyState
        icon={SearchX}
        title={t("browse.emptyLibrary")}
        detail={isAdmin ? t("browse.emptyLibraryDetail") : undefined}
        action={
          isAdmin ? (
            <Link to="/system/libraries" className={buttonVariants({ variant: "outline", size: "sm" })}>
              {t("nav.manageLibraries")}
            </Link>
          ) : null
        }
      />
    );
  } else {
    const shared = {
      items,
      libraryKind: library.kind,
      hasNextPage: itemsQuery.hasNextPage,
      isFetchingNextPage: itemsQuery.isFetchingNextPage,
      fetchNextPage: loadMore,
      onOpen: open,
      selection: picking ?? undefined,
    };
    content = view === "grid" ? <ItemGrid {...shared} /> : <ItemList {...shared} />;
  }

  return (
    <>
      <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
        <FolderTrail library={library} parentId={parentId} keepFileView={keepFileView} />
        <div className="flex items-center gap-2">
          <ToggleButton
            size="sm"
            isSelected={picking !== null}
            isDisabled={empty || itemsQuery.isPending}
            onChange={(on) => setPicking(on ? new Set() : null)}
            className="rounded-control"
          >
            <CheckSquare aria-hidden />
            {t("browse.select")}
          </ToggleButton>
          <ToggleButtonGroup
            aria-label={t("browse.view")}
            selectionMode="single"
            disallowEmptySelection
            selectedKeys={[view]}
            onSelectionChange={(keys) => {
              const next = [...keys][0];
              if (next === "grid" || next === "list") setView(next);
            }}
            className="pv-segmented"
          >
            <ToggleButton id="grid" aria-label={t("browse.grid")}>
              <LayoutGrid aria-hidden />
            </ToggleButton>
            <ToggleButton id="list" aria-label={t("browse.list")}>
              <List aria-hidden />
            </ToggleButton>
          </ToggleButtonGroup>
        </div>
      </div>

      <Section title={t("browse.files")} bare unframed={view === "grid"} bodyClassName={cn(variant === "rule" && view === "grid" && "pt-5")}>
        {content}
      </Section>

      {itemsQuery.isFetchingNextPage ? (
        <p role="status" className="flex items-center justify-center gap-2 text-sm text-fg-muted">
          <Spinner size="sm" color="current" />
          {t("browse.loadingMore")}
        </p>
      ) : null}
      {itemsQuery.isFetchNextPageError ? (
        <div role="alert" className="flex flex-wrap items-center justify-center gap-3 text-sm text-fg-muted">
          <span>{t("browse.loadMoreFailed", { error: errorText(t, itemsQuery.error) })}</span>
          <Button variant="outline" size="sm" onPress={loadMore}>
            <RotateCw aria-hidden />
            {t("common.retry")}
          </Button>
        </div>
      ) : null}

      {picking ? (
        <SelectionBar
          picked={picking}
          loaded={items.length}
          onSelectLoaded={() => setPicking(new Set(items.map((item) => item.id)))}
          onDone={() => setPicking(null)}
        />
      ) : null}

      <ItemDetailsSheet item={selected} library={library} onClose={() => setSelected(null)} />
    </>
  );
}

export function LibraryBrowsePage() {
  const { t } = useTranslation();
  const { libraryId } = useParams({ from: "/_app/library/$libraryId" });
  const musicEnabled = useFeature("music");
  const videoEnabled = useFeature("video");
  const spokenEnabled = useFeature("spoken");
  const { parent, view } = useSearch({ from: "/_app/library/$libraryId" });
  const libraryQuery = useLibrary(libraryId);
  const library = libraryQuery.data;

  if (!library) {
    return (
      <PageFrame>
        {libraryQuery.isPending ? (
          <LoadingRows rows={4} />
        ) : (
          <EmptyState
            icon={LibraryIcon}
            title={t("browse.notFound")}
            detail={t("browse.notFoundDetail")}
            action={
              <Link to="/library" className={buttonVariants({ variant: "outline", size: "sm" })}>
                {t("browse.backToLibraries")}
              </Link>
            }
          />
        )}
      </PageFrame>
    );
  }

  const spokenKind = spokenEnabled && (library.kind === "audiobook" || library.kind === "podcast") ? library.kind : undefined;
  const isVideo = videoEnabled && library.kind === "video";
  const organized = !parent && view !== "files" && (isVideo || !!spokenKind);
  const collectionView = isVideo ? "works" : "spoken";
  return (
    <PageFrame>
      <BrowseHeader library={library} />
      {(isVideo || spokenKind) && (
        <nav aria-label={t("browse.view")} className="flex flex-wrap gap-2">
          <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: collectionView }} aria-current={organized ? "page" : undefined} className={buttonVariants({ variant: organized ? "secondary" : "ghost", size: "sm" })}>
            {isVideo ? t("browse.works") : spokenKind === "audiobook" ? t("spoken.shelf") : t("spoken.shows")}
          </Link>
          <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "files" }} aria-current={!organized ? "page" : undefined} className={buttonVariants({ variant: !organized ? "secondary" : "ghost", size: "sm" })}>{t("browse.files")}</Link>
        </nav>
      )}
      {musicEnabled && library.kind === "music" && (
        <nav aria-label={t("music.browse")} className="flex flex-wrap gap-2">
          <Link to="/library/$libraryId" params={{ libraryId }} search={{}} aria-current={view !== "music" ? "page" : undefined} className={buttonVariants({ variant: view !== "music" ? "secondary" : "ghost", size: "sm" })}>{t("browse.files")}</Link>
          <Link to="/library/$libraryId" params={{ libraryId }} search={{ view: "music" }} aria-current={view === "music" ? "page" : undefined} className={buttonVariants({ variant: view === "music" ? "secondary" : "ghost", size: "sm" })}>{t("music.artists")}</Link>
        </nav>
      )}
      {organized && isVideo ? <VideoBrowse libraryId={libraryId} />
        : organized && spokenKind ? <SpokenBrowse libraryId={libraryId} kind={spokenKind} />
        : view === "music" && musicEnabled && library.kind === "music"
        ? <MusicBrowse libraryId={libraryId} />
        : <BrowseBody key={`${library.id}:${parent ?? ""}`} library={library} parentId={parent ?? null} keepFileView={isVideo || !!spokenKind} />}
    </PageFrame>
  );
}

function BrowseHeader({ library }: { library: Library }) {
  const isAdmin = useIsAdmin();
  const { running } = useLibraryScan(library);
  // A library that was never scanned offers the scan as the page's next step instead.
  const scan = isAdmin && (library.lastScanAt !== null || running) ? <ScanControl library={library} /> : null;
  return <PageHeader title={library.name} summary={<LibraryMeta library={library} scanning={running} />} actions={scan} />;
}
