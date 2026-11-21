import { AlertDialog, Button, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import { isApiError } from "@/api/errors";
import { useIsAdmin } from "@/api/queries/auth";
import { queryKeys } from "@/api/query-keys";
import type { Library, ScanReview } from "@/api/types";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { useSecureSurface } from "@/skins/secure";
import { Section } from "@/ui/section";
import { ErrorState } from "@/ui/states";
import { useLibraryScan } from "../use-library-scan";

const PAGE_SIZE = 50;

export function ScanReviews({ library }: { library: Library }) {
  const admin = useIsAdmin();
  const enabled = useFeature("scanReviews");
  return admin && enabled ? <PendingReviews library={library} /> : null;
}

function PendingReviews({ library }: { library: Library }) {
  const { t } = useTranslation();
  const [selected, select] = useState<ScanReview | null>(null);
  const { running } = useLibraryScan(library);
  const list = useQuery({
    queryKey: queryKeys.libraries.scanReviews(library.id),
    queryFn: ({ signal }) => api.libraries.scanReviews(library.id, signal),
  });
  if (!list.data && !list.isError) return null;
  const reviews = list.data?.reviews ?? [];
  return <>
    {(reviews.length > 0 || list.isError) && <Section title={t("scanReviews.title")} raised>
      {list.isError && <ErrorState error={list.error} onRetry={() => void list.refetch()} compact />}
      {reviews.length > 0 && <>
        <p className="text-sm text-fg-muted">{t("scanReviews.kept")}</p>
        <ul className="divide-y divide-line">
          {reviews.map((review) => <li key={review.revision} className="flex flex-wrap items-center justify-between gap-3 py-3">
            <div className="min-w-0">
              <p className="font-mono text-sm break-all text-fg">{review.scope}</p>
              <p className="text-sm text-fg-muted">{t("scanReviews.missing", { count: review.missingCount })}</p>
            </div>
            <Button size="sm" variant="outline" aria-label={t("scanReviews.openScope", { scope: review.scope })} onPress={() => select(review)}>
              {t("scanReviews.open")}
            </Button>
          </li>)}
        </ul>
      </>}
    </Section>}
    {selected && <ReviewDialog key={selected.revision} libraryId={library.id} review={selected}
      current={reviews.some((row) => row.revision === selected.revision)}
      running={running} close={() => select(null)} />}
  </>;
}

function conflict(error: unknown) {
  return isApiError(error) && error.status === 409;
}

function ReviewDialog({ libraryId, review, current, running, close }: {
  libraryId: string; review: ScanReview; current: boolean; running: boolean; close: () => void;
}) {
  const { t } = useTranslation();
  const secure = useSecureSurface();
  const client = useQueryClient();
  const [offset, setOffset] = useState(0);
  const page = useQuery({
    queryKey: queryKeys.libraries.reviewPaths(libraryId, review.revision, offset),
    queryFn: ({ signal }) => api.libraries.scanReviewPaths(libraryId, review.revision, offset, PAGE_SIZE, signal),
    enabled: current,
    retry: false,
  });
  const confirm = useMutation({
    mutationFn: () => api.libraries.confirmScanReview(libraryId, review.revision),
    onSuccess: ({ jobId }) => {
      client.setQueryData(queryKeys.libraries.scanJob(libraryId), jobId);
      void client.invalidateQueries({ queryKey: queryKeys.jobs.all });
      void client.invalidateQueries({ queryKey: queryKeys.libraries.scanReviews(libraryId), exact: true });
      toast.success(t("scanReviews.queued"));
      close();
    },
  });
  const rejected = conflict(confirm.error) || conflict(page.error);
  useEffect(() => {
    if (rejected) {
      void client.invalidateQueries({ queryKey: queryKeys.libraries.scanReviews(libraryId), exact: true });
      void client.invalidateQueries({ queryKey: queryKeys.jobs.all });
    }
  }, [client, libraryId, rejected]);
  const unavailable = !current || rejected;
  const total = page.data?.total ?? review.missingCount;
  return (
    <AlertDialog.Backdrop isOpen onOpenChange={(open) => { if (!open && !confirm.isPending) close(); }} isDismissable={!confirm.isPending}>
      <AlertDialog.Container size="md">
        <AlertDialog.Dialog {...secure}>
          <AlertDialog.Header>
            <AlertDialog.Icon status="danger"><TriangleAlert aria-hidden className="size-5" /></AlertDialog.Icon>
            <AlertDialog.Heading>{t("scanReviews.open")}</AlertDialog.Heading>
          </AlertDialog.Header>
          <AlertDialog.Body className="grid gap-3">
            <p className="font-mono text-sm break-all">{review.scope}</p>
            {unavailable ? <p role="alert">{t("scanReviews.changed")}</p> : <>
              <p>{t("scanReviews.confirmHint", { count: review.missingCount })}</p>
              {page.isPending ? <p role="status">{t("common.loading")}</p> : page.isError ?
                <ErrorState error={page.error} onRetry={() => void page.refetch()} compact /> : (
                <ul aria-label={t("scanReviews.paths")} className="max-h-[32vh] overflow-auto rounded-tile bg-sunken px-3 py-2">
                  {page.data.paths.map((path) => <li key={path} className="py-1 font-mono text-sm break-all">{path}</li>)}
                </ul>
              )}
              {total > PAGE_SIZE && <nav aria-label={t("scanReviews.pages")} className="flex flex-wrap items-center justify-between gap-2">
                <Button size="sm" variant="outline" isDisabled={offset === 0 || confirm.isPending} onPress={() => setOffset(offset - PAGE_SIZE)}>{t("scanReviews.previous")}</Button>
                <span role="status" className="text-sm tabular-nums">{t("scanReviews.range", { start: offset + 1, end: Math.min(offset + PAGE_SIZE, total), total })}</span>
                <Button size="sm" variant="outline" isDisabled={offset + PAGE_SIZE >= total || confirm.isPending} onPress={() => setOffset(offset + PAGE_SIZE)}>{t("scanReviews.next")}</Button>
              </nav>}
              {running && <p role="status">{t("scanReviews.busy")}</p>}
              {confirm.isError && <p role="alert">{errorText(t, confirm.error)}</p>}
            </>}
          </AlertDialog.Body>
          <AlertDialog.Footer>
            <Button variant="secondary" onPress={close} isDisabled={confirm.isPending}>{t("common.cancel")}</Button>
            {!unavailable && <Button variant="danger" isDisabled={running || !page.isSuccess || confirm.isPending} onPress={() => confirm.mutate()}>
              {t("scanReviews.confirm", { count: review.missingCount })}
            </Button>}
          </AlertDialog.Footer>
        </AlertDialog.Dialog>
      </AlertDialog.Container>
    </AlertDialog.Backdrop>
  );
}
