import { Button, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { isActiveJob, useJobs } from "@/api/queries/jobs";
import { queryKeys } from "@/api/query-keys";
import { errorText } from "@/lib/errors";
import { ErrorState } from "@/ui/states";

export function AnalyzeLoudness({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation();
  const client = useQueryClient();
  const { data } = useJobs();
  const active = data?.jobs.find((j) => j.kind === "music.loudness" && j.subject?.id === libraryId && isActiveJob(j));
  const action = useMutation({
    mutationFn: async (jobId: string | undefined) => {
      if (jobId) {
        await request(`/jobs/${encodeURIComponent(jobId)}/cancel`, { method: "POST" });
      } else {
        await request(`/libraries/${encodeURIComponent(libraryId)}/music/loudness`, { method: "POST" });
      }
    },
    onSuccess: (_, jobId) => {
      void client.invalidateQueries({ queryKey: queryKeys.jobs.all });
      toast.success(t(jobId ? "music.loudnessStopping" : "music.loudnessQueued"));
    },
    onError: (error) => toast.danger(errorText(t, error)),
  });
  return <Button size="sm" variant="outline" isDisabled={action.isPending} onPress={() => action.mutate(active?.id)}>
    {t(active ? "music.stopLoudness" : "music.analyzeLoudness")}
  </Button>;
}

export function TrackLoudness({ id, libraryId }: { id: string; libraryId: string }) {
  const { t, i18n } = useTranslation();
  const query = useQuery({
    queryKey: [...queryKeys.items.loudness(libraryId), id],
    queryFn: ({ signal }) => request<components["schemas"]["TrackLoudness"]>(`/items/${encodeURIComponent(id)}/loudness`, { signal }),
    staleTime: 0,
  });
  if (query.isError) return <ErrorState error={query.error} onRetry={() => void query.refetch()} compact />;
  if (!query.data) return <span>…</span>;
  const value = query.data.measurement;
  if (!value) return <span>{t("music.loudnessPending")}</span>;
  const number = new Intl.NumberFormat(i18n.resolvedLanguage, { maximumFractionDigits: 2 });
  const values = [
    value.integratedLufs == null ? t("music.belowGate") : `${number.format(value.integratedLufs)} LUFS`,
    value.rangeLu == null ? null : `${number.format(value.rangeLu)} LU`,
    value.truePeakDbtp == null ? null : `${number.format(value.truePeakDbtp)} dBTP`,
  ].filter(Boolean).join(" · ");
  return <span>{values}{!value.current && ` · ${t("music.loudnessStale")}`}</span>;
}
