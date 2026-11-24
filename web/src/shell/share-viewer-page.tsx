import { Button, buttonVariants } from "@heroui/react";
import { useParams } from "@tanstack/react-router";
import {
  BookOpen,
  Clock,
  Disc3,
  Download,
  Film,
  FolderOpen,
  Image as ImageIcon,
  Lock,
  MapPin,
  Music2,
  Play,
  ShieldCheck,
  X,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApiError } from "@/api/errors";
import {
  publicShareCoverUrl,
  publicShareMediaUrl,
  usePublicShare,
  useUnlockPublicShare,
  type PublicShareItem,
} from "@/api/queries/shares-collections";
import { cn } from "@/lib/cn";
import { formatBytes } from "@/lib/format";
import { EmptyState, LoadingRows } from "@/ui/states";

function formatDurationMs(ms: number | null | undefined): string | null {
  if (!ms || ms <= 0) return null;
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function ShareViewerPage() {
  const { t, i18n } = useTranslation();
  const { token } = useParams({ from: "/s/$token" });
  const locale = i18n.resolvedLanguage ?? "zh-CN";

  const [proof, setProof] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [activeMediaId, setActiveMediaId] = useState<string | null>(null);
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);

  const shareQuery = usePublicShare(token, proof);
  const unlock = useUnlockPublicShare(token);

  const handleUnlock = (e: React.FormEvent) => {
    e.preventDefault();
    if (!password) return;
    setPasswordError(null);
    unlock.mutate(password, {
      onSuccess: (res) => {
        setProof(res.proof);
        setPassword("");
      },
      onError: (err) => {
        if (err instanceof ApiError && err.status === 401) {
          setPasswordError(t("sharesApp.viewer.invalidPassword"));
        } else {
          setPasswordError(err instanceof Error ? err.message : String(err));
        }
      },
    });
  };

  if (shareQuery.isPending) {
    return (
      <main className="mx-auto flex min-h-svh w-full max-w-5xl flex-col gap-6 bg-page px-4 py-10 text-fg sm:px-6">
        <LoadingRows rows={4} />
      </main>
    );
  }

  if (shareQuery.isError) {
    const err = shareQuery.error;
    const isGone = err instanceof ApiError && err.status === 410;
    return (
      <main className="flex min-h-svh items-center justify-center bg-page p-6 text-fg">
        <div className="w-full max-w-md rounded-panel border border-line bg-surface p-8 text-center shadow-panel">
          <div className="mx-auto mb-4 grid size-11 place-items-center rounded-tile bg-sunken text-fg-muted">
            <Clock className="size-5" aria-hidden />
          </div>
          <h1 className="font-heading text-lg font-title text-heading">
            {isGone ? t("sharesApp.viewer.expiredTitle") : t("sharesApp.viewer.notFoundTitle")}
          </h1>
          <p className="mt-2 text-sm text-fg-muted">
            {isGone ? t("sharesApp.viewer.expiredDetail") : t("sharesApp.viewer.notFoundDetail")}
          </p>
        </div>
      </main>
    );
  }

  const share = shareQuery.data;
  if (!share) return null;

  if (share.locked) {
    return (
      <main className="flex min-h-svh items-center justify-center bg-page p-6 text-fg">
        <div className="grid w-full max-w-md gap-5 rounded-panel border border-line bg-surface p-7 shadow-panel">
          <div className="flex items-center gap-3">
            <span className="grid size-10 shrink-0 place-items-center rounded-tile bg-sunken text-fg">
              <Lock className="size-5" aria-hidden />
            </span>
            <div className="min-w-0">
              <p className="text-xs font-mid text-fg-muted">{t("sharesApp.viewer.sharedVia")}</p>
              <h1 className="truncate font-heading text-lg font-title text-heading">{share.title}</h1>
            </div>
          </div>

          <p className="text-sm text-fg-muted">{t("sharesApp.viewer.lockedDetail")}</p>

          <form onSubmit={handleUnlock} className="grid gap-3">
            <div className="grid gap-1.5">
              <label htmlFor="share-unlock-password" className="text-sm font-mid text-fg">
                {t("sharesApp.viewer.passwordInput")}
              </label>
              <input
                id="share-unlock-password"
                type="password"
                autoFocus
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="h-10 rounded-control border border-line-strong bg-surface px-3 text-sm text-fg focus-visible:outline-2 focus-visible:outline-(--pv-focus)"
              />
            </div>
            {passwordError ? (
              <p role="alert" className="text-sm font-mid text-failed">
                {passwordError}
              </p>
            ) : null}
            <Button type="submit" variant="primary" isDisabled={unlock.isPending || !password}>
              {t("sharesApp.viewer.unlockButton")}
            </Button>
          </form>
        </div>
      </main>
    );
  }

  const activeProof = share.proof ?? proof ?? "";
  const videos = share.items.filter((i) => i.mediaKind === "video");
  const audios = share.items.filter((i) => i.mediaKind === "audio");
  const photos = share.items.filter((i) => i.mediaKind === "photo");
  const booksAndComics = share.items.filter((i) => i.mediaKind === "book" || i.mediaKind === "comic");

  const selectedVideo: PublicShareItem | null =
    videos.find((v) => v.itemId === activeMediaId) ?? videos[0] ?? null;
  const selectedAudio: PublicShareItem | null =
    audios.find((a) => a.itemId === activeMediaId) ?? audios[0] ?? null;
  const lightboxPhoto = lightboxIndex !== null ? photos[lightboxIndex] ?? null : null;

  return (
    <main className="min-h-svh bg-page text-fg">
      <header className="border-b border-line bg-surface">
        <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-4 px-4 py-5 sm:px-6">
          <div className="grid min-w-0 gap-1">
            <div className="flex flex-wrap items-center gap-2 text-xs font-mid text-fg-muted">
              <span>{t("sharesApp.viewer.sharedVia")}</span>
              <span aria-hidden>·</span>
              <span>{t(`sharesApp.targetKinds.${share.targetKind}`)}</span>
              <span aria-hidden>·</span>
              <span data-num>{t("sharesApp.viewer.itemCount", { count: share.items.length })}</span>
            </div>
            <h1 className="truncate font-heading text-2xl font-title text-heading">{share.title}</h1>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {share.stripGps ? (
              <span className="inline-flex items-center gap-1.5 rounded-chip bg-sunken px-2.5 py-1 text-xs font-mid text-fg-muted">
                <ShieldCheck className="size-3.5 text-ok" aria-hidden />
                {t("sharesApp.viewer.gpsHidden")}
              </span>
            ) : null}
            {share.expiresAt ? (
              <span className="inline-flex items-center gap-1.5 rounded-chip bg-sunken px-2.5 py-1 text-xs font-mid text-fg-muted tabular-nums">
                <Clock className="size-3.5" aria-hidden />
                {t("sharesApp.expiresAt", {
                  time: new Date(share.expiresAt).toLocaleDateString(locale),
                })}
              </span>
            ) : null}
          </div>
        </div>
      </header>

      <div className="mx-auto grid w-full max-w-6xl gap-8 px-4 py-7 sm:px-6">
        {share.items.length === 0 ? (
          <EmptyState icon={FolderOpen} title={t("sharesApp.viewer.emptyItems")} />
        ) : null}

        {/* Inline HTML5 Video Player */}
        {selectedVideo ? (
          <section className="grid gap-4">
            <div className="overflow-hidden rounded-panel border border-line bg-neutral-950 shadow-panel">
              <video
                key={selectedVideo.itemId}
                controls
                playsInline
                preload="metadata"
                poster={publicShareCoverUrl(token, selectedVideo.itemId, activeProof, 960)}
                src={publicShareMediaUrl(token, selectedVideo.itemId, activeProof)}
                className="aspect-video max-h-[72vh] w-full bg-black"
              />
              <div className="flex flex-wrap items-center justify-between gap-3 border-t border-white/10 bg-neutral-900 px-4 py-3 text-white">
                <div className="min-w-0">
                  <h2 className="truncate text-base font-strong">{selectedVideo.title}</h2>
                  {selectedVideo.subtitle ? (
                    <p className="text-xs text-white/70">{selectedVideo.subtitle}</p>
                  ) : null}
                </div>
                {share.allowDownload ? (
                  <a
                    href={publicShareMediaUrl(token, selectedVideo.itemId, activeProof, { download: true })}
                    download
                    className={buttonVariants({ variant: "outline", size: "sm" })}
                  >
                    <Download className="size-4" aria-hidden />
                    {t("sharesApp.viewer.download")}
                  </a>
                ) : null}
              </div>
            </div>

            {videos.length > 1 ? (
              <ul className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {videos.map((v) => {
                  const active = v.itemId === selectedVideo.itemId;
                  return (
                    <li key={v.itemId}>
                      <button
                        type="button"
                        onClick={() => setActiveMediaId(v.itemId)}
                        className={cn(
                          "flex w-full items-center gap-3 rounded-tile border p-3 text-start transition-colors",
                          active
                            ? "border-line-strong bg-action text-action-fg"
                            : "border-line bg-surface hover:bg-hover",
                        )}
                      >
                        <Film className="size-4 shrink-0" aria-hidden />
                        <span className="min-w-0 flex-1 truncate text-sm font-mid">{v.title}</span>
                        {v.subtitle ? (
                          <span className="text-xs opacity-75 tabular-nums">{v.subtitle}</span>
                        ) : null}
                      </button>
                    </li>
                  );
                })}
              </ul>
            ) : null}
          </section>
        ) : null}

        {/* Inline HTML5 Audio Player & Tracklist */}
        {selectedAudio ? (
          <section className="grid gap-4 rounded-panel border border-line bg-surface p-5 shadow-panel">
            <div className="flex flex-col gap-4 sm:flex-row sm:items-center">
              <div className="size-20 shrink-0 overflow-hidden rounded-tile bg-sunken">
                <img
                  src={publicShareCoverUrl(token, selectedAudio.itemId, activeProof, 240)}
                  alt={selectedAudio.title}
                  onError={(e) => {
                    e.currentTarget.style.display = "none";
                  }}
                  className="size-full object-cover"
                />
              </div>
              <div className="grid min-w-0 flex-1 gap-2">
                <div>
                  <h2 className="truncate font-heading text-lg font-strong text-heading">
                    {selectedAudio.title}
                  </h2>
                  {selectedAudio.subtitle ? (
                    <p className="truncate text-sm text-fg-muted">{selectedAudio.subtitle}</p>
                  ) : null}
                </div>
                <audio
                  key={selectedAudio.itemId}
                  controls
                  preload="metadata"
                  src={publicShareMediaUrl(token, selectedAudio.itemId, activeProof)}
                  className="h-10 w-full"
                />
              </div>
              {share.allowDownload ? (
                <a
                  href={publicShareMediaUrl(token, selectedAudio.itemId, activeProof, { download: true })}
                  download
                  className={buttonVariants({ variant: "outline", size: "sm" })}
                >
                  <Download className="size-4" aria-hidden />
                  {t("sharesApp.viewer.download")}
                </a>
              ) : null}
            </div>

            {audios.length > 1 ? (
              <ol className="divide-y divide-line border-t border-line pt-2">
                {audios.map((track, idx) => {
                  const current = track.itemId === selectedAudio.itemId;
                  const dur = formatDurationMs(track.durationMs);
                  return (
                    <li key={track.itemId}>
                      <button
                        type="button"
                        onClick={() => setActiveMediaId(track.itemId)}
                        className={cn(
                          "flex w-full items-center gap-3 rounded-control px-3 py-2.5 text-start transition-colors",
                          current ? "bg-sunken font-strong text-heading" : "hover:bg-hover",
                        )}
                      >
                        <span data-num className="w-6 text-xs text-fg-muted tabular-nums">
                          {current ? <Play className="size-3.5 fill-current" aria-hidden /> : idx + 1}
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-sm">{track.title}</span>
                          {track.subtitle ? (
                            <span className="block truncate text-xs text-fg-muted">{track.subtitle}</span>
                          ) : null}
                        </span>
                        {dur ? (
                          <span data-num className="text-xs text-fg-muted tabular-nums">
                            {dur}
                          </span>
                        ) : null}
                      </button>
                    </li>
                  );
                })}
              </ol>
            ) : null}
          </section>
        ) : null}

        {/* Photo Grid & Lightbox (GPS stripped when `share.stripGps` is true) */}
        {photos.length > 0 ? (
          <section className="grid gap-4">
            <ul className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
              {photos.map((photo, idx) => (
                <li key={photo.itemId}>
                  <button
                    type="button"
                    onClick={() => setLightboxIndex(idx)}
                    className="group relative aspect-square w-full overflow-hidden rounded-tile border border-line bg-sunken outline-none focus-visible:ring-2 focus-visible:ring-(--pv-focus)"
                  >
                    <img
                      src={publicShareMediaUrl(token, photo.itemId, activeProof, { size: 480 })}
                      alt={photo.title}
                      loading="lazy"
                      className="size-full object-cover transition-transform duration-200 group-hover:scale-[1.03]"
                    />
                    <span className="pointer-events-none absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent px-2.5 py-2 text-start text-xs text-white opacity-0 transition-opacity group-hover:opacity-100">
                      <span className="block truncate font-mid">{photo.title}</span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {/* Books & Comics (Downloadable when `allowDownload` is true) */}
        {booksAndComics.length > 0 ? (
          <section className="grid gap-3 sm:grid-cols-2">
            {booksAndComics.map((entry) => (
              <div
                key={entry.itemId}
                className="flex items-center gap-4 rounded-panel border border-line bg-surface p-4 shadow-panel"
              >
                <div className="grid size-16 shrink-0 place-items-center overflow-hidden rounded-tile bg-sunken text-fg-muted">
                  {entry.hasCover ? (
                    <img
                      src={publicShareCoverUrl(token, entry.itemId, activeProof, 240)}
                      alt={entry.title}
                      className="size-full object-cover"
                    />
                  ) : (
                    <BookOpen className="size-6" aria-hidden />
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <h3 className="truncate font-strong text-heading">{entry.title}</h3>
                  <p className="truncate text-xs text-fg-muted">
                    {[
                      entry.subtitle,
                      entry.format?.toUpperCase(),
                      entry.sizeBytes != null ? formatBytes(entry.sizeBytes, locale) : null,
                    ]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                </div>
                {share.allowDownload ? (
                  <a
                    href={publicShareMediaUrl(token, entry.itemId, activeProof, { download: true })}
                    download
                    className={buttonVariants({ variant: "primary", size: "sm" })}
                  >
                    <Download className="size-4" aria-hidden />
                    {t("sharesApp.viewer.download")}
                  </a>
                ) : null}
              </div>
            ))}
          </section>
        ) : null}
      </div>

      {/* Photo Lightbox Modal */}
      {lightboxPhoto ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-label={lightboxPhoto.title}
          className="fixed inset-0 z-50 flex flex-col bg-black/92 text-white"
          onClick={(e) => {
            if (e.target === e.currentTarget) setLightboxIndex(null);
          }}
        >
          <div className="flex items-center justify-between gap-3 px-4 py-3">
            <div className="min-w-0">
              <p className="truncate text-sm font-strong">{lightboxPhoto.title}</p>
              <div className="flex flex-wrap items-center gap-2 text-xs text-white/70">
                {lightboxPhoto.camera ? <span>{lightboxPhoto.camera}</span> : null}
                {!share.stripGps &&
                lightboxPhoto.latitude != null &&
                lightboxPhoto.longitude != null ? (
                  <span className="inline-flex items-center gap-1 tabular-nums">
                    <MapPin className="size-3" aria-hidden />
                    {lightboxPhoto.latitude.toFixed(4)}, {lightboxPhoto.longitude.toFixed(4)}
                  </span>
                ) : null}
              </div>
            </div>
            <div className="flex items-center gap-2">
              {share.allowDownload ? (
                <a
                  href={publicShareMediaUrl(token, lightboxPhoto.itemId, activeProof, { download: true })}
                  download
                  className="inline-flex h-8 items-center gap-1.5 rounded-control bg-white/15 px-3 text-xs font-mid text-white hover:bg-white/25"
                >
                  <Download className="size-3.5" aria-hidden />
                  {t("sharesApp.viewer.download")}
                </a>
              ) : null}
              <button
                type="button"
                aria-label={t("sharesApp.viewer.closePreview")}
                onClick={() => setLightboxIndex(null)}
                className="grid size-8 place-items-center rounded-control bg-white/15 text-white hover:bg-white/25"
              >
                <X className="size-4" aria-hidden />
              </button>
            </div>
          </div>
          <div className="grid min-h-0 flex-1 place-items-center p-4">
            <img
              src={publicShareMediaUrl(token, lightboxPhoto.itemId, activeProof, { size: 1920 })}
              alt={lightboxPhoto.title}
              className="max-h-full max-w-full rounded-tile object-contain"
            />
          </div>
        </div>
      ) : null}

      <span className="sr-only">
        <Disc3 />
        <Music2 />
        <ImageIcon />
      </span>
    </main>
  );
}
