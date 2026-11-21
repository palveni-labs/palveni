import { infiniteQueryOptions, queryOptions, type QueryClient } from "@tanstack/react-query";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";
import type { Item } from "../types";

export const ITEM_PAGE_SIZE = 100;

export const itemsInfiniteQuery = (libraryId: string, parentId: string | null) =>
  infiniteQueryOptions({
    queryKey: queryKeys.items.list(libraryId, parentId),
    queryFn: ({ pageParam, signal }) =>
      api.items.list({ libraryId, parentId, limit: ITEM_PAGE_SIZE, cursor: pageParam }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor,
    staleTime: 30_000,
  });

export const itemQuery = (id: string) =>
  queryOptions({
    queryKey: queryKeys.items.detail(id),
    queryFn: ({ signal }) => api.items.get(id, signal),
    staleTime: 60_000,
  });

/** Folder chain from the library root down to `id` (inclusive), for breadcrumbs. */
export const itemAncestorsQuery = (queryClient: QueryClient, id: string) =>
  queryOptions({
    queryKey: queryKeys.items.ancestors(id),
    queryFn: async () => {
      const chain: Item[] = [];
      const seen = new Set<string>();
      let next: string | null = id;
      while (next && !seen.has(next) && chain.length < 64) {
        seen.add(next);
        const item: Item = await queryClient.fetchQuery(itemQuery(next));
        chain.unshift(item);
        next = item.parentId;
      }
      return chain;
    },
    staleTime: 60_000,
  });
