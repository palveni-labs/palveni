import { Button, Drawer } from "@heroui/react";
import { Sparkles } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { Item, Library } from "@/api/types";
import { ItemArtwork } from "@/components/artwork";
import { formatBytes, formatDateTime } from "@/lib/format";
import { LIBRARY_KIND_META } from "@/lib/kinds";
import { useFeature } from "@/modules/access";
import { shell } from "@/shell/shell-store";
import { Measure } from "@/ui/marks";
import { BookDownloadButton } from "./book-download";
import { TrackLoudness } from "./music-loudness";
import { useIsPhone } from "./use-is-phone";

function Fact({ label, children, wide = false }: { label: string; children: ReactNode; wide?: boolean }) {
  return (
    <div className={cn("grid gap-0.5 py-2.5", wide && "col-span-2")}>
      <dt className="text-sm text-fg-muted">{label}</dt>
      <dd className="min-w-0 text-base text-fg">{children}</dd>
    </div>
  );
}

/** Read-only facts about one file. The workspace previews libraries; it does not play media. */
export function ItemDetailsSheet({ item, library, onClose }: { item: Item | null; library: Library; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  const phone = useIsPhone();
  const downloads = useFeature("bookDownloads");
  const canDownload = downloads && item?.kind === "book" && library.kind === "book";
  const loudness = useFeature("musicLoudness");
  const assistantEnabled = useFeature("assistant");

  return (
    <Drawer.Backdrop isOpen={item !== null} onOpenChange={(open) => (open ? undefined : onClose())}>
      <Drawer.Content placement={phone ? "bottom" : "right"}>
        <Drawer.Dialog aria-label={item?.title ?? t("browse.details.title")} className={cn("gap-0 p-0", !phone && "w-[26rem] max-w-[90vw]")}>
          {phone ? <Drawer.Handle /> : null}
          {item ? (
            <>
              <Drawer.CloseTrigger aria-label={t("a11y.close")} className="absolute top-3 end-3 z-10" />
              <Drawer.Header className="flex items-start gap-4 border-b border-line px-6 pt-6 pb-5">
                <ItemArtwork item={item} className={cn("w-20 shrink-0 rounded-tile ring-1 ring-line ring-inset", LIBRARY_KIND_META[library.kind].aspectClass)} />
                <div className="grid min-w-0 flex-1 gap-1 pe-8">
                  <Drawer.Heading className="font-heading text-lg font-title break-words text-heading">{item.title}</Drawer.Heading>
                  <p className="text-sm text-fg-muted">
                    {t(`itemKinds.${item.kind}`)}
                    <span aria-hidden> · </span>
                    {library.name}
                  </p>
                </div>
              </Drawer.Header>
              <Drawer.Body className="overflow-y-auto px-6 py-2">
                <dl className="grid grid-cols-2 gap-x-6 divide-line">
                  <Fact label={t("browse.details.size")}>{item.sizeBytes !== null ? <Measure value={formatBytes(item.sizeBytes, locale)} /> : "—"}</Fact>
                  <Fact label={t("browse.details.kind")}>{t(`itemKinds.${item.kind}`)}</Fact>
                  <Fact label={t("browse.details.modified")}>
                    <span className="tabular-nums">{item.modifiedAt ? formatDateTime(item.modifiedAt, locale) : "—"}</span>
                  </Fact>
                  <Fact label={t("browse.details.indexed")}>
                    <span className="tabular-nums">{formatDateTime(item.createdAt, locale)}</span>
                  </Fact>
                  {loudness && library.kind === "music" && item.kind === "track" && <Fact label={t("music.loudness")} wide><TrackLoudness id={item.id} libraryId={library.id} /></Fact>}
                  {item.path ? (
                    <Fact label={t("browse.details.path")} wide>
                      <span className="font-mono text-sm break-all">{item.path}</span>
                    </Fact>
                  ) : null}
                  <Fact label={t("browse.details.id")} wide>
                    <span className="text-sm break-all text-fg-muted select-all">{item.id}</span>
                  </Fact>
                </dl>
              </Drawer.Body>
              {assistantEnabled || canDownload ? (
                <Drawer.Footer className="border-t border-line px-6 py-4">
                  {canDownload && <BookDownloadButton key={item.id} id={item.id} />}
                  {assistantEnabled && <Button
                    variant="primary"
                    onPress={() => {
                      shell.openAssistant({ label: t("browse.handoffOne", { title: item.title, library: library.name }), itemIds: [item.id] });
                      onClose();
                    }}
                  >
                    <Sparkles aria-hidden />
                    {t("browse.handToAssistant")}
                  </Button>}
                </Drawer.Footer>
              ) : null}
            </>
          ) : null}
        </Drawer.Dialog>
      </Drawer.Content>
    </Drawer.Backdrop>
  );
}
