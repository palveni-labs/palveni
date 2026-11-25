import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";

export const systemInfoQuery = () =>
  queryOptions({
    queryKey: queryKeys.system.info,
    queryFn: ({ signal }) => api.system.info(signal),
    staleTime: 5 * 60_000,
  });

export const hardwareQuery = () =>
  queryOptions({
    queryKey: queryKeys.system.hardware,
    queryFn: ({ signal }) => api.system.hardware(false, signal),
    staleTime: 5 * 60_000,
  });

export function useSystemInfo() {
  return useQuery(systemInfoQuery());
}

export function useHardware(options: { enabled?: boolean } = {}) {
  return useQuery({ ...hardwareQuery(), enabled: options.enabled ?? true });
}

/** Re-runs hardware detection on the server (`?refresh=true`) and stores the new report. */
export function useRefreshHardware() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => api.system.hardware(true),
    onSuccess: (report) => queryClient.setQueryData(queryKeys.system.hardware, report),
  });
}

export function useRuntimeSettings(enabled = true) {
  return useQuery({
    queryKey: queryKeys.system.settings,
    queryFn: ({ signal }) => api.system.settings(signal),
    enabled,
  });
}

export function useUpdateRuntimeSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: api.system.updateSettings,
    onSuccess: (settings) => {
      client.setQueryData(queryKeys.system.settings, settings);
      void client.invalidateQueries({ queryKey: queryKeys.system.info });
      void client.invalidateQueries({ queryKey: queryKeys.system.hardware });
    },
  });
}
