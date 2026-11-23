import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  photosApi,
  type CreatePhotoAlbumRequest,
  type PhotoTimelineFilter,
  type PhotoUploadRequest,
  type UpdatePhotoAlbumRequest,
} from "./api";

export const photoKeys = {
  all: ["photos"] as const,
  timeline: (filter: Omit<PhotoTimelineFilter, "cursor">) => ["photos", "timeline", filter] as const,
  memories: () => ["photos", "memories"] as const,
  info: (itemId: string) => ["photos", "info", itemId] as const,
  albums: () => ["photos", "albums"] as const,
  album: (albumId: string) => ["photos", "album", albumId] as const,
};

export function usePhotoTimeline(filter: Omit<PhotoTimelineFilter, "cursor"> = {}) {
  return useInfiniteQuery({
    queryKey: photoKeys.timeline(filter),
    initialPageParam: undefined as string | undefined,
    queryFn: ({ pageParam, signal }) =>
      photosApi.timeline({ ...filter, cursor: pageParam, limit: filter.limit ?? 80 }, signal),
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
  });
}

export function usePhotoMemories() {
  return useQuery({
    queryKey: photoKeys.memories(),
    queryFn: ({ signal }) => photosApi.memories(signal),
  });
}

export function usePhotoInfo(itemId: string | null | undefined, enabled = true) {
  return useQuery({
    queryKey: photoKeys.info(itemId ?? ""),
    queryFn: ({ signal }) => photosApi.info(itemId ?? "", signal),
    enabled: Boolean(itemId) && enabled,
  });
}

export function usePhotoAlbums() {
  return useQuery({
    queryKey: photoKeys.albums(),
    queryFn: ({ signal }) => photosApi.listAlbums(signal),
  });
}

export function usePhotoAlbum(albumId: string) {
  return useQuery({
    queryKey: photoKeys.album(albumId),
    queryFn: ({ signal }) => photosApi.getAlbum(albumId, signal),
    enabled: Boolean(albumId),
  });
}

export function useTogglePhotoFavorite() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ itemId, favorite }: { itemId: string; favorite: boolean }) =>
      photosApi.setFavorite(itemId, favorite),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ["photos", "timeline"] });
      void qc.invalidateQueries({ queryKey: photoKeys.memories() });
      void qc.invalidateQueries({ queryKey: photoKeys.info(vars.itemId) });
    },
  });
}

export function useBatchPhotoFavorite() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ itemIds, favorite }: { itemIds: string[]; favorite: boolean }) => {
      await Promise.all(itemIds.map((id) => photosApi.setFavorite(id, favorite)));
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["photos", "timeline"] });
      void qc.invalidateQueries({ queryKey: photoKeys.memories() });
      void qc.invalidateQueries({ queryKey: ["photos", "info"] });
    },
  });
}

export function useCreatePhotoAlbum() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreatePhotoAlbumRequest) => photosApi.createAlbum(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: photoKeys.albums() });
    },
  });
}

export function useUpdatePhotoAlbum() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ albumId, body }: { albumId: string; body: UpdatePhotoAlbumRequest }) =>
      photosApi.updateAlbum(albumId, body),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: photoKeys.albums() });
      void qc.invalidateQueries({ queryKey: photoKeys.album(vars.albumId) });
      void qc.invalidateQueries({ queryKey: ["photos", "info"] });
    },
  });
}

export function useDeletePhotoAlbum() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (albumId: string) => photosApi.deleteAlbum(albumId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: photoKeys.albums() });
      void qc.invalidateQueries({ queryKey: ["photos", "info"] });
    },
  });
}

export function useAddPhotoAlbumItems() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ albumId, itemIds }: { albumId: string; itemIds: string[] }) =>
      photosApi.addAlbumItems(albumId, itemIds),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: photoKeys.albums() });
      void qc.invalidateQueries({ queryKey: photoKeys.album(vars.albumId) });
      void qc.invalidateQueries({ queryKey: ["photos", "timeline"] });
      void qc.invalidateQueries({ queryKey: ["photos", "info"] });
    },
  });
}

export function useRemovePhotoAlbumItems() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ albumId, itemIds }: { albumId: string; itemIds: string[] }) =>
      photosApi.removeAlbumItems(albumId, itemIds),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: photoKeys.albums() });
      void qc.invalidateQueries({ queryKey: photoKeys.album(vars.albumId) });
      void qc.invalidateQueries({ queryKey: ["photos", "timeline"] });
      void qc.invalidateQueries({ queryKey: ["photos", "info"] });
    },
  });
}

export function useUploadPhoto() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ libraryId, body }: { libraryId: string; body: PhotoUploadRequest }) =>
      photosApi.upload(libraryId, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["photos"] });
      void qc.invalidateQueries({ queryKey: ["libraries"] });
    },
  });
}
