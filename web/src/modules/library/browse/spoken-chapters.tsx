import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { useFeature } from "@/modules/access";
import { ErrorState } from "@/ui/states";

type Props = { id: string; disabled: boolean; onSeek: (startMs: number) => void };

export function SpokenChapters({ id, disabled, onSeek }: Props) {
  const { t } = useTranslation();
  const enabled = useFeature("spokenChapters");
  const query = useQuery({
    queryKey: ["items", "spoken-chapters", id],
    queryFn: ({ signal }) => request<components["schemas"]["SpokenChapterList"]>(
      `/items/${encodeURIComponent(id)}/chapters`, { signal },
    ),
    enabled, retry: false, gcTime: 0,
  });
  if (!enabled || query.isPending) return null;
  if (query.isError) return <ErrorState compact error={query.error} onRetry={() => void query.refetch()} />;
  if (!query.data.chapters.length) return null;
  return <select aria-label={t("spoken.jumpToChapter")} disabled={disabled} value="" onChange={(event) => {
    if (event.target.value !== "") onSeek(Number(event.target.value));
  }} className="pv-select pv-select--sm w-fit min-w-0 max-w-full">
    <option disabled value="">{t("spoken.jumpToChapter")}</option>
    {query.data.chapters.map((chapter, index) => <option key={chapter.startMs} value={chapter.startMs}>
      {chapter.title ?? t("spoken.numberedChapter", { number: index + 1 })}
    </option>)}
  </select>;
}
