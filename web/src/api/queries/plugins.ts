import { queryOptions, useQuery } from "@tanstack/react-query";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";

export const pluginsQuery = () =>
  queryOptions({
    queryKey: queryKeys.plugins.list,
    queryFn: ({ signal }) => api.plugins.list(signal),
    staleTime: 60_000,
  });

export function usePlugins() {
  return useQuery(pluginsQuery());
}
