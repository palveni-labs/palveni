import { useTranslation } from "react-i18next";
import type { Library } from "@/api/types";
import { RelativeTime } from "@/components/relative-time";
import { StatusLabel } from "@/ui/marks";
import { useKindCountLabel } from "./use-item-count-label";

/** "187 个视频 · 3小时前扫描", "… · 扫描中" or "… · 从未扫描". */
export function LibraryMeta({ library, scanning = false, className }: { library: Library; scanning?: boolean; className?: string }) {
  const { t } = useTranslation();
  const count = useKindCountLabel();
  return (
    <span className={className}>
      <span data-num className="tabular-nums">
        {count(library.itemCount, library.kind)}
      </span>
      <span aria-hidden> · </span>
      {scanning ? (
        <StatusLabel tone="active">{t("libraries.scanning")}</StatusLabel>
      ) : library.lastScanAt ? (
        <RelativeTime value={library.lastScanAt} format={(time) => t("libraries.scannedAt", { time })} />
      ) : (
        <span>{t("libraries.neverScanned")}</span>
      )}
    </span>
  );
}
