import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/api/query-keys";
import { provisionalApi } from "../api";
import { provisionalKeys } from "../keys";
import type {
  CreateDownloaderClientRequest,
  CreateDownloadTaskRequest,
  CreateSubscriptionRequest,
  OrganizeApplyRequest,
  OrganizePreviewRequest,
} from "../types";

export function useDownloadsOverview(enabled = true) {
  return useQuery({
    queryKey: provisionalKeys.downloads,
    queryFn: ({ signal }) => provisionalApi.downloads.overview(signal),
    enabled,
    refetchInterval: 8_000,
  });
}

export function useSaveDownloaderClient() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateDownloaderClientRequest) => provisionalApi.downloads.saveClient(body),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: provisionalKeys.downloads }),
  });
}

export function useDeleteDownloaderClient() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.downloads.deleteClient(id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: provisionalKeys.downloads }),
  });
}

export function useCreateDownloadTask() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateDownloadTaskRequest) => provisionalApi.downloads.createTask(body),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: provisionalKeys.downloads }),
  });
}

export function useCreateSubscription() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateSubscriptionRequest) => provisionalApi.downloads.createSubscription(body),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: provisionalKeys.downloads }),
  });
}

export function useDeleteSubscription() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.downloads.deleteSubscription(id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: provisionalKeys.downloads }),
  });
}

export function useCheckSubscription() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.downloads.checkSubscription(id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: provisionalKeys.downloads }),
  });
}

export function useOrganizeBatches(enabled = true) {
  return useQuery({
    queryKey: provisionalKeys.organizerBatches,
    queryFn: ({ signal }) => provisionalApi.organizer.batches(signal),
    enabled,
  });
}

export function usePreviewOrganize() {
  return useMutation({
    mutationFn: (body: OrganizePreviewRequest) => provisionalApi.organizer.preview(body),
  });
}

export function useApplyOrganize() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: OrganizeApplyRequest) => provisionalApi.organizer.apply(body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: provisionalKeys.organizerBatches });
      void queryClient.invalidateQueries({ queryKey: queryKeys.libraries.all });
    },
  });
}

export function useUndoOrganizeBatch() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (batchId: string) => provisionalApi.organizer.undo(batchId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: provisionalKeys.organizerBatches });
      void queryClient.invalidateQueries({ queryKey: queryKeys.libraries.all });
    },
  });
}
