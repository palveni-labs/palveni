import { Button } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { request, requestBlob } from "@/api/client";
import type { components } from "@/api/generated";
import { useFeature } from "@/modules/access";
import { ErrorState } from "@/ui/states";

type SubtitleTrack = components["schemas"]["SubtitleTrack"];
type SubtitleList = components["schemas"]["SubtitleList"];
type Media = RefObject<HTMLMediaElement | null>;

function ActiveSubtitle({ path, track, media, label }: { path: string; track: SubtitleTrack; media: Media; label: string }) {
  const [failed, setFailed] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const { t } = useTranslation();
  useEffect(() => {
    if (!(media.current instanceof HTMLVideoElement)) return;
    const video = media.current;
    const controller = new AbortController();
    let url: string | undefined;
    let element: HTMLTrackElement | undefined;
    const load = async () => {
      if (controller.signal.aborted) return;
      try {
        const blob = await requestBlob(`${path}/${encodeURIComponent(track.id)}`, { signal: controller.signal });
        if (controller.signal.aborted) return;
        url = URL.createObjectURL(blob);
        element = document.createElement("track");
        element.kind = "subtitles";
        element.label = label;
        element.srclang = track.language ?? "und";
        element.default = true;
        element.onerror = () => setFailed(true);
        element.src = url;
        video.appendChild(element);
        element.track.mode = "showing";
      } catch {
        if (!controller.signal.aborted) setFailed(true);
      }
    };
    queueMicrotask(() => void load());
    return () => {
      controller.abort();
      if (element) {
        element.onerror = null;
        element.track.mode = "disabled";
        element.remove();
      }
      if (url) URL.revokeObjectURL(url);
    };
  }, [attempt, label, media, path, track.id, track.language]);
  if (!failed) return null;
  return <p role="alert" className="flex flex-wrap items-center gap-2 text-sm text-fg">
    {t("videoSubtitles.unreadable")}
    <Button size="sm" variant="ghost" onPress={() => { setFailed(false); setAttempt((n) => n + 1); }}>{t("common.retry")}</Button>
  </p>;
}

export function VideoSubtitles({ id, media }: { id: string; media: Media }) {
  const { t, i18n } = useTranslation();
  const enabled = useFeature("videoSubtitles");
  const path = `/items/${encodeURIComponent(id)}/subtitles`;
  const query = useQuery({
    queryKey: ["items", "subtitles", id],
    queryFn: ({ signal }) => request<SubtitleList>(path, { signal }),
    enabled, staleTime: 0, gcTime: 0,
  });
  const [selected, setSelected] = useState("");
  if (!enabled || query.isPending) return null;
  if (query.isError) return <ErrorState compact error={query.error} onRetry={() => void query.refetch()} />;
  if (!query.data.tracks.length && !query.data.truncated) return null;
  const names = new Intl.DisplayNames(i18n.resolvedLanguage ?? "en", { type: "language" });
  const label = (track: SubtitleTrack) => [
    track.language ? names.of(track.language) ?? track.language : t("videoSubtitles.unspecified"),
    track.forced ? t("videoSubtitles.forced") : null,
    track.hearingImpaired ? t("videoSubtitles.sdh") : null,
    track.format.toUpperCase(),
  ].filter(Boolean).join(" · ");
  const current = query.data.tracks.find((track) => track.id === selected);
  return <>
    <label className="flex min-w-0 flex-wrap items-center gap-2 text-sm text-fg">
      {t("videoSubtitles.label")}
      <select className="min-w-0 max-w-full rounded-control border border-line bg-surface px-2 py-1.5 text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
        value={current?.id ?? ""} onChange={(event) => setSelected(event.target.value)}>
        <option value="">{t("videoSubtitles.off")}</option>
        {query.data.tracks.map((track) => <option key={track.id} value={track.id}>{label(track)}</option>)}
      </select>
    </label>
    {query.data.truncated && <p className="text-sm text-fg-muted">{t("videoSubtitles.limited")}</p>}
    {current && <ActiveSubtitle key={current.id} path={path} track={current} media={media} label={label(current)} />}
  </>;
}
