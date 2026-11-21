import { Button, Input, Label, TextField } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, File, Folder, Music, Network, Plus, Video } from "lucide-react";
import { useRef, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { useIsAdmin } from "@/api/queries/auth";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

type Schema = components["schemas"];
type Connection = Schema["ConnectionSummary"];
type Entry = Schema["ConnectionEntry"];
const protocols = { smb: "SMB", webdav: "WebDAV", jellyfin: "Jellyfin", emby: "Emby", plex: "Plex" } as const;
const fields = "rounded-control border border-line bg-surface px-3 py-2 text-fg";

function AddConnection({ done, close }: { done: (source: Connection) => void; close: () => void }) {
  const { t } = useTranslation();
  const [kind, setKind] = useState<Connection["kind"]>("smb");
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [username, setUsername] = useState("");
  const [secret, setSecret] = useState("");
  const needsUser = kind === "smb" || kind === "webdav";
  const save = useMutation({
    mutationFn: (body: Schema["CreateConnection"]) => request<Connection>("/connections", { method: "POST", body }),
    onSuccess: done,
    gcTime: 0,
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    save.mutate({
      kind,
      name: name.trim(),
      address: address.trim(),
      username: needsUser ? username : undefined,
      secret,
    });
  }
  return (
    <Section title={t("connections.add")}>
      <form className="grid max-w-xl gap-3" onSubmit={submit}>
        <label className="grid gap-1">
          <span>{t("connections.protocol")}</span>
          <select
            className={fields}
            value={kind}
            onChange={(e) => {
              setKind(e.target.value as Connection["kind"]);
              setSecret("");
              setUsername("");
            }}
          >
            {(Object.entries(protocols) as [Connection["kind"], string][]).map(([value, label]) => (
              <option value={value} key={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <TextField isRequired value={name} onChange={setName}>
          <Label>{t("connections.name")}</Label>
          <Input maxLength={100} />
        </TextField>
        <TextField isRequired value={address} onChange={setAddress}>
          <Label>{t("connections.address")}</Label>
          <Input
            placeholder={kind === "smb" ? "smb://nas/Music" : "https://media.example.org"}
            autoCapitalize="none"
            spellCheck={false}
          />
        </TextField>
        {needsUser && (
          <TextField value={username} onChange={setUsername}>
            <Label>{t("connections.username")}</Label>
            <Input autoComplete="off" autoCapitalize="none" spellCheck={false} />
          </TextField>
        )}
        <TextField value={secret} onChange={setSecret}>
          <Label>{t(needsUser ? "connections.password" : "connections.token")}</Label>
          <Input type="password" autoComplete="new-password" />
        </TextField>
        {save.isError && <ErrorState compact error={save.error} />}
        <div className="flex gap-2">
          <Button type="submit" isPending={save.isPending}>
            {t("connections.connect")}
          </Button>
          <Button type="button" variant="ghost" onPress={close}>
            {t("connections.cancel")}
          </Button>
        </div>
      </form>
    </Section>
  );
}

function RemotePlayer({ id, entry, close }: { id: string; entry: Entry; close: () => void }) {
  const { t } = useTranslation();
  const [attempt, retry] = useState(0);
  const ticket = useQuery({
    queryKey: ["connection-playback", id, entry.id, attempt],
    queryFn: ({ signal }) =>
      request<Schema["PlaybackInfo"]>(`/connections/${encodeURIComponent(id)}/playback`, {
        method: "POST",
        body: { entry: entry.id },
        signal,
      }),
    staleTime: 0,
    gcTime: 0,
    refetchOnWindowFocus: false,
  });
  const [failed, setFailed] = useState(false);
  const lastPosition = useRef(0);
  const Media = entry.kind === "video" ? "video" : "audio";
  function renew() {
    setFailed(false);
    retry((value) => value + 1);
  }
  return (
    <Section
      title={entry.name}
      actions={
        <Button size="sm" variant="ghost" onPress={close}>
          {t("connections.close")}
        </Button>
      }
    >
      {ticket.isPending ? (
        <LoadingRows rows={1} />
      ) : ticket.isError ? (
        <ErrorState compact error={ticket.error} onRetry={() => void ticket.refetch()} />
      ) : (
        <Media
          className={entry.kind === "video" ? "max-h-[65vh] w-full bg-sunken" : "w-full"}
          src={ticket.data.url}
          aria-label={entry.name}
          controls
          playsInline
          autoPlay
          preload="metadata"
          onLoadedMetadata={({ currentTarget }) => {
            if (lastPosition.current > 0 && Number.isFinite(currentTarget.duration)) {
              currentTarget.currentTime = Math.min(lastPosition.current, Math.max(0, currentTarget.duration - 0.1));
            }
          }}
          onTimeUpdate={({ currentTarget }) => {
            if (currentTarget.readyState >= 1 && currentTarget.currentTime > 0)
              lastPosition.current = currentTarget.currentTime;
          }}
          onError={() => (Date.now() >= Date.parse(ticket.data.expiresAt) - 2000 ? renew() : setFailed(true))}
        />
      )}
      {failed && (
        <Button variant="outline" onPress={renew}>
          {t("connections.retry")}
        </Button>
      )}
    </Section>
  );
}

function Browser({ source, close, removed }: { source: Connection; close: () => void; removed: () => void }) {
  const { t } = useTranslation();
  const admin = useIsAdmin();
  const [path, setPath] = useState<Pick<Entry, "id" | "name">[]>([]);
  const [offset, setOffset] = useState(0);
  const [playing, setPlaying] = useState<Entry>();
  const [confirm, setConfirm] = useState(false);
  const parent = path.at(-1);
  const base = `/connections/${encodeURIComponent(source.id)}`;
  const listing = useQuery({
    queryKey: ["connection-entries", source.id, parent?.id ?? "", offset],
    queryFn: ({ signal }) =>
      request<Schema["ConnectionPage"]>(base + "/entries", {
        query: { parent: parent?.id, offset, limit: 50 },
        signal,
      }),
  });
  const remove = useMutation({ mutationFn: () => request<void>(base, { method: "DELETE" }), onSuccess: removed });
  function navigate(next: typeof path) {
    setPlaying(undefined);
    setOffset(0);
    setPath(next);
  }
  return (
    <>
      <Section
        title={parent?.name ?? source.name}
        actions={
          <Button variant="ghost" size="sm" onPress={() => (parent ? navigate(path.slice(0, -1)) : close())}>
            <ArrowLeft aria-hidden />
            {t("connections.back")}
          </Button>
        }
      >
        {listing.isPending ? (
          <LoadingRows rows={3} />
        ) : listing.isError ? (
          <ErrorState compact error={listing.error} onRetry={() => void listing.refetch()} />
        ) : (
          <>
            {listing.data.entries.length === 0 && <p className="py-4 text-fg-muted">{t("connections.emptyFolder")}</p>}
            <SectionRows>
              {listing.data.entries.map((entry) => {
                const Icon = { folder: Folder, audio: Music, video: Video, file: File }[entry.kind];
                return (
                  <li key={entry.id}>
                    <button
                      type="button"
                      disabled={entry.kind === "file"}
                      className="flex w-full items-center gap-3 px-3 py-2 text-start hover:bg-hover disabled:text-fg-muted"
                      onClick={() => (entry.kind === "folder" ? navigate([...path, entry]) : setPlaying(entry))}
                    >
                      <Icon className="size-4 shrink-0" aria-hidden />
                      <span className="break-words">{entry.name}</span>
                    </button>
                  </li>
                );
              })}
            </SectionRows>
            {(offset > 0 || listing.data.nextOffset != null) && (
              <div className="mt-3 flex gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  isDisabled={!offset}
                  onPress={() => setOffset(Math.max(0, offset - 50))}
                >
                  {t("broadcast.previous")}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  isDisabled={listing.data.nextOffset == null}
                  onPress={() => setOffset(listing.data.nextOffset ?? offset)}
                >
                  {t("broadcast.nextPage")}
                </Button>
              </div>
            )}
          </>
        )}
      </Section>
      {playing && <RemotePlayer key={playing.id} id={source.id} entry={playing} close={() => setPlaying(undefined)} />}
      {admin && !parent && (
        <div className="flex flex-wrap items-center gap-2">
          {confirm ? (
            <>
              <span>{t("connections.confirm")}</span>
              <Button size="sm" variant="danger" isPending={remove.isPending} onPress={() => remove.mutate()}>
                {t("connections.remove")}
              </Button>
              <Button size="sm" variant="ghost" onPress={() => setConfirm(false)}>
                {t("connections.cancel")}
              </Button>
            </>
          ) : (
            <Button size="sm" variant="ghost" onPress={() => setConfirm(true)}>
              {t("connections.remove")}
            </Button>
          )}
          {remove.isError && <ErrorState compact error={remove.error} />}
        </div>
      )}
    </>
  );
}

export function ConnectionsPage() {
  const { t } = useTranslation();
  const admin = useIsAdmin();
  const cache = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [selected, setSelected] = useState<Connection>();
  const list = useQuery({
    queryKey: ["connections"],
    queryFn: ({ signal }) => request<Schema["ConnectionList"]>("/connections", { signal }),
  });
  return (
    <PageFrame>
      <PageHeader
        title={t("connections.title")}
        actions={
          admin &&
          !adding && (
            <Button
              size="sm"
              onPress={() => {
                setSelected(undefined);
                setAdding(true);
              }}
            >
              <Plus aria-hidden />
              {t("connections.add")}
            </Button>
          )
        }
      />
      {adding ? (
        <AddConnection
          close={() => setAdding(false)}
          done={(source) => {
            setAdding(false);
            setSelected(source);
            void cache.invalidateQueries({ queryKey: ["connections"] });
          }}
        />
      ) : selected ? (
        <Browser
          key={selected.id}
          source={selected}
          close={() => setSelected(undefined)}
          removed={() => {
            setSelected(undefined);
            void cache.invalidateQueries({ queryKey: ["connections"] });
          }}
        />
      ) : list.isPending ? (
        <LoadingRows rows={3} />
      ) : list.isError ? (
        <ErrorState error={list.error} onRetry={() => void list.refetch()} />
      ) : list.data.connections.length === 0 ? (
        <EmptyState icon={Network} title={t("connections.empty")} />
      ) : (
        <Section title={t("connections.saved")} count={list.data.connections.length}>
          <SectionRows>
            {list.data.connections.map((source) => (
              <li key={source.id}>
                <button
                  type="button"
                  className="flex w-full items-center gap-3 px-3 py-3 text-start hover:bg-hover"
                  onClick={() => setSelected(source)}
                >
                  <Network className="size-4" aria-hidden />
                  <span className="grow">{source.name}</span>
                  <span className="text-sm text-fg-muted">{protocols[source.kind]}</span>
                </button>
              </li>
            ))}
          </SectionRows>
        </Section>
      )}
    </PageFrame>
  );
}
