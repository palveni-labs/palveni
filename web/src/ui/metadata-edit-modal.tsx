import { Button, Chip, Input, Spinner, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, PencilLine, RotateCcw, Search, Sparkles, Star, X } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import { queryKeys } from "@/api/query-keys";
import type {
  MetadataOverride,
  MetadataProvider,
  MetadataSearchCandidate,
  MetadataSearchMediaType,
} from "@/api/types";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { FieldTextarea, Select } from "@/ui/controls";

export interface MetadataEditInitialValues {
  title: string;
  originalTitle?: string | null;
  year?: number | null;
  rating?: number | null;
  genres?: string[];
  overview?: string | null;
  posterUrl?: string | null;
  backdropUrl?: string | null;
}

export interface MetadataEditModalProps {
  isOpen: boolean;
  onClose: () => void;
  itemId: string;
  workKey?: string | null;
  defaultMediaType?: MetadataSearchMediaType;
  initialValues: MetadataEditInitialValues;
  onSaved?: (saved: MetadataOverride) => void;
  onReset?: () => void;
}

const ONLINE_PROVIDERS = ["tmdb", "bangumi", "openlibrary"] as const;
const MEDIA_TYPES: MetadataSearchMediaType[] = ["movie", "tv", "anime", "book"];

interface FormDraft {
  provider: MetadataProvider;
  externalId: string;
  title: string;
  originalTitle: string;
  year: string;
  rating: string;
  genres: string;
  overview: string;
  posterUrl: string;
  backdropUrl: string;
}

function buildDraft(
  initial: MetadataEditInitialValues,
  existing: MetadataOverride | null | undefined,
): FormDraft {
  return {
    provider: existing?.provider ?? "manual",
    externalId: existing?.externalId ?? "",
    title: existing?.title ?? initial.title ?? "",
    originalTitle: existing?.originalTitle ?? initial.originalTitle ?? "",
    year:
      existing?.year != null
        ? String(existing.year)
        : initial.year != null
          ? String(initial.year)
          : "",
    rating:
      existing?.rating != null
        ? String(existing.rating)
        : initial.rating != null
          ? String(initial.rating)
          : "",
    genres: (existing?.genres?.length ? existing.genres : initial.genres ?? []).join(", "),
    overview: existing?.overview ?? initial.overview ?? "",
    posterUrl: existing?.posterUrl ?? initial.posterUrl ?? "",
    backdropUrl: existing?.backdropUrl ?? initial.backdropUrl ?? "",
  };
}

function MetadataEditDialogBody({
  onClose,
  itemId,
  workKey,
  defaultMediaType,
  initialValues,
  existingOverride,
  onSaved,
  onReset,
}: {
  onClose: () => void;
  itemId: string;
  workKey?: string | null;
  defaultMediaType: MetadataSearchMediaType;
  initialValues: MetadataEditInitialValues;
  existingOverride: MetadataOverride | null;
  onSaved?: (saved: MetadataOverride) => void;
  onReset?: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [tab, setTab] = useState<"scrape" | "manual">("scrape");
  const [provider, setProvider] = useState<(typeof ONLINE_PROVIDERS)[number]>(
    defaultMediaType === "book" ? "openlibrary" : "tmdb",
  );
  const [mediaType, setMediaType] = useState<MetadataSearchMediaType>(defaultMediaType);
  const [query, setQuery] = useState(initialValues.title);
  const [searchYear, setSearchYear] = useState(
    initialValues.year != null ? String(initialValues.year) : "",
  );
  const [candidates, setCandidates] = useState<MetadataSearchCandidate[] | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [draft, setDraft] = useState<FormDraft>(() => buildDraft(initialValues, existingOverride));

  const searchMutation = useMutation({
    mutationFn: () => {
      const parsedYear = searchYear.trim() ? Number.parseInt(searchYear.trim(), 10) : undefined;
      return api.metadata.search({
        provider,
        query: query.trim(),
        mediaType,
        year: parsedYear && Number.isFinite(parsedYear) ? parsedYear : undefined,
      });
    },
    onSuccess: (res) => {
      setSearchError(null);
      setCandidates(res.candidates);
    },
    onError: (err) => {
      setSearchError(errorText(t, err));
      setCandidates(null);
    },
  });

  const saveMutation = useMutation({
    mutationFn: () => {
      const parsedYear = draft.year.trim() ? Number.parseInt(draft.year.trim(), 10) : null;
      const parsedRating = draft.rating.trim() ? Number.parseFloat(draft.rating.trim()) : null;
      const genres = draft.genres
        .split(/[,，]/)
        .map((g) => g.trim())
        .filter(Boolean);
      return api.metadata.saveOverride(itemId, {
        workKey: workKey ?? null,
        provider: draft.provider,
        externalId: draft.externalId.trim() || null,
        title: draft.title.trim() || null,
        originalTitle: draft.originalTitle.trim() || null,
        year: parsedYear != null && Number.isFinite(parsedYear) ? parsedYear : null,
        rating: parsedRating != null && Number.isFinite(parsedRating) ? parsedRating : null,
        genres,
        overview: draft.overview.trim() || null,
        posterUrl: draft.posterUrl.trim() || null,
        backdropUrl: draft.backdropUrl.trim() || null,
      });
    },
    onSuccess: (saved) => {
      toast.success(t("metadataModal.savedToast"));
      void queryClient.invalidateQueries({ queryKey: queryKeys.metadata.override(itemId) });
      onSaved?.(saved);
      onClose();
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const resetMutation = useMutation({
    mutationFn: () => api.metadata.deleteOverride(itemId),
    onSuccess: () => {
      toast.success(t("metadataModal.resetToast"));
      void queryClient.invalidateQueries({ queryKey: queryKeys.metadata.override(itemId) });
      onReset?.();
      onClose();
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  const handleApplyCandidate = (candidate: MetadataSearchCandidate) => {
    setDraft({
      provider: candidate.provider,
      externalId: candidate.externalId,
      title: candidate.title,
      originalTitle: candidate.originalTitle ?? "",
      year: candidate.year != null ? String(candidate.year) : "",
      rating: candidate.rating != null ? String(candidate.rating) : "",
      genres: candidate.genres.join(", "),
      overview: candidate.overview ?? "",
      posterUrl: candidate.posterUrl ?? "",
      backdropUrl: candidate.backdropUrl ?? "",
    });
    setTab("manual");
    toast.success(t("metadataModal.appliedToast", { title: candidate.title }));
  };

  const handleSearchSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;
    searchMutation.mutate();
  };

  const handleSaveSubmit = (e: FormEvent) => {
    e.preventDefault();
    saveMutation.mutate();
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("metadataModal.dialogTitle")}
      className="flex max-h-[88vh] w-full max-w-2xl flex-col overflow-hidden rounded-panel border border-line bg-surface text-fg shadow-overlay"
    >
      {/* Header */}
      <div className="flex items-center justify-between gap-3 border-b border-line px-5 py-3.5">
        <div className="flex items-center gap-2">
          <Sparkles className="size-4 text-fg-muted" aria-hidden />
          <h2 className="font-heading text-lg font-strong text-heading">
            {t("metadataModal.dialogTitle")}
          </h2>
        </div>
        <button
          type="button"
          aria-label={t("a11y.close")}
          onClick={onClose}
          className="grid size-8 place-items-center rounded-control text-fg-muted hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        >
          <X className="size-4" aria-hidden />
        </button>
      </div>

      {/* Tabs */}
      <div role="tablist" className="flex items-center gap-2 border-b border-line bg-sunken/60 px-5 py-2">
        <button
          type="button"
          role="tab"
          aria-selected={tab === "scrape"}
          onClick={() => setTab("scrape")}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-control px-3 py-1.5 text-sm font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
            tab === "scrape"
              ? "bg-surface text-heading shadow-xs ring-1 ring-line font-strong"
              : "text-fg-muted hover:text-fg",
          )}
        >
          <Search className="size-3.5" aria-hidden />
          {t("metadataModal.tabScrape")}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === "manual"}
          onClick={() => setTab("manual")}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-control px-3 py-1.5 text-sm font-mid transition-colors focus-visible:outline-2 focus-visible:outline-(--pv-focus)",
            tab === "manual"
              ? "bg-surface text-heading shadow-xs ring-1 ring-line font-strong"
              : "text-fg-muted hover:text-fg",
          )}
        >
          <PencilLine className="size-3.5" aria-hidden />
          {t("metadataModal.tabManual")}
        </button>
        {existingOverride ? (
          <Chip size="sm" variant="soft" className="ms-auto bg-ok-soft text-ok">
            {t("metadataModal.overrideActive", { provider: existingOverride.provider.toUpperCase() })}
          </Chip>
        ) : null}
      </div>

      {/* Content */}
      <div className="min-h-0 flex-1 overflow-y-auto p-5">
        {tab === "scrape" ? (
          <div className="grid gap-4">
            <form onSubmit={handleSearchSubmit} className="grid gap-3">
              <div className="flex flex-wrap items-center gap-2.5">
                <label className="grid gap-1 text-xs font-mid text-fg-muted">
                  <span>{t("metadataModal.providerLabel")}</span>
                  <Select
                    aria-label={t("metadataModal.providerLabel")}
                    value={provider}
                    onChange={(e) => setProvider(e.target.value as (typeof ONLINE_PROVIDERS)[number])}
                  >
                    {ONLINE_PROVIDERS.map((p) => (
                      <option key={p} value={p}>
                        {t(`metadataModal.providers.${p}`)}
                      </option>
                    ))}
                  </Select>
                </label>

                <label className="grid gap-1 text-xs font-mid text-fg-muted">
                  <span>{t("metadataModal.mediaTypeLabel")}</span>
                  <Select
                    aria-label={t("metadataModal.mediaTypeLabel")}
                    value={mediaType}
                    onChange={(e) => setMediaType(e.target.value as MetadataSearchMediaType)}
                  >
                    {MEDIA_TYPES.map((m) => (
                      <option key={m} value={m}>
                        {t(`metadataModal.mediaTypes.${m}`)}
                      </option>
                    ))}
                  </Select>
                </label>

                <label className="grid min-w-44 flex-1 gap-1 text-xs font-mid text-fg-muted">
                  <span>{t("metadataModal.queryLabel")}</span>
                  <Input
                    aria-label={t("metadataModal.queryLabel")}
                    placeholder={t("metadataModal.queryPlaceholder")}
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                  />
                </label>

                <label className="grid w-24 gap-1 text-xs font-mid text-fg-muted">
                  <span>{t("metadataModal.yearLabel")}</span>
                  <Input
                    aria-label={t("metadataModal.yearLabel")}
                    placeholder="2024"
                    maxLength={4}
                    value={searchYear}
                    onChange={(e) => setSearchYear(e.target.value)}
                  />
                </label>
              </div>

              <div className="flex justify-end">
                <Button
                  type="submit"
                  size="sm"
                  variant="primary"
                  isDisabled={!query.trim()}
                  isPending={searchMutation.isPending}
                >
                  {searchMutation.isPending ? (
                    <Spinner size="sm" color="current" />
                  ) : (
                    <Search className="size-3.5" aria-hidden />
                  )}
                  {searchMutation.isPending ? t("metadataModal.searching") : t("metadataModal.searchButton")}
                </Button>
              </div>
            </form>

            {searchError ? (
              <div
                role="alert"
                className="rounded-tile border border-failed/30 bg-sunken px-3.5 py-2.5 text-sm text-failed"
              >
                {searchError}
              </div>
            ) : null}

            {candidates !== null ? (
              candidates.length === 0 ? (
                <p className="rounded-tile bg-sunken px-4 py-6 text-center text-sm text-fg-muted">
                  {t("metadataModal.noResults")}
                </p>
              ) : (
                <ul className="grid gap-3">
                  {candidates.map((cand) => (
                    <li
                      key={`${cand.provider}:${cand.externalId}`}
                      className="flex flex-col gap-3 rounded-tile border border-line bg-sunken/50 p-3.5 sm:flex-row sm:items-start"
                    >
                      <div className="grid min-w-0 flex-1 gap-1.5">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="font-strong text-fg">{cand.title}</span>
                          {cand.year != null ? (
                            <span data-num className="text-xs text-fg-muted tabular-nums">
                              {cand.year}
                            </span>
                          ) : null}
                          {cand.rating != null ? (
                            <span
                              data-num
                              className="inline-flex items-center gap-0.5 text-xs font-strong text-fg tabular-nums"
                            >
                              <Star className="size-3 fill-amber-400 text-amber-400" aria-hidden />
                              {cand.rating.toFixed(1)}
                            </span>
                          ) : null}
                          <Chip size="sm" variant="soft" className="bg-surface text-fg-muted">
                            {cand.provider.toUpperCase()} · {cand.externalId}
                          </Chip>
                        </div>
                        {cand.originalTitle ? (
                          <p className="text-xs text-fg-muted">{cand.originalTitle}</p>
                        ) : null}
                        {cand.genres.length > 0 ? (
                          <div className="flex flex-wrap gap-1">
                            {cand.genres.map((g) => (
                              <span
                                key={g}
                                className="rounded-chip bg-surface px-2 py-0.5 text-xs text-fg-muted ring-1 ring-line"
                              >
                                {g}
                              </span>
                            ))}
                          </div>
                        ) : null}
                        {cand.overview ? (
                          <p className="line-clamp-3 text-xs leading-relaxed text-fg-muted">
                            {cand.overview}
                          </p>
                        ) : null}
                      </div>

                      <Button
                        size="sm"
                        variant="outline"
                        className="shrink-0"
                        onPress={() => handleApplyCandidate(cand)}
                      >
                        <Check className="size-3.5" aria-hidden />
                        {t("metadataModal.applyCandidate")}
                      </Button>
                    </li>
                  ))}
                </ul>
              )
            ) : null}
          </div>
        ) : (
          <form id="metadata-override-form" onSubmit={handleSaveSubmit} className="grid gap-3.5">
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="grid gap-1 text-xs font-mid text-fg-muted">
                <span>{t("metadataModal.fieldTitle")}</span>
                <Input
                  aria-label={t("metadataModal.fieldTitle")}
                  maxLength={255}
                  value={draft.title}
                  onChange={(e) => setDraft((d) => ({ ...d, title: e.target.value }))}
                />
              </label>

              <label className="grid gap-1 text-xs font-mid text-fg-muted">
                <span>{t("metadataModal.fieldOriginalTitle")}</span>
                <Input
                  aria-label={t("metadataModal.fieldOriginalTitle")}
                  maxLength={255}
                  value={draft.originalTitle}
                  onChange={(e) => setDraft((d) => ({ ...d, originalTitle: e.target.value }))}
                />
              </label>
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              <label className="grid gap-1 text-xs font-mid text-fg-muted">
                <span>{t("metadataModal.fieldYear")}</span>
                <Input
                  aria-label={t("metadataModal.fieldYear")}
                  maxLength={4}
                  value={draft.year}
                  onChange={(e) => setDraft((d) => ({ ...d, year: e.target.value }))}
                />
              </label>

              <label className="grid gap-1 text-xs font-mid text-fg-muted">
                <span>{t("metadataModal.fieldRating")}</span>
                <Input
                  aria-label={t("metadataModal.fieldRating")}
                  placeholder="8.5"
                  value={draft.rating}
                  onChange={(e) => setDraft((d) => ({ ...d, rating: e.target.value }))}
                />
              </label>

              <label className="grid gap-1 text-xs font-mid text-fg-muted">
                <span>{t("metadataModal.fieldGenres")}</span>
                <Input
                  aria-label={t("metadataModal.fieldGenres")}
                  value={draft.genres}
                  onChange={(e) => setDraft((d) => ({ ...d, genres: e.target.value }))}
                />
              </label>
            </div>

            <label className="grid gap-1 text-xs font-mid text-fg-muted">
              <span>{t("metadataModal.fieldOverview")}</span>
              <FieldTextarea
                aria-label={t("metadataModal.fieldOverview")}
                rows={4}
                value={draft.overview}
                onChange={(e) => setDraft((d) => ({ ...d, overview: e.target.value }))}
              />
            </label>

            <div className="grid gap-3 sm:grid-cols-2">
              <label className="grid gap-1 text-xs font-mid text-fg-muted">
                <span>{t("metadataModal.fieldPosterUrl")}</span>
                <Input
                  aria-label={t("metadataModal.fieldPosterUrl")}
                  placeholder="https://..."
                  value={draft.posterUrl}
                  onChange={(e) => setDraft((d) => ({ ...d, posterUrl: e.target.value }))}
                />
              </label>

              <label className="grid gap-1 text-xs font-mid text-fg-muted">
                <span>{t("metadataModal.fieldBackdropUrl")}</span>
                <Input
                  aria-label={t("metadataModal.fieldBackdropUrl")}
                  placeholder="https://..."
                  value={draft.backdropUrl}
                  onChange={(e) => setDraft((d) => ({ ...d, backdropUrl: e.target.value }))}
                />
              </label>
            </div>
          </form>
        )}
      </div>

      {/* Footer */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line px-5 py-3">
        <div>
          {existingOverride ? (
            <Button
              size="sm"
              variant="outline"
              isPending={resetMutation.isPending}
              onPress={() => resetMutation.mutate()}
            >
              <RotateCcw className="size-3.5" aria-hidden />
              {t("metadataModal.reset")}
            </Button>
          ) : null}
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="ghost" onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <Button
            size="sm"
            variant="primary"
            isPending={saveMutation.isPending}
            onPress={() => saveMutation.mutate()}
          >
            {saveMutation.isPending ? <Spinner size="sm" color="current" /> : <Check className="size-3.5" aria-hidden />}
            {t("metadataModal.save")}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function MetadataEditModal({
  isOpen,
  onClose,
  itemId,
  workKey,
  defaultMediaType = "movie",
  initialValues,
  onSaved,
  onReset,
}: MetadataEditModalProps) {
  const overrideQuery = useQuery({
    queryKey: queryKeys.metadata.override(itemId),
    queryFn: ({ signal }) => api.metadata.getOverride(itemId, signal),
    enabled: isOpen && Boolean(itemId),
  });

  useEffect(() => {
    if (!isOpen) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const existingOverride = overrideQuery.data?.override ?? null;
  const formKey = `${itemId}:${existingOverride?.updatedAt ?? "initial"}`;

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <MetadataEditDialogBody
        key={formKey}
        onClose={onClose}
        itemId={itemId}
        workKey={workKey}
        defaultMediaType={defaultMediaType}
        initialValues={initialValues}
        existingOverride={existingOverride}
        onSaved={onSaved}
        onReset={onReset}
      />
    </div>
  );
}
