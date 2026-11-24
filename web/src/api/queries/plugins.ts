import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";
import type { InstallPluginRequest, UpdatePluginRequest } from "../types";

export const pluginsQuery = () =>
  queryOptions({
    queryKey: queryKeys.plugins.list,
    queryFn: ({ signal }) => api.plugins.list(signal),
    staleTime: 60_000,
  });

export const pluginCatalogQuery = () =>
  queryOptions({
    queryKey: queryKeys.plugins.catalog,
    queryFn: ({ signal }) => api.plugins.catalog(signal),
    staleTime: 60_000,
  });

export function usePlugins(enabled = true) {
  return useQuery({ ...pluginsQuery(), enabled });
}

export function usePluginCatalog(enabled = true) {
  return useQuery({ ...pluginCatalogQuery(), enabled });
}

export function useInstallPlugin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: InstallPluginRequest) => api.plugins.install(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plugins.all });
    },
  });
}

export function useUpdatePlugin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdatePluginRequest }) => api.plugins.update(id, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plugins.all });
    },
  });
}

export function useRollbackPlugin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.plugins.rollback(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plugins.all });
    },
  });
}

export function useUninstallPlugin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.plugins.uninstall(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.plugins.all });
    },
  });
}
