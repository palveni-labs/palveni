import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLibraries } from "@/api/queries/libraries";
import type { Library } from "@/api/types";
import {
  readingApi,
  readingImagePaths,
  type CreateBookmarkRequest,
  type ReadingEntry,
  type ReadingKind,
  type ReadingListQuery,
  type ReadingProgressUpdate,
} from "./api";

export const readingKeys = {
  all: ["reading"] as const,
  home: (libraryId?: string) => ["reading", "home", libraryId ?? "all"] as const,
  lists: () => ["reading", "list"] as const,
  list: (libraryId: string, query: ReadingListQuery) => ["reading", "list", libraryId, query] as const,
  multiList: (libraryIds: string[], query: ReadingListQuery) => ["reading", "multi-list", libraryIds, query] as const,
  manifests: () => ["reading", "manifest"] as const,
  manifest: (itemId: string) => ["reading", "manifest", itemId] as const,
  chapter: (itemId: string, index: number) => ["reading", "chapter", itemId, index] as const,
  inline: (itemId: string) => ["reading", "inline", itemId] as const,
  image: (path: string) => ["reading", "image", path] as const,
};

export function useReadingLibraries(kind?: ReadingKind): {
  libraries: Library[];
  isLoading: boolean;
  error: unknown;
} {
  const libsQuery = useLibraries();
  const libraries = (libsQuery.data?.libraries ?? []).filter((lib) =>
    kind ? lib.kind === kind : lib.kind === "book" || lib.kind === "comic",
  );
  return {
    libraries,
    isLoading: libsQuery.isLoading,
    error: libsQuery.error,
  };
}

export function useReadingHome(libraryId?: string) {
  return useQuery({
    queryKey: readingKeys.home(libraryId),
    queryFn: ({ signal }) => readingApi.home(libraryId, signal),
  });
}

export function useReadingLibraryList(libraryId: string | undefined, query: ReadingListQuery) {
  return useInfiniteQuery({
    queryKey: readingKeys.list(libraryId ?? "", query),
    enabled: Boolean(libraryId),
    initialPageParam: null as string | null,
    queryFn: ({ pageParam, signal }) =>
      readingApi.list(libraryId!, { ...query, cursor: pageParam, limit: query.limit ?? 48 }, signal),
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
  });
}

/** Aggregates items across multiple reading libraries when "All Libraries" is selected. */
export function useReadingAcrossLibraries(libraries: Library[], query: ReadingListQuery, enabled = true) {
  const ids = libraries.map((l) => l.id);
  return useQuery({
    queryKey: readingKeys.multiList(ids, query),
    enabled: enabled && ids.length > 0,
    queryFn: async ({ signal }) => {
      const pages = await Promise.all(
        ids.map((id) => readingApi.list(id, { ...query, limit: query.limit ?? 100 }, signal)),
      );
      const entries: ReadingEntry[] = pages.flatMap((p) => p.entries);
      const total = pages.reduce((sum, p) => sum + p.total, 0);
      return { entries, total };
    },
  });
}

export function useReadingManifest(itemId: string) {
  return useQuery({
    queryKey: readingKeys.manifest(itemId),
    queryFn: ({ signal }) => readingApi.manifest(itemId, signal),
    enabled: Boolean(itemId),
  });
}

export function useReadingChapter(itemId: string, index: number, enabled: boolean) {
  return useQuery({
    queryKey: readingKeys.chapter(itemId, index),
    queryFn: ({ signal }) => readingApi.chapter(itemId, index, signal),
    enabled: enabled && Boolean(itemId) && index >= 0,
    staleTime: 5 * 60_000,
  });
}

export function useReadingInlinePdf(itemId: string, enabled: boolean) {
  return useQuery({
    queryKey: readingKeys.inline(itemId),
    queryFn: ({ signal }) => readingApi.prepareInline(itemId, signal),
    enabled: enabled && Boolean(itemId),
    staleTime: 10 * 60_000,
  });
}

export function usePrefetchComicPages(itemId: string, totalPages: number) {
  const queryClient = useQueryClient();
  const prefetch = (page: number) => {
    if (page < 0 || page >= totalPages) return;
    const path = readingImagePaths.page(itemId, page);
    void queryClient.prefetchQuery({
      queryKey: readingKeys.image(path),
      queryFn: ({ signal }) => readingApi.image(path, signal),
      staleTime: 5 * 60_000,
    });
  };
  return prefetch;
}

export function useSaveReadingProgress(itemId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: ReadingProgressUpdate) => readingApi.saveProgress(itemId, body),
    onSuccess: (updatedState) => {
      queryClient.setQueryData(readingKeys.manifest(itemId), (old: unknown) => {
        if (!old || typeof old !== "object" || !("entry" in old)) return old;
        const prev = old as { entry: ReadingEntry };
        return { ...prev, entry: { ...prev.entry, state: updatedState } };
      });
      void queryClient.invalidateQueries({ queryKey: ["reading", "home"] });
      void queryClient.invalidateQueries({ queryKey: readingKeys.lists() });
      void queryClient.invalidateQueries({ queryKey: ["reading", "multi-list"] });
    },
  });
}

export function useToggleReadingFavorite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ itemId, favorite }: { itemId: string; favorite: boolean }) =>
      readingApi.setFavorite(itemId, favorite),
    onSuccess: (_, { itemId }) => {
      void queryClient.invalidateQueries({ queryKey: readingKeys.manifest(itemId) });
      void queryClient.invalidateQueries({ queryKey: ["reading", "home"] });
      void queryClient.invalidateQueries({ queryKey: readingKeys.lists() });
      void queryClient.invalidateQueries({ queryKey: ["reading", "multi-list"] });
    },
  });
}

export function useCreateReadingBookmark(itemId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateBookmarkRequest) => readingApi.createBookmark(itemId, body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: readingKeys.manifest(itemId) });
    },
  });
}

export function useDeleteReadingBookmark(itemId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (bookmarkId: string) => readingApi.deleteBookmark(bookmarkId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: readingKeys.manifest(itemId) });
    },
  });
}
