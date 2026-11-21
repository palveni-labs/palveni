import { Button } from "@heroui/react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import { ApiError } from "@/api/errors";
import type { components } from "@/api/generated";
import { ErrorState, LoadingRows } from "@/ui/states";
import { VideoSubtitles } from "./video-subtitles";
import { spokenProgress } from "./spoken-progress";

type Ticket = components["schemas"]["PlaybackInfo"];
type MediaState = components["schemas"]["MediaState"];
type Opened = { ticket: Ticket; state: MediaState };
type Props = { id: string; title: string; video?: boolean; onProgressSaved?: () => void };

function ticketFor(id: string, signal?: AbortSignal) {
  return request<Ticket>(`/items/${encodeURIComponent(id)}/playback`, {
    method: "POST",
    signal,
  });
}

function MediaSession({
  id,
  title,
  video,
  onProgressSaved,
  opened,
  retry,
}: Props & { opened: Opened; retry: () => void }) {
  const { t } = useTranslation();
  const [ticket, setTicket] = useState(opened.ticket);
  const [error, setError] = useState<unknown>();
  const [progressError, setProgressError] = useState<unknown>();
  const media = useRef<HTMLMediaElement>(null);
  const Element = video ? "video" : "audio";
  const last = useRef(opened.state.positionMs / 1000);
  const lastReportedAt = useRef(0);
  const ready = useRef(false);
  const played = useRef(false);
  const renewal = useRef<AbortController | null>(null);
  const [save] = useState(() =>
    spokenProgress(
      opened.state,
      async (body) => {
        const updated = await request<MediaState>(`/me/items/${encodeURIComponent(id)}/progress`, {
          method: "PUT",
          body,
        });
        onProgressSaved?.();
        return updated;
      },
      (cause) => setProgressError(cause),
    ),
  );
  useEffect(() => {
    const element = media.current;
    return () => {
      renewal.current?.abort();
      if (element && ready.current && played.current)
        save(last.current, element.ended);
    };
  }, [save]);

  const renew = () => {
    if (renewal.current) return;
    const controller = new AbortController();
    renewal.current = controller;
    ready.current = false;
    void ticketFor(id, controller.signal)
      .then((next) => {
        if (!controller.signal.aborted) setTicket(next);
      })
      .catch((cause) => {
        if (!controller.signal.aborted) setError(cause);
      })
      .finally(() => {
        if (renewal.current === controller) renewal.current = null;
      });
  };
  const remember = (element: HTMLMediaElement) => {
    if (!ready.current || !played.current || element.ended) return;
    last.current = element.currentTime;
    save(last.current, false);
    lastReportedAt.current = Date.now();
  };
  return (
    <section
      aria-label={title}
      className="grid gap-2 rounded-tile border border-line p-3"
    >
      {!video && <h3 className="font-mid">{title}</h3>}
      {error ? (
        <ErrorState error={error} onRetry={retry} compact />
      ) : (
        <Element
          ref={(element) => {
            media.current = element;
          }}
          controls
          playsInline={video}
          autoPlay
          preload="metadata"
          src={ticket.url}
          aria-label={title}
          className={
            video ? "max-h-[65vh] w-full rounded-tile bg-sunken" : "w-full"
          }
          onLoadedMetadata={(event) => {
            const element = event.currentTarget;
            const end = Number.isFinite(element.duration)
              ? Math.max(0, element.duration - 0.1)
              : last.current;
            if (last.current > 0)
              element.currentTime = Math.min(last.current, end);
            ready.current = true;
          }}
          onPlaying={() => {
            played.current = true;
          }}
          onTimeUpdate={(event) => {
            const element = event.currentTarget;
            if (
              !ready.current ||
              !played.current ||
              element.seeking ||
              element.ended
            )
              return;
            last.current = element.currentTime;
            if (Date.now() - lastReportedAt.current >= 15_000)
              remember(element);
          }}
          onSeeked={(event) => remember(event.currentTarget)}
          onPause={(event) => remember(event.currentTarget)}
          onEnded={() => {
            last.current = 0;
            save(0, true);
          }}
          onError={() => {
            if (Date.now() >= Date.parse(ticket.expiresAt) - 2_000) renew();
            else
              setError(
                new Error(
                  t(video ? "video.unsupported" : "spoken.playbackFailed"),
                ),
              );
          }}
        />
      )}
      {video && !error && <VideoSubtitles id={id} media={media} />}
      {progressError instanceof ApiError && progressError.status === 409 ? (
        <p role="alert" className="text-sm">
          {t("spoken.progressConflict")}{" "}
          <Button size="sm" variant="ghost" onPress={retry}>
            {t("common.retry")}
          </Button>
        </p>
      ) : progressError ? (
        <ErrorState error={progressError} onRetry={retry} compact />
      ) : null}
    </section>
  );
}

function OpenRecording({
  id,
  title,
  video,
  onProgressSaved,
  retry,
}: Props & { retry: () => void }) {
  const [opened, setOpened] = useState<Opened | null>(null);
  const [error, setError] = useState<unknown>();
  useEffect(() => {
    const controller = new AbortController();
    const { signal } = controller;
    void Promise.all([
      ticketFor(id, signal),
      request<MediaState>(`/me/items/${encodeURIComponent(id)}`, { signal }),
    ])
      .then(([ticket, state]) => {
        if (!signal.aborted) setOpened({ ticket, state });
      })
      .catch((cause) => {
        if (!signal.aborted) setError(cause);
      });
    return () => controller.abort();
  }, [id]);
  if (error) return <ErrorState error={error} onRetry={retry} compact />;
  if (!opened) return <LoadingRows rows={1} />;
  return (
    <MediaSession
      id={id}
      title={title}
      video={video}
      onProgressSaved={onProgressSaved}
      opened={opened}
      retry={retry}
    />
  );
}

export function OriginalPlayer({ id, title, video, onProgressSaved }: Props) {
  const [attempt, setAttempt] = useState(0);
  return (
    <OpenRecording
      key={`${id}-${attempt}`}
      id={id}
      title={title}
      video={video}
      onProgressSaved={onProgressSaved}
      retry={() => setAttempt((n) => n + 1)}
    />
  );
}
