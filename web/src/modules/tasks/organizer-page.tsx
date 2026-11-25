import { Button, Input, Spinner, toast } from "@heroui/react";
import { Check, Eye, FolderSync, RotateCcw } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import { useSystemInfo } from "@/api/queries/system";
import { ChangePreviewView } from "@/blocks/renderers/changes";
import { RelativeTime } from "@/components/relative-time";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import {
  useApplyOrganize,
  useOrganizeBatches,
  usePreviewOrganize,
  useUndoOrganizeBatch,
} from "@/provisional/queries/downloads";
import type { OrganizeMode, OrganizePreset, OrganizePreviewResponse } from "@/provisional/types";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

const SELECT_CLASS = "pv-select w-full";

const FALLBACK_PRESETS: OrganizePreset[] = [
  {
    id: "movie-year",
    label: "电影标准（片名 + 年份）",
    mediaKind: "video",
    template: "{title} ({year})",
    example: "沙丘2 (2024).mkv",
  },
  {
    id: "series-season",
    label: "剧集分季（剧名 / Season / SxxExx）",
    mediaKind: "video",
    template: "{title}/Season {season}/{title} - S{season}E{episode}",
    example: "三体/Season 01/三体 - S01E01.mkv",
  },
  {
    id: "music-artist-title",
    label: "音乐曲目（艺人 - 标题）",
    mediaKind: "music",
    template: "{artist} - {title}",
    example: "周杰伦 - 晴天 Live.flac",
  },
  {
    id: "book-author-title",
    label: "电子书标准（作者 - 书名）",
    mediaKind: "book",
    template: "{author} - {title}",
    example: "刘慈欣 - 三体.epub",
  },
];

export function OrganizerPage() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const hasFeature = useFeature("organizer");
  const { data: sysInfo } = useSystemInfo();
  const libsQuery = useLibraries();
  const batchesQuery = useOrganizeBatches(hasFeature && isAdmin);
  const previewMut = usePreviewOrganize();
  const applyMut = useApplyOrganize();
  const undoMut = useUndoOrganizeBatch();

  const libraries = libsQuery.data?.libraries ?? [];
  const presets = batchesQuery.data?.presets?.length ? batchesQuery.data.presets : FALLBACK_PRESETS;
  const batches = batchesQuery.data?.batches ?? [];

  const [selectedLibraryId, setSelectedLibraryId] = useState("");
  const [mode, setMode] = useState<OrganizeMode>("rename");
  const [presetId, setPresetId] = useState(FALLBACK_PRESETS[0]?.id ?? "movie-year");
  const [template, setTemplate] = useState(FALLBACK_PRESETS[0]?.template ?? "{title} ({year})");
  const [previewResult, setPreviewResult] = useState<OrganizePreviewResponse | null>(null);
  const [selectedRows, setSelectedRows] = useState<ReadonlySet<string>>(new Set());

  const libraryId = selectedLibraryId || libraries[0]?.id || "";
  const inset = useRowInset();

  if (!hasFeature) {
    return (
      <PageFrame>
        <EmptyState
          icon={FolderSync}
          title={t("feature.unavailableTitle")}
          detail={t("feature.unavailableDetail", {
            version: sysInfo?.version ?? "0.1.0",
            feature: t("organizerApp.title"),
          })}
        />
      </PageFrame>
    );
  }

  if (!isAdmin) {
    return (
      <PageFrame>
        <EmptyState icon={FolderSync} title={t("feature.adminOnly")} />
      </PageFrame>
    );
  }

  if (batchesQuery.isError) {
    return (
      <PageFrame>
        <ErrorState error={batchesQuery.error} onRetry={() => void batchesQuery.refetch()} />
      </PageFrame>
    );
  }

  const onPresetChange = (nextId: string) => {
    setPresetId(nextId);
    const found = presets.find((entry) => entry.id === nextId);
    if (found) setTemplate(found.template);
  };

  const onRunPreview = (event: FormEvent) => {
    event.preventDefault();
    if (!libraryId || !template.trim()) return;
    previewMut.mutate(
      {
        libraryId,
        mode,
        template: template.trim(),
      },
      {
        onSuccess: (res) => {
          setPreviewResult(res);
          setSelectedRows(new Set(res.candidates.filter((c) => c.selectable).map((c) => c.rowId)));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const toggleRow = (rowId: string) => {
    if (!previewResult) return;
    const candidate = previewResult.candidates.find((c) => c.rowId === rowId);
    if (candidate && !candidate.selectable) return;
    setSelectedRows((prev) => {
      const next = new Set(prev);
      if (next.has(rowId)) next.delete(rowId);
      else next.add(rowId);
      return next;
    });
  };

  const toggleAllRows = (all: boolean) => {
    if (!previewResult) return;
    setSelectedRows(
      all ? new Set(previewResult.candidates.filter((c) => c.selectable).map((c) => c.rowId)) : new Set(),
    );
  };

  const onApplySelected = () => {
    if (!previewResult) return;
    const selectedItemIds = previewResult.candidates
      .filter((c) => c.selectable && selectedRows.has(c.rowId))
      .map((c) => c.itemId);
    if (selectedItemIds.length === 0) return;

    applyMut.mutate(
      {
        libraryId: previewResult.libraryId,
        mode: previewResult.mode,
        template: previewResult.template,
        selectedItemIds,
      },
      {
        onSuccess: (batch) => {
          toast.success(t("organizerApp.appliedToast", { count: batch.operationCount }));
          setPreviewResult(null);
          setSelectedRows(new Set());
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const previewBlock =
    previewResult?.previewBlock.type === "change-preview" ? previewResult.previewBlock : null;

  return (
    <PageFrame className="grid gap-6">
      <PageHeader title={t("organizerApp.title")} summary={t("organizerApp.summary")} />

      <Section title={t("organizerApp.configSection")} bodyClassName="px-(--pv-pad-panel) pb-4">
        <form onSubmit={onRunPreview} className="grid gap-3.5 sm:grid-cols-2">
          <div className="grid gap-1.5">
            <label htmlFor="org-library" className="text-sm font-mid text-fg">
              {t("organizerApp.library")}
            </label>
            <select
              id="org-library"
              value={libraryId}
              onChange={(event) => setSelectedLibraryId(event.target.value)}
              className={SELECT_CLASS}
            >
              {libraries.map((lib) => (
                <option key={lib.id} value={lib.id}>
                  {lib.name}
                </option>
              ))}
            </select>
          </div>

          <div className="grid gap-1.5">
            <label htmlFor="org-mode" className="text-sm font-mid text-fg">
              {t("organizerApp.mode")}
            </label>
            <select
              id="org-mode"
              value={mode}
              onChange={(event) => setMode(event.target.value as OrganizeMode)}
              className={SELECT_CLASS}
            >
              <option value="rename">{t("organizerApp.modes.rename")}</option>
              <option value="hardlink">{t("organizerApp.modes.hardlink")}</option>
            </select>
          </div>

          <div className="grid gap-1.5">
            <label htmlFor="org-preset" className="text-sm font-mid text-fg">
              {t("organizerApp.preset")}
            </label>
            <select
              id="org-preset"
              value={presetId}
              onChange={(event) => onPresetChange(event.target.value)}
              className={SELECT_CLASS}
            >
              {presets.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.label}
                </option>
              ))}
            </select>
          </div>

          <div className="grid gap-1.5">
            <label htmlFor="org-template" className="text-sm font-mid text-fg">
              {t("organizerApp.template")}
            </label>
            <Input
              id="org-template"
              value={template}
              onChange={(event) => setTemplate(event.target.value)}
              className="font-mono text-sm"
            />
          </div>

          <div className="flex justify-end sm:col-span-2">
            <Button
              type="submit"
              size="sm"
              variant="primary"
              isDisabled={!libraryId || !template.trim() || previewMut.isPending}
            >
              {previewMut.isPending ? <Spinner size="sm" color="current" /> : <Eye className="size-4" aria-hidden />}
              {t("organizerApp.previewAction")}
            </Button>
          </div>
        </form>
      </Section>

      {previewResult && previewBlock ? (
        <Section
          title={t("organizerApp.previewSection")}
          count={previewResult.candidates.length}
          actions={
            <Button
              size="sm"
              variant="primary"
              className="button--confirm"
              isDisabled={
                previewResult.readOnlyMount || selectedRows.size === 0 || applyMut.isPending
              }
              onPress={onApplySelected}
            >
              {applyMut.isPending ? <Spinner size="sm" color="current" /> : <Check className="size-4" aria-hidden />}
              {t("organizerApp.applySelected", { count: selectedRows.size })}
            </Button>
          }
          bodyClassName="grid gap-3 px-(--pv-pad-panel) pb-4"
        >
          {previewResult.readOnlyMount ? (
            <p className="rounded-control border border-line bg-sunken px-3 py-2 text-sm text-fg-muted">
              {t("organizerApp.readOnlyBanner")}
            </p>
          ) : null}
          {previewResult.collisionCount > 0 ? (
            <p className="rounded-control border border-line bg-sunken px-3 py-2 text-sm text-fg-muted">
              {t("organizerApp.collisionBanner", { count: previewResult.collisionCount })}
            </p>
          ) : null}

          <ChangePreviewView
            block={previewBlock}
            selected={selectedRows}
            readOnly={previewResult.readOnlyMount}
            onToggle={toggleRow}
            onToggleAll={toggleAllRows}
          />
        </Section>
      ) : null}

      <Section title={t("organizerApp.batchesSection")} count={batches.length}>
        {batchesQuery.isPending ? (
          <div className="px-(--pv-pad-panel) pb-4">
            <LoadingRows rows={3} />
          </div>
        ) : batches.length === 0 ? (
          <div className="px-(--pv-pad-panel) pb-5 text-sm text-fg-muted">
            {t("organizerApp.batchesEmpty")}
          </div>
        ) : (
          <SectionRows>
            {batches.map((batch) => {
              const undone = Boolean(batch.undoneAt);
              return (
                <li
                  key={batch.id}
                  className={cn("flex flex-wrap items-center justify-between gap-3 py-3", inset)}
                >
                  <div className="grid min-w-0 gap-0.5">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-mid text-fg">{batch.libraryName ?? batch.libraryId}</span>
                      <span className="text-xs text-fg-muted">
                        <RelativeTime value={batch.createdAt} />
                      </span>
                      {undone ? (
                        <span className="rounded-control bg-sunken px-2 py-0.5 text-xs text-fg-muted">
                          {t("organizerApp.undoneBadge")}
                        </span>
                      ) : null}
                    </div>
                    <span className="truncate font-mono text-xs text-fg-muted">
                      {t("organizerApp.batchMeta", {
                        mode: t(`organizerApp.modes.${batch.mode}`),
                        count: batch.operationCount,
                        template: batch.template,
                      })}
                    </span>
                  </div>

                  {!undone ? (
                    <Button
                      size="sm"
                      variant="outline"
                      isDisabled={undoMut.isPending}
                      onPress={() =>
                        undoMut.mutate(batch.id, {
                          onSuccess: (reverted) =>
                            toast.success(t("organizerApp.undoneToast", { count: reverted.operationCount })),
                          onError: (error) => toast.danger(errorText(t, error)),
                        })
                      }
                    >
                      <RotateCcw className="size-3.5" aria-hidden />
                      {t("organizerApp.undoAction")}
                    </Button>
                  ) : null}
                </li>
              );
            })}
          </SectionRows>
        )}
      </Section>
    </PageFrame>
  );
}
