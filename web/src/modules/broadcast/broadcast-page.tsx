import { Button, buttonVariants } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { ExternalLink, Heart, Plus, Radio } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useFeature } from "@/modules/access";
import { useSecureSurface } from "@/skins/secure";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { broadcastApi, type BroadcastEntry, type BroadcastKind, type BroadcastSource } from "./api";
import { fieldClass, SourceForm } from "./source-form";

const kinds = ["radio", "podcast", "iptv"] as const;
function isPlayableUrl(value: string) {
  try {
    const url = new URL(value);
    return ["https:", "http:"].includes(url.protocol) && !url.username && !url.password;
  } catch {
    return false;
  }
}

function Player({
  entry,
  kind,
  close,
}: {
  entry: BroadcastEntry;
  kind: BroadcastKind;
  close: () => void;
}) {
  const { t } = useTranslation();
  const [line, setLine] = useState(0);
  const [error, setError] = useState(false);
  const [hasVideo, setHasVideo] = useState(false);
  const streams = entry.streams.filter(isPlayableUrl);
  const url = streams[line];
  if (!url)
    return (
      <p role="status" className="text-sm text-fg-muted">
        {t("broadcast.noStreams")}
      </p>
    );
  const common = {
    controls: true,
    autoPlay: true,
    src: url,
    preload: "none",
    onError: () => setError(true),
  };
  return (
    <div className="grid gap-3 rounded-panel border border-line bg-surface p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <strong>{entry.title}</strong>
        <Button size="sm" variant="ghost" onPress={close}>
          {t("broadcast.close")}
        </Button>
      </div>
      {kind === "radio" ? (
        <audio key={url} {...common} className="w-full max-w-xl" />
      ) : (
        <video
          key={url}
          {...common}
          playsInline
          onLoadedMetadata={(event) => setHasVideo(event.currentTarget.videoWidth > 0)}
          className={kind === "iptv" || hasVideo ? "w-full max-w-3xl" : "h-12 w-full max-w-xl"}
        />
      )}
      {error && (
        <p role="status" className="text-sm text-fg-muted">
          {t("broadcast.playerUnsupported")}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        {streams.length > 1 && (
          <select
            aria-label={t("broadcast.chooseLine")}
            className="rounded-control border border-line bg-surface px-2 text-sm"
            value={line}
            onChange={(e) => {
              setError(false);
              setHasVideo(false);
              setLine(Number(e.target.value));
            }}
          >
            {streams.map((stream, index) => (
              <option key={stream} value={index}>
                {t("broadcast.line", { n: index + 1 })}
              </option>
            ))}
          </select>
        )}
        <a
          href={url}
          target="_blank"
          rel="noreferrer noopener"
          className={buttonVariants({ variant: "ghost", size: "sm" })}
        >
          <ExternalLink aria-hidden />
          {t("broadcast.external")}
        </a>
      </div>
    </div>
  );
}

function EntryList({ source, reload }: { source: BroadcastSource; reload: () => void }) {
  const { t } = useTranslation();
  const [offset, setOffset] = useState(0);
  const [playing, setPlaying] = useState<BroadcastEntry | null>(null);
  const entries = useQuery({
    queryKey: ["broadcast", "entries", source.id, source.version, offset],
    queryFn: ({ signal }) => broadcastApi.entries(source.id, source.version, offset, signal),
  });
  const favorite = useMutation({
    mutationFn: (entry: BroadcastEntry) =>
      broadcastApi.favorite(source.id, entry.id, !entry.favorite),
    onSuccess: () => entries.refetch(),
  });
  return (
    <Section
      title={t(source.kind === "podcast" ? "broadcast.episodes" : "broadcast.channels")}
      count={source.entryCount}
    >
      {playing && (
        <Player
          key={playing.id}
          entry={playing}
          kind={source.kind}
          close={() => setPlaying(null)}
        />
      )}
      {entries.isPending ? (
        <LoadingRows rows={3} />
      ) : entries.isError ? (
        <ErrorState
          error={entries.error}
          onRetry={() => {
            reload();
            void entries.refetch();
          }}
        />
      ) : (
        <ul className="divide-y divide-line">
          {entries.data.entries.map((entry) => (
            <li key={entry.id} className="flex flex-wrap items-center justify-between gap-2 py-2">
              <div className="min-w-0 flex-1">
                <span className="break-words font-medium">{entry.title}</span>
                {entry.group && <div className="text-sm text-fg-muted">{entry.group}</div>}
                {entry.current && (
                  <p className="text-sm">
                    {t("broadcast.current")} · {entry.current.title}
                  </p>
                )}
                {entry.next && (
                  <p className="text-sm text-fg-muted">
                    {t("broadcast.next")} · {entry.next.title}
                  </p>
                )}
                {entry.publishedAt && (
                  <time className="block text-sm text-fg-muted" dateTime={entry.publishedAt}>
                    {new Date(entry.publishedAt).toLocaleDateString()}
                  </time>
                )}
              </div>
              <Button
                variant="ghost"
                size="sm"
                isIconOnly
                aria-label={
                  t(entry.favorite ? "broadcast.unfavorite" : "broadcast.favorite") +
                  " " +
                  entry.title
                }
                aria-pressed={entry.favorite}
                isDisabled={favorite.isPending}
                onPress={() => favorite.mutate(entry)}
              >
                <Heart fill={entry.favorite ? "currentColor" : "none"} aria-hidden />
              </Button>
              <Button
                size="sm"
                variant="outline"
                aria-label={t("broadcast.preview") + " " + entry.title}
                onPress={() => setPlaying(entry)}
              >
                {t("broadcast.preview")}
              </Button>
            </li>
          ))}
        </ul>
      )}
      {favorite.isError && <ErrorState compact error={favorite.error} />}
      {entries.data && (offset > 0 || entries.data.nextOffset != null) && (
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            isDisabled={!offset}
            onPress={() => setOffset(Math.max(0, offset - 50))}
          >
            {t("broadcast.previous")}
          </Button>
          <span className="text-sm text-fg-muted">
            {t("broadcast.range", {
              from: offset + 1,
              to: offset + entries.data.entries.length,
              count: source.entryCount,
            })}
          </span>
          <Button
            variant="outline"
            size="sm"
            isDisabled={entries.data.nextOffset == null}
            onPress={() => {
              if (entries.data.nextOffset != null) setOffset(entries.data.nextOffset);
            }}
          >
            {t("broadcast.nextPage")}
          </Button>
        </div>
      )}
    </Section>
  );
}

export function BroadcastPage({ kind = "radio" }: { kind?: BroadcastKind }) {
  const { t } = useTranslation();
  const client = useQueryClient();
  const admin = useIsAdmin();
  const hasVirtualTv = useFeature("virtualTv");
  const [selectedId, setSelected] = useState("");
  const [editing, setEditing] = useState<BroadcastSource | "new" | null>(null);
  const secure = useSecureSurface();
  const [removing, setRemoving] = useState<BroadcastSource | null>(null);
  const sources = useQuery({
    queryKey: ["broadcast", "sources", kind],
    queryFn: ({ signal }) => broadcastApi.list(kind, signal),
  });
  const list = sources.data?.sources ?? [];
  const selected = list.find((s) => s.id === selectedId) ?? list[0];
  const reload = () => client.invalidateQueries({ queryKey: ["broadcast", "sources", kind] });
  const refresh = useMutation({
    mutationFn: (source: BroadcastSource) =>
      broadcastApi.save(source.id, {
        name: source.name,
        kind,
        expectedVersion: source.version,
        feedUrl: source.feedUrl,
        guideUrl: source.guideUrl,
      }),
    onSuccess: reload,
  });
  const remove = useMutation({
    mutationFn: broadcastApi.remove,
    onSuccess: () => {
      setRemoving(null);
      setSelected("");
      void reload();
    },
  });
  function select(id: string) {
    setSelected(id);
    setRemoving(null);
    refresh.reset();
    remove.reset();
    setEditing(null);
  }

  return (
    <PageFrame>
      <PageHeader
        title={t("modules.broadcast.title")}
        actions={
          admin && (
            <Button
              size="sm"
              onPress={() => {
                setRemoving(null);
                setEditing("new");
              }}
            >
              <Plus aria-hidden />
              {t("broadcast.add")}
            </Button>
          )
        }
      />
      <nav aria-label={t("broadcast.switchKind")} className="flex flex-wrap gap-2">
        {kinds.map((k) => (
          <Link
            key={k}
            to="/broadcast"
            search={{ kind: k }}
            aria-current={kind === k ? "page" : undefined}
            className={buttonVariants({ variant: kind === k ? "primary" : "outline", size: "sm" })}
          >
            {t(("broadcast." + k) as "broadcast.radio" | "broadcast.iptv" | "broadcast.podcast")}
          </Link>
        ))}
        {hasVirtualTv && (
          <Link
            to="/broadcast/virtual"
            className={buttonVariants({ variant: "outline", size: "sm" })}
          >
            {t("broadcast.virtualTv.nav")}
          </Link>
        )}
      </nav>
      {editing && (
        <SourceForm
          key={editing === "new" ? "new" : editing.id}
          kind={kind}
          source={editing === "new" ? undefined : editing}
          close={() => setEditing(null)}
          saved={(id) => {
            select(id);
            void reload();
          }}
        />
      )}
      {sources.isPending ? (
        <LoadingRows rows={3} />
      ) : sources.isError ? (
        <ErrorState error={sources.error} onRetry={() => void sources.refetch()} />
      ) : !selected ? (
        <EmptyState icon={Radio} title={t("broadcast.noSources")} />
      ) : (
        <>
          <Section title={t("broadcast.sources")} count={list.length}>
            <label className="flex max-w-xl flex-col gap-1 text-sm text-fg-muted">
              <span className="sr-only">{t("broadcast.sources")}</span>
              <select
                value={selected.id}
                className={fieldClass}
                onChange={(e) => select(e.target.value)}
              >
                {list.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
              </select>
            </label>
            {admin && (
              <div className="mt-3 flex flex-wrap gap-2">
                {selected.feedUrl && (
                  <Button
                    size="sm"
                    variant="outline"
                    isDisabled={refresh.isPending}
                    onPress={() => refresh.mutate(selected)}
                  >
                    {t(refresh.isPending ? "broadcast.updating" : "broadcast.refresh")}
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="ghost"
                  onPress={() => {
                    setRemoving(null);
                    setEditing(selected);
                  }}
                >
                  {t("broadcast.edit")}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onPress={() => {
                    setEditing(null);
                    remove.reset();
                    setRemoving(selected);
                  }}
                >
                  {t("broadcast.remove")}
                </Button>
              </div>
            )}
            {admin && removing?.id === selected.id && removing.version === selected.version && (
              <div {...secure} className="mt-3 flex flex-wrap items-center gap-2">
                <span className="min-w-0 wrap-anywhere">{t("broadcast.removeConfirm", { name: removing.name })}</span>
                <Button
                  size="sm"
                  variant="danger"
                  isDisabled={remove.isPending}
                  onPress={() => remove.mutate(removing)}
                >
                  {t("broadcast.confirmRemove")}
                </Button>
                <Button size="sm" variant="ghost" isDisabled={remove.isPending} onPress={() => setRemoving(null)}>
                  {t("broadcast.cancel")}
                </Button>
                {remove.isError && <ErrorState compact error={remove.error} />}
              </div>
            )}
            {refresh.isError && (
              <ErrorState compact error={refresh.error} />
            )}
          </Section>
          <EntryList
            key={selected.id + ":" + selected.version}
            source={selected}
            reload={() => void reload()}
          />
        </>
      )}
    </PageFrame>
  );
}

export function BroadcastRoute() {
  const { kind = "radio" } = useSearch({ from: "/_app/broadcast/" });
  return <BroadcastPage key={kind} kind={kind} />;
}
