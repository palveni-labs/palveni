import { useTranslation } from "react-i18next";
import type { LibraryKind } from "@/api/types";
import { formatNumber } from "@/lib/format";

/** "1,284 项" / "1,284 items" with locale grouping. */
export function useItemCountLabel() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  return (count: number) => t("counts.items", { count, formatted: formatNumber(count, locale) });
}

/** "187 个视频" / "1,090 tracks": a library's count in its own unit. */
export function useKindCountLabel() {
  const { t, i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? "en";
  return (count: number, kind: LibraryKind) => `${formatNumber(count, locale)} ${t(`units.${kind}`)}`;
}
