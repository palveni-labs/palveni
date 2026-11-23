import { useQuery } from "@tanstack/react-query";
import { photosApi, type PhotoThumbSize } from "./api";

const MAX_URLS = 450;
const urlCache = new Map<string, string | null>();

function rememberPhotoUrl(key: string, blob: Blob | null): string | null {
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

export function usePhotoUrl(itemId: string | null | undefined, size: PhotoThumbSize = 480, enabled = true): string | null {
  const cacheKey = itemId ? `${itemId}@${size}` : "";
  const query = useQuery({
    queryKey: ["photos", "blob", itemId ?? null, size] as const,
    queryFn: async ({ signal }) => {
      if (!itemId) return null;
      const existing = urlCache.get(cacheKey);
      if (existing !== undefined) return existing;
      const blob = await photosApi.photoBlob(itemId, size, signal);
      return rememberPhotoUrl(cacheKey, blob);
    },
    enabled: Boolean(itemId) && enabled,
    staleTime: 10 * 60_000,
    gcTime: 15 * 60_000,
    retry: false,
  });
  return itemId ? (query.data ?? urlCache.get(cacheKey) ?? null) : null;
}
