import { Button, Modal, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight, Heart, Play } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { useCurrentUser } from "@/api/queries/auth";
import type { Library } from "@/api/types";
import { errorText } from "@/lib/errors";
import { ITEM_KIND_ICON } from "@/lib/kinds";
import { Section } from "@/ui/section";
import { ErrorState, LoadingRows } from "@/ui/states";
import { OriginalPlayer } from "./browse/original-player";

type Entry = components["schemas"]["PersonalItem"];
type View = components["schemas"]["MediaView"];
type Page = components["schemas"]["PersonalItemPage"];

function clockTime(ms: number) {
  const seconds = Math.floor(ms / 1000);
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor(seconds / 60) % 60;
  return [hours || null, hours ? String(minutes).padStart(2, "0") : minutes, String(seconds % 60).padStart(2, "0")]
    .filter((part) => part !== null).join(":");
}

function PersonalTile({ entry, libraryName, open, refresh }: {
  entry: Entry; libraryName: string; open: () => void; refresh: () => Promise<void>;
}) {
  const { t } = useTranslation();
  const Icon = ITEM_KIND_ICON[entry.kind];
  const favorite = useMutation({
    mutationFn: () => request<components["schemas"]["MediaState"]>(`/me/items/${encodeURIComponent(entry.itemId)}/favorite`, {
      method: "PUT", body: { favorite: !entry.state.favorite },
    }),
    onSettled: refresh,
    onError: (error) => toast.danger(errorText(t, error)),
  });
  const resume = entry.state.positionMs > 0 && !entry.state.completed;
  const playLabel = resume
    ? t("libraryHome.resumeAt", { title: entry.title, time: clockTime(entry.state.positionMs) })
    : t("spoken.play", { title: entry.title });
  return (
    <li className="flex min-w-0 items-center gap-1 rounded-tile border border-line bg-surface pe-2">
      <button type="button" aria-label={playLabel} title={playLabel} onClick={open}
        className="group flex min-w-0 flex-1 items-center gap-3 rounded-tile px-3 py-2 text-start hover:bg-hover focus-visible:outline-2 focus-visible:outline-(--pv-focus)">
        <span className="relative grid size-10 shrink-0 place-items-center rounded-control bg-sunken text-icon">
          <Icon aria-hidden className="size-5 group-hover:invisible group-focus-visible:invisible" />
          <Play aria-hidden className="invisible absolute size-5 group-hover:visible group-focus-visible:visible" />
        </span>
        <span className="grid min-w-0 flex-1 gap-0.5">
          <span className="truncate text-sm font-strong text-fg">{entry.title}</span>
          <span className="flex min-w-0 items-baseline gap-1 text-xs text-fg-muted">
            <span className="truncate">{libraryName}</span>
            {resume && <span className="shrink-0 tabular-nums">· {clockTime(entry.state.positionMs)}</span>}
          </span>
        </span>
      </button>
      <Button size="sm" variant="ghost" isIconOnly aria-label={t("libraryHome.favoriteItem", { title: entry.title })}
        aria-pressed={entry.state.favorite} isDisabled={favorite.isPending} onPress={() => favorite.mutate()}>
        <Heart aria-hidden className={entry.state.favorite ? "size-4 fill-current" : "size-4"} />
      </Button>
    </li>
  );
}

export function PersonalShelf({ libraries }: { libraries: Library[] }) {
  const { t } = useTranslation();
  const user = useCurrentUser();
  const client = useQueryClient();
  const [view, setView] = useState<View>("continue");
  const [cursors, setCursors] = useState<number[]>([]);
  const [playing, setPlaying] = useState<Entry | null>(null);
  const before = cursors.at(-1);
  const accountKey = ["personal-media", user.id];
  const refresh = () => client.invalidateQueries({ queryKey: accountKey });
  const result = useQuery({
    queryKey: [...accountKey, view, before ?? null],
    queryFn: ({ signal }) => request<Page>("/me/items", { query: { view, before, limit: 6 }, signal }),
    refetchOnWindowFocus: "always",
  });
  const names = new Map(libraries.map((library) => [library.id, library.name]));
  const entries = result.data?.items.filter((item) => names.has(item.libraryId)) ?? [];
  const next = result.data?.nextBefore;
  const title = t("libraryHome.title");

  return (
    <Section title={title} unframed bodyClassName="grid min-w-0 gap-3">
      <div role="group" aria-label={title} className="flex flex-wrap gap-1">
        {(["continue", "recent", "favorites"] as const).map((option) => (
          <Button key={option} size="sm" variant={option === view ? "primary" : "ghost"}
            aria-pressed={option === view} onPress={() => {
              setView(option);
              setCursors([]);
            }}>
            {t(`libraryHome.views.${option}`)}
          </Button>
        ))}
      </div>
      {result.isPending ? <LoadingRows rows={1} /> : result.isError ? (
        <ErrorState compact error={result.error} onRetry={() => void result.refetch()} />
      ) : entries.length ? (
        <ul className="grid min-w-0 gap-2 md:grid-cols-2 xl:grid-cols-3">
          {entries.map((entry) => <PersonalTile key={entry.itemId} entry={entry} libraryName={names.get(entry.libraryId)!}
            open={() => setPlaying(entry)} refresh={refresh} />)}
        </ul>
      ) : <p className="py-2 text-sm text-fg-muted">{t(`libraryHome.empty.${view}`)}</p>}
      {(cursors.length > 0 || next != null) && (
        <nav aria-label={t("libraryHome.pages")} className="flex gap-2">
          <Button size="sm" variant="outline" isDisabled={!cursors.length} onPress={() => setCursors((old) => old.slice(0, -1))}>
            <ChevronLeft aria-hidden className="size-4" />{t("libraryHome.previous")}
          </Button>
          <Button size="sm" variant="outline" isDisabled={next == null || result.isFetching}
            onPress={() => { if (next != null) setCursors((old) => [...old, next]); }}>
            {t("libraryHome.next")}<ChevronRight aria-hidden className="size-4" />
          </Button>
        </nav>
      )}
      {playing && names.has(playing.libraryId) && (
        <Modal.Backdrop isOpen onOpenChange={(open) => { if (!open) { setPlaying(null); void refresh(); } }}>
          <Modal.Container size={playing.kind === "video" ? "lg" : "md"} scroll="inside">
            <Modal.Dialog>
              <Modal.Header><Modal.Heading className="break-words pe-8">{playing.title}</Modal.Heading></Modal.Header>
              <Modal.CloseTrigger aria-label={t("connections.close")} />
              <Modal.Body>
                <OriginalPlayer key={playing.itemId} id={playing.itemId} title={playing.title}
                  video={playing.kind === "video"} onProgressSaved={() => { void refresh(); }} />
              </Modal.Body>
            </Modal.Dialog>
          </Modal.Container>
        </Modal.Backdrop>
      )}
    </Section>
  );
}
