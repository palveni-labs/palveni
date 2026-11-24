import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { provisionalApi } from "../api";
import { provisionalKeys } from "../keys";
import type { StorageReport } from "../types";

export function useCatalog(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.catalog,
    queryFn: ({ signal }) => provisionalApi.artwork.catalog(signal),
    enabled,
    staleTime: 60_000,
  });
}

export function useMetrics(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.metrics,
    queryFn: ({ signal }) => provisionalApi.metrics.current(signal),
    enabled,
    refetchInterval: 10_000,
    staleTime: 5_000,
  });
}

export function useReconnectMount() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (mountId: string) => provisionalApi.storage.reconnect(mountId),
    onSuccess: (mount) =>
      queryClient.setQueryData<StorageReport>(provisionalKeys.storage, (report) =>
        report ? { ...report, mounts: report.mounts.map((entry) => (entry.id === mount.id ? mount : entry)) } : report,
      ),
  });
}
