import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
} from "@tanstack/react-query";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";
import type { CreateLibraryRequest, Library, LibraryList } from "../types";

export const librariesQuery = () =>
  queryOptions({
    queryKey: queryKeys.libraries.list,
    queryFn: ({ signal }) => api.libraries.list(signal),
    staleTime: 30_000,
  });

export const libraryQuery = (queryClient: QueryClient, id: string) =>
  queryOptions({
    queryKey: queryKeys.libraries.detail(id),
    queryFn: ({ signal }) => api.libraries.get(id, signal),
    staleTime: 30_000,
    initialData: () =>
      queryClient.getQueryData<LibraryList>(queryKeys.libraries.list)?.libraries.find((library) => library.id === id),
    initialDataUpdatedAt: () => queryClient.getQueryState(queryKeys.libraries.list)?.dataUpdatedAt,
  });

export function useLibraries() {
  return useQuery(librariesQuery());
}

export function useLibrary(id: string) {
  const queryClient = useQueryClient();
  return useQuery(libraryQuery(queryClient, id));
}

/** Inserts or replaces a library in every cache that holds it. */
export function upsertLibrary(queryClient: QueryClient, library: Library) {
  queryClient.setQueryData(queryKeys.libraries.detail(library.id), library);
  queryClient.setQueryData<LibraryList>(queryKeys.libraries.list, (list) => {
    if (!list) return list;
    const exists = list.libraries.some((entry) => entry.id === library.id);
    return {
      libraries: exists
        ? list.libraries.map((entry) => (entry.id === library.id ? library : entry))
        : [...list.libraries, library],
    };
  });
}

/** Drops a deleted library and everything browsed inside it. */
export function removeLibrary(queryClient: QueryClient, id: string) {
  queryClient.setQueryData<LibraryList>(queryKeys.libraries.list, (list) =>
    list ? { libraries: list.libraries.filter((entry) => entry.id !== id) } : list,
  );
  queryClient.removeQueries({ queryKey: queryKeys.libraries.detail(id) });
  queryClient.removeQueries({ queryKey: queryKeys.libraries.scanJob(id) });
  queryClient.removeQueries({ queryKey: queryKeys.items.libraryLists(id) });
}

export function useCreateLibrary() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateLibraryRequest) => api.libraries.create(body),
    onSuccess: (library) => upsertLibrary(queryClient, library),
  });
}

export function useDeleteLibrary() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.libraries.remove(id),
    onSuccess: (_result, id) => removeLibrary(queryClient, id),
  });
}

export function useScanLibrary() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.libraries.scan(id),
    onSuccess: ({ jobId }, id) => {
      queryClient.setQueryData(queryKeys.libraries.scanJob(id), jobId);
      void queryClient.invalidateQueries({ queryKey: queryKeys.jobs.all });
    },
  });
}

/** The id of the scan job this browser last started for a library, if any. */
export function useLibraryScanJobId(id: string): string | undefined {
  const { data } = useQuery<string | null>({
    queryKey: queryKeys.libraries.scanJob(id),
    queryFn: () => null,
    enabled: false,
    staleTime: Infinity,
    gcTime: Infinity,
  });
  return data ?? undefined;
}
