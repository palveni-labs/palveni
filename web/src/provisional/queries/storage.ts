import { useQuery } from "@tanstack/react-query";
import { provisionalApi } from "../api";
import { provisionalKeys } from "../keys";

export function useStorage(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.storage,
    queryFn: ({ signal }) => provisionalApi.storage.report(signal),
    enabled,
    staleTime: 5 * 60_000,
  });
}
