import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  photosApi,
  type CreatePhotoAlbumRequest,
  type CreatePhotoPersonRequest,
  type PhotoTimelineFilter,
  type PhotoUploadRequest,
  type UpdatePhotoAlbumRequest,
  type UpdatePhotoPersonRequest,
} from "./api";

export const photoKeys = {
  all: ["photos"] as const,
  timeline: (filter: Omit<PhotoTimelineFilter, "cursor">) => ["photos", "timeline", filter] as const,
  memories: () => ["photos", "memories"] as const,
  info: (itemId: string) => ["photos", "info", itemId] as const,
  albums: () => ["photos", "albums"] as const,
  album: (albumId: string) => ["photos", "album", albumId] as const,
  search: (q: string, personId?: string) => ["photos", "search", q, personId ?? ""] as const,
  duplicates: (threshold: number) => ["photos", "duplicates", threshold] as const,
  people: () => ["photos", "people"] as const,
  itemPeople: (itemId: string) => ["photos", "item-people", itemId] as const,
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

export function useAnalyzePhotos() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => photosApi.analyze(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["photos", "search"] });
      void qc.invalidateQueries({ queryKey: ["photos", "duplicates"] });
    },
  });
}

export function usePhotoSearch(
  params: { q?: string; personId?: string; limit?: number },
  enabled = true,
) {
  return useQuery({
    queryKey: photoKeys.search(params.q ?? "", params.personId),
    queryFn: ({ signal }) => photosApi.search(params, signal),
    enabled,
  });
}

export function usePhotoDuplicates(threshold = 6) {
  return useQuery({
    queryKey: photoKeys.duplicates(threshold),
    queryFn: ({ signal }) => photosApi.duplicates(threshold, signal),
  });
}

export function usePhotoPeople(enabled = true) {
  return useQuery({
    queryKey: photoKeys.people(),
    queryFn: ({ signal }) => photosApi.listPeople(signal),
    enabled,
  });
}

export function useCreatePhotoPerson() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreatePhotoPersonRequest) => photosApi.createPerson(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: photoKeys.people() });
    },
  });
}

export function useUpdatePhotoPerson() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ personId, body }: { personId: string; body: UpdatePhotoPersonRequest }) =>
      photosApi.updatePerson(personId, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: photoKeys.people() });
      void qc.invalidateQueries({ queryKey: ["photos", "item-people"] });
      void qc.invalidateQueries({ queryKey: ["photos", "search"] });
    },
  });
}

export function useDeletePhotoPerson() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (personId: string) => photosApi.deletePerson(personId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: photoKeys.people() });
      void qc.invalidateQueries({ queryKey: ["photos", "item-people"] });
      void qc.invalidateQueries({ queryKey: ["photos", "search"] });
    },
  });
}

export function usePhotoItemPeople(itemId: string | null | undefined, enabled = true) {
  return useQuery({
    queryKey: photoKeys.itemPeople(itemId ?? ""),
    queryFn: ({ signal }) => photosApi.listItemPeople(itemId ?? "", signal),
    enabled: Boolean(itemId) && enabled,
  });
}

export function useTagPhotoPerson() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ itemId, personId }: { itemId: string; personId: string }) =>
      photosApi.tagPerson(itemId, personId),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: photoKeys.people() });
      void qc.invalidateQueries({ queryKey: photoKeys.itemPeople(vars.itemId) });
      void qc.invalidateQueries({ queryKey: ["photos", "search"] });
    },
  });
}

export function useUntagPhotoPerson() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ itemId, personId }: { itemId: string; personId: string }) =>
      photosApi.untagPerson(itemId, personId),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: photoKeys.people() });
      void qc.invalidateQueries({ queryKey: photoKeys.itemPeople(vars.itemId) });
      void qc.invalidateQueries({ queryKey: ["photos", "search"] });
    },
  });
}
