import { useTranslation } from "react-i18next";
import { formatDateTime, formatRelativeTime } from "./format";
import { useNow } from "./use-now";

/** Relative label ("3 hours ago") plus the absolute time for tooltips; re-renders as time passes. */
export function useRelativeTime(value: string): { label: string; title: string } {
  const { t, i18n } = useTranslation();
  const now = useNow();
  const locale = i18n.resolvedLanguage ?? "en";
  return { label: formatRelativeTime(value, locale, now, t("time.justNow")), title: formatDateTime(value, locale) };
}
