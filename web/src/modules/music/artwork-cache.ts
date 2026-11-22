/**
 * Cached authenticated music artwork hook (GET /items/{id}/artwork?size=...).
 * Keeps a bounded LRU map of blob object URLs so virtualized grids and track tables scroll without
 * re-fetching or thrashing object URLs.
 */
import { useQuery } from "@tanstack/react-query";
import { musicApi } from "./api";

export type ArtworkSize = 96 | 240 | 480 | 960;

const MAX_URLS = 400;
const urlCache = new Map<string, string | null>();

function rememberUrl(key: string, blob: Blob | null): string | null {
  const existing = urlCache.get(key);
  if (existing !== undefined) {
    urlCache.delete(key);
    urlCache.set(key, existing);
    return existing;
  }
  const url = blob ? URL.createObjectURL(blob) : null;
  urlCache.set(key, url);
  if (urlCache.size > MAX_URLS) {
    const oldestKey = urlCache.keys().next().value;
    if (oldestKey !== undefined) {
      const oldestUrl = urlCache.get(oldestKey);
      urlCache.delete(oldestKey);
      if (oldestUrl) URL.revokeObjectURL(oldestUrl);
    }
  }
  return url;
}

export function useMusicArtworkUrl(itemId: string | null | undefined, size: ArtworkSize = 240, enabled = true): string | null {
  const cacheKey = itemId ? `${itemId}@${size}` : "";
  const query = useQuery({
    queryKey: ["music", "artwork", itemId ?? null, size] as const,
    queryFn: async ({ signal }) => {
      if (!itemId) return null;
      const existing = urlCache.get(cacheKey);
      if (existing !== undefined) return existing;
      const blob = await musicApi.artwork(itemId, size, signal);
      return rememberUrl(cacheKey, blob);
    },
    enabled: Boolean(itemId) && enabled,
    staleTime: 10 * 60_000,
    gcTime: 15 * 60_000,
    retry: false,
  });
  return itemId ? (query.data ?? urlCache.get(cacheKey) ?? null) : null;
}
