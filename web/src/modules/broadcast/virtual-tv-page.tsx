import { Button, buttonVariants, Chip, Input, Modal, Spinner, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Copy, Play, Plus, Trash2, Tv } from "lucide-react";
import { useEffect, useId, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useSecureSurface } from "@/skins/secure";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { ProgressBar } from "@/ui/progress";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { virtualTvApi, type VirtualChannel, type VirtualChannelGuide } from "./api";

const BROADCAST_KINDS = ["radio", "podcast", "iptv"] as const;

async function copyFeedUrl(value: string): Promise<void> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(value);
    }
  } catch {
    // Clipboard access may be unavailable over plain HTTP LAN; toast still confirms action.
  }
}

function formatClockRange(startedAt: string, endsAt: string): string {
  const fmt = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
  return `${fmt.format(new Date(startedAt))} – ${fmt.format(new Date(endsAt))}`;
}

function formatMinutesSeconds(ms: number): string {
  const totalSecs = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(totalSecs / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return hours > 0 ? `${hours}:${pad(mins)}:${pad(secs)}` : `${mins}:${pad(secs)}`;
}

function episodeBadge(season: number | null | undefined, episode: number | null | undefined): string | null {
  if (season == null || episode == null) return null;
  return `S${String(season).padStart(2, "0")}E${String(episode).padStart(2, "0")}`;
}

function CreateVirtualChannelModal({
  open,
  onOpenChange,
  nextNumber,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  nextNumber: number;
}) {
  const { t } = useTranslation();
  const formId = useId();
  const queryClient = useQueryClient();
  const libraries = useLibraries();
  const videoLibs = (libraries.data?.libraries ?? []).filter((lib) => lib.kind === "video");

  const [number, setNumber] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [libraryId, setLibraryId] = useState("");
  const [genreFilter, setGenreFilter] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const effectiveNumber = number ?? String(nextNumber);
  const effectiveLibraryId = libraryId || videoLibs[0]?.id || "";

  const resetModal = () => {
    setNumber(null);
    setName("");
    setGenreFilter("");
    setFormError(null);
  };

  const create = useMutation({
    mutationFn: virtualTvApi.create,
    onSuccess: () => {
      resetModal();
      onOpenChange(false);
      void queryClient.invalidateQueries({ queryKey: ["virtual-tv"] });
    },
    onError: (err) => setFormError(errorText(t, err)),
  });

  function onSubmit(event: FormEvent) {
    event.preventDefault();
    setFormError(null);
    const parsedNum = Number(effectiveNumber);
    if (!Number.isInteger(parsedNum) || parsedNum < 1 || parsedNum > 9999) {
      setFormError("1–9999");
      return;
    }
    if (!effectiveLibraryId) {
      setFormError(t("broadcast.virtualTv.noVideoLibraries"));
      return;
    }
    create.mutate({
      number: parsedNum,
      name: name.trim(),
      libraryId: effectiveLibraryId,
      genreFilter: genreFilter.trim() || null,
      enabled: true,
    });
  }

  return (
    <Modal.Backdrop
      isOpen={open}
      onOpenChange={(next) => {
        if (!create.isPending) {
          if (!next) resetModal();
          onOpenChange(next);
        }
      }}
      isDismissable={!create.isPending}
    >
      <Modal.Container size="md" scroll="inside">
        <Modal.Dialog className="gap-0 p-0">
          <Modal.Header className="grid gap-1 px-6 pt-6 pb-4">
            <Modal.Heading className="font-heading text-lg font-title text-heading">
              {t("broadcast.virtualTv.addChannel")}
            </Modal.Heading>
          </Modal.Header>
          <Modal.Body className="px-6 pb-2">
            {videoLibs.length === 0 ? (
              <p className="py-2 text-sm text-fg-muted">
                {t("broadcast.virtualTv.noVideoLibraries")}
              </p>
            ) : (
              <form id={formId} onSubmit={onSubmit} className="grid gap-4">
                <div className="grid gap-3 sm:grid-cols-3">
                  <label className="grid gap-1 text-sm text-fg">
                    {t("broadcast.virtualTv.channelNumber")}
                    <Input
                      required
                      type="number"
                      min={1}
                      max={9999}
                      aria-label={t("broadcast.virtualTv.channelNumber")}
                      value={effectiveNumber}
                      onChange={(e) => setNumber(e.target.value)}
                    />
                  </label>
                  <label className="grid gap-1 text-sm text-fg sm:col-span-2">
                    {t("broadcast.virtualTv.channelName")}
                    <Input
                      required
                      aria-label={t("broadcast.virtualTv.channelName")}
                      placeholder={t("broadcast.virtualTv.channelNamePlaceholder")}
                      value={name}
                      onChange={(e) => setName(e.target.value)}
                    />
                  </label>
                </div>

                <label className="grid gap-1 text-sm text-fg">
                  {t("broadcast.virtualTv.videoLibrary")}
                  <select
                    aria-label={t("broadcast.virtualTv.videoLibrary")}
                    className="pv-select w-full"
                    value={effectiveLibraryId}
                    onChange={(e) => setLibraryId(e.target.value)}
                  >
                    {videoLibs.map((lib) => (
                      <option key={lib.id} value={lib.id}>
                        {lib.name}
                      </option>
                    ))}
                  </select>
                </label>

                <label className="grid gap-1 text-sm text-fg">
                  {t("broadcast.virtualTv.genreFilter")}
                  <Input
                    aria-label={t("broadcast.virtualTv.genreFilter")}
                    placeholder={t("broadcast.virtualTv.genrePlaceholder")}
                    value={genreFilter}
                    onChange={(e) => setGenreFilter(e.target.value)}
                  />
                </label>

                {formError && (
                  <p role="alert" className="text-sm text-failed">
                    {formError}
                  </p>
                )}
              </form>
            )}
          </Modal.Body>
          <Modal.Footer className="flex justify-end gap-2 px-6 pt-4 pb-6">
            <Button
              variant="secondary"
              isDisabled={create.isPending}
              onPress={() => onOpenChange(false)}
            >
              {t("common.cancel")}
            </Button>
            {videoLibs.length > 0 && (
              <Button type="submit" form={formId} variant="primary" isPending={create.isPending}>
                {create.isPending ? <Spinner size="sm" color="current" /> : null}
                {t("broadcast.virtualTv.createSubmit")}
              </Button>
            )}
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

function ChannelGuideCard({
  entry,
  nowTickMs,
  generatedAtMs,
  isAdmin,
}: {
  entry: VirtualChannelGuide;
  nowTickMs: number;
  generatedAtMs: number;
  isAdmin: boolean;
}) {
  const { t } = useTranslation();
  const inset = useRowInset();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();
  const [confirmDelete, setConfirmDelete] = useState<VirtualChannel | null>(null);

  const remove = useMutation({
    mutationFn: (id: string) => virtualTvApi.remove(id),
    onSuccess: () => {
      setConfirmDelete(null);
      void queryClient.invalidateQueries({ queryKey: ["virtual-tv"] });
    },
  });

  const { channel, nowPlaying, programmes, totalItems, cycleDurationMs } = entry;
  const driftMs = Math.max(0, nowTickMs - generatedAtMs);
  const liveOffsetMs = nowPlaying
    ? Math.min(nowPlaying.durationMs, Math.max(0, nowPlaying.offsetMs + driftMs))
    : 0;
  const progressRatio =
    nowPlaying && nowPlaying.durationMs > 0 ? liveOffsetMs / nowPlaying.durationMs : 0;
  const epCode = episodeBadge(nowPlaying?.season, nowPlaying?.episode);
  const cycleMinutes = Math.max(1, Math.round(cycleDurationMs / 60_000));

  return (
    <Section
      title={`CH ${channel.number} · ${channel.name}`}
      actions={
        <div className="flex flex-wrap items-center gap-2">
          {channel.genreFilter && (
            <Chip size="sm" variant="soft">
              {channel.genreFilter}
            </Chip>
          )}
          <span className="text-xs text-fg-muted">
            {t("broadcast.virtualTv.cycleSummary", {
              items: totalItems,
              minutes: cycleMinutes,
            })}
          </span>
          {isAdmin && (
            <Button
              size="sm"
              variant="ghost"
              isIconOnly
              aria-label={t("broadcast.virtualTv.deleteChannel") + " " + channel.name}
              onPress={() => setConfirmDelete(channel)}
            >
              <Trash2 aria-hidden />
            </Button>
          )}
        </div>
      }
    >
      {confirmDelete?.id === channel.id && (
        <div
          {...secure}
          className={cn("flex flex-wrap items-center gap-2 border-b border-line py-3 text-sm", inset)}
        >
          <span>{t("broadcast.virtualTv.confirmDelete", { name: channel.name })}</span>
          <Button
            size="sm"
            variant="danger"
            isPending={remove.isPending}
            onPress={() => remove.mutate(channel.id)}
          >
            {t("common.delete")}
          </Button>
          <Button size="sm" variant="ghost" onPress={() => setConfirmDelete(null)}>
            {t("common.cancel")}
          </Button>
        </div>
      )}

      {!nowPlaying ? (
        <p className={cn("py-4 text-sm text-fg-muted", inset)}>
          {t("broadcast.virtualTv.noPlayableItems")}
        </p>
      ) : (
        <div className={cn("grid gap-4 py-4", inset)}>
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="grid min-w-0 flex-1 gap-1">
              <div className="flex flex-wrap items-center gap-2">
                <Chip size="sm" color="accent" variant="soft">
                  {t("broadcast.virtualTv.liveNow")}
                </Chip>
                <span className="text-base font-strong text-fg">{nowPlaying.workTitle}</span>
                {epCode && <span className="font-mono text-xs text-fg-muted">{epCode}</span>}
                {nowPlaying.title !== nowPlaying.workTitle && (
                  <span className="text-sm text-fg-muted">{nowPlaying.title}</span>
                )}
              </div>
              <div className="flex flex-wrap items-center gap-3 text-xs text-fg-muted tabular-nums">
                <span>{formatClockRange(nowPlaying.startedAt, nowPlaying.endsAt)}</span>
                <span>
                  {formatMinutesSeconds(liveOffsetMs)} /{" "}
                  {formatMinutesSeconds(nowPlaying.durationMs)}
                </span>
              </div>
            </div>

            <Link
              to="/video/play/$itemId"
              params={{ itemId: nowPlaying.itemId }}
              search={{
                libraryId: nowPlaying.libraryId,
                startMs: Math.floor(liveOffsetMs),
                title: nowPlaying.workTitle,
              }}
              className={buttonVariants({ variant: "primary", size: "sm" })}
            >
              <Play aria-hidden />
              {t("broadcast.virtualTv.tuneInNow")}
            </Link>
          </div>

          <ProgressBar
            value={progressRatio}
            label={`${channel.name} · ${nowPlaying.workTitle}`}
          />

          {programmes.length > 0 && (
            <div className="grid gap-1.5 border-t border-line pt-3">
              <span className="text-xs font-mid text-fg-muted">
                {t("broadcast.virtualTv.upcomingSchedule")}
              </span>
              <SectionRows>
                {programmes.slice(0, 6).map((prog, idx) => {
                  const nextEp = episodeBadge(prog.season, prog.episode);
                  return (
                    <li
                      key={`${prog.itemId}:${prog.startedAt}:${idx}`}
                      className="flex flex-wrap items-center justify-between gap-2 py-2 text-sm"
                    >
                      <div className="flex min-w-0 items-center gap-2">
                        <span className="truncate text-fg">{prog.workTitle}</span>
                        {nextEp && (
                          <span className="font-mono text-xs text-fg-muted">{nextEp}</span>
                        )}
                      </div>
                      <span className="shrink-0 text-xs text-fg-muted tabular-nums">
                        {formatClockRange(prog.startedAt, prog.endsAt)}
                      </span>
                    </li>
                  );
                })}
              </SectionRows>
            </div>
          )}
        </div>
      )}
    </Section>
  );
}

export function VirtualTvPage() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const [modalOpen, setModalOpen] = useState(false);
  const [nowTickMs, setNowTickMs] = useState(() => Date.now());

  useEffect(() => {
    const id = window.setInterval(() => setNowTickMs(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, []);

  const guideQuery = useQuery({
    queryKey: ["virtual-tv", "guide"],
    queryFn: ({ signal }) => virtualTvApi.guide(undefined, signal),
    refetchInterval: 30_000,
  });

  const channels = guideQuery.data?.channels ?? [];
  const nextChannelNumber =
    channels.length > 0 ? Math.max(...channels.map((c) => c.channel.number)) + 1 : 101;
  const generatedAtMs = guideQuery.data
    ? new Date(guideQuery.data.generatedAt).getTime()
    : nowTickMs;

  const m3uHref = guideQuery.data
    ? new URL(guideQuery.data.playlistUrl, window.location.origin).href
    : "";
  const epgHref = guideQuery.data
    ? new URL(guideQuery.data.epgUrl, window.location.origin).href
    : "";

  return (
    <PageFrame>
      <PageHeader
        title={t("broadcast.virtualTv.title")}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {m3uHref && (
              <Button
                size="sm"
                variant="outline"
                onPress={() => {
                  void copyFeedUrl(m3uHref).then(() =>
                    toast.success(t("broadcast.virtualTv.copiedM3u")),
                  );
                }}
              >
                <Copy aria-hidden />
                {t("broadcast.virtualTv.copyM3u")}
              </Button>
            )}
            {epgHref && (
              <Button
                size="sm"
                variant="outline"
                onPress={() => {
                  void copyFeedUrl(epgHref).then(() =>
                    toast.success(t("broadcast.virtualTv.copiedXmltv")),
                  );
                }}
              >
                <Copy aria-hidden />
                {t("broadcast.virtualTv.copyXmltv")}
              </Button>
            )}
            {isAdmin && (
              <Button size="sm" onPress={() => setModalOpen(true)}>
                <Plus aria-hidden />
                {t("broadcast.virtualTv.addChannel")}
              </Button>
            )}
          </div>
        }
      />

      <nav aria-label={t("broadcast.switchKind")} className="flex flex-wrap gap-2">
        {BROADCAST_KINDS.map((k) => (
          <Link
            key={k}
            to="/broadcast"
            search={{ kind: k }}
            className={buttonVariants({ variant: "outline", size: "sm" })}
          >
            {t(("broadcast." + k) as "broadcast.radio" | "broadcast.iptv" | "broadcast.podcast")}
          </Link>
        ))}
        <Link
          to="/broadcast/virtual"
          aria-current="page"
          className={buttonVariants({ variant: "primary", size: "sm" })}
        >
          {t("broadcast.virtualTv.nav")}
        </Link>
      </nav>

      {isAdmin && (
        <CreateVirtualChannelModal
          open={modalOpen}
          onOpenChange={setModalOpen}
          nextNumber={nextChannelNumber}
        />
      )}

      {guideQuery.isPending ? (
        <LoadingRows rows={3} />
      ) : guideQuery.isError ? (
        <ErrorState error={guideQuery.error} onRetry={() => void guideQuery.refetch()} />
      ) : channels.length === 0 ? (
        <EmptyState icon={Tv} title={t("broadcast.virtualTv.noChannels")} />
      ) : (
        channels.map((entry) => (
          <ChannelGuideCard
            key={entry.channel.id}
            entry={entry}
            nowTickMs={nowTickMs}
            generatedAtMs={generatedAtMs}
            isAdmin={isAdmin}
          />
        ))
      )}
    </PageFrame>
  );
}
