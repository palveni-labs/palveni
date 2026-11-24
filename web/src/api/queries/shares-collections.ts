import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { buildUrl, request } from "@/api/client";

export type ShareTargetKind = "item" | "video_work" | "music_album" | "playlist" | "photo_album" | "collection";
export type ShareMediaKind = "video" | "audio" | "photo" | "book" | "comic";

export interface ShareLinkSummary {
  id: string;
  token: string;
  sharePath: string;
  creatorId: string;
  targetKind: ShareTargetKind;
  targetId: string;
  libraryId?: string | null;
  title: string;
  hasPassword: boolean;
  allowDownload: boolean;
  stripGps: boolean;
  expiresAt?: string | null;
  maxViews?: number | null;
  viewCount: number;
  revokedAt?: string | null;
  expired: boolean;
  createdAt: string;
}

export interface ShareLinkList {
  shares: ShareLinkSummary[];
}

export interface CreateShareRequest {
  targetKind: ShareTargetKind;
  targetId: string;
  libraryId?: string | null;
  title?: string | null;
  password?: string | null;
  allowDownload?: boolean;
  stripGps?: boolean;
  expiresAt?: string | null;
  expiresInHours?: number | null;
  maxViews?: number | null;
}

export interface PublicShareItem {
  itemId: string;
  mediaKind: ShareMediaKind;
  title: string;
  subtitle?: string | null;
  durationMs?: number | null;
  width?: number | null;
  height?: number | null;
  takenAt?: string | null;
  camera?: string | null;
  latitude?: number | null;
  longitude?: number | null;
  format?: string | null;
  sizeBytes?: number | null;
  hasCover: boolean;
}

export interface PublicShareView {
  token: string;
  title: string;
  targetKind: ShareTargetKind;
  locked: boolean;
  allowDownload: boolean;
  stripGps: boolean;
  expiresAt?: string | null;
  proof?: string | null;
  items: PublicShareItem[];
}

export interface UnlockShareResponse {
  proof: string;
  expiresAt: string;
}

export type MediaCollectionEntryKind = "video" | "music_album" | "track" | "book" | "comic" | "photo";

export interface MediaCollectionSummary {
  id: string;
  ownerId: string;
  title: string;
  description?: string | null;
  coverArtworkId?: string | null;
  previewArtworkIds: string[];
  itemCount: number;
  kinds: MediaCollectionEntryKind[];
  createdAt: string;
  updatedAt: string;
}

export interface MediaCollectionList {
  collections: MediaCollectionSummary[];
}

export interface MediaCollectionEntry {
  id: string;
  collectionId: string;
  entryKind: MediaCollectionEntryKind;
  targetId: string;
  libraryId?: string | null;
  title: string;
  subtitle?: string | null;
  artworkId?: string | null;
  position: number;
  addedAt: string;
}

export interface RelatedWork {
  entryKind: MediaCollectionEntryKind;
  targetId: string;
  libraryId: string;
  title: string;
  subtitle?: string | null;
  artworkId?: string | null;
}

export interface RelatedWorkList {
  works: RelatedWork[];
}

export interface MediaCollectionDetail {
  collection: MediaCollectionSummary;
  items: MediaCollectionEntry[];
  autoRelated: RelatedWork[];
}

export interface CreateMediaCollectionRequest {
  title: string;
  description?: string | null;
  coverArtworkId?: string | null;
}

export interface UpdateMediaCollectionRequest {
  title?: string | null;
  description?: string | null;
  coverArtworkId?: string | null;
  itemOrder?: string[] | null;
}

export interface AddMediaCollectionItemRequest {
  entryKind: MediaCollectionEntryKind;
  targetId: string;
  libraryId?: string | null;
  title?: string | null;
  subtitle?: string | null;
  artworkId?: string | null;
}

export type GlobalSearchKind =
  | "video_work"
  | "music_album"
  | "track"
  | "book"
  | "comic"
  | "photo"
  | "broadcast"
  | "collection"
  | "home_device"
  | "home_scene";

export interface GlobalSearchHit {
  kind: GlobalSearchKind;
  id: string;
  libraryId?: string | null;
  sourceId?: string | null;
  title: string;
  subtitle?: string | null;
  artworkId?: string | null;
}

export interface GlobalSearchResponse {
  query: string;
  hits: GlobalSearchHit[];
}

export const shareKeys = {
  all: ["shares"] as const,
  public: (token: string, proof?: string | null) => ["public-share", token, proof ?? ""] as const,
};

export const collectionKeys = {
  all: ["collections"] as const,
  detail: (id: string) => ["collections", id] as const,
  related: (title: string, excludeKind?: string) => ["related-works", title, excludeKind ?? ""] as const,
};

export const globalSearchKeys = {
  search: (q: string, limit = 16) => ["global-search", q, limit] as const,
};

export function publicShareMediaUrl(
  token: string,
  itemId: string,
  proof: string,
  options?: { size?: number; download?: boolean },
): string {
  return buildUrl(`/public/shares/${encodeURIComponent(token)}/media/${encodeURIComponent(itemId)}`, {
    proof,
    size: options?.size,
    download: options?.download,
  });
}

export function publicShareCoverUrl(token: string, itemId: string, proof: string, size = 480): string {
  return buildUrl(`/public/shares/${encodeURIComponent(token)}/cover/${encodeURIComponent(itemId)}`, {
    proof,
    size,
  });
}

export function useShares(enabled = true) {
  return useQuery({
    queryKey: shareKeys.all,
    queryFn: ({ signal }) => request<ShareLinkList>("/shares", { signal }),
    enabled,
  });
}

export function useCreateShare() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateShareRequest) => request<ShareLinkSummary>("/shares", { method: "POST", body }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: shareKeys.all }),
  });
}

export function useRevokeShare() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => request<void>(`/shares/${encodeURIComponent(id)}`, { method: "DELETE" }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: shareKeys.all }),
  });
}

export const publicShareQuery = (token: string, proof?: string | null) =>
  queryOptions({
    queryKey: shareKeys.public(token, proof),
    queryFn: ({ signal }) =>
      request<PublicShareView>(`/public/shares/${encodeURIComponent(token)}`, {
        query: proof ? { proof } : undefined,
        auth: false,
        signal,
      }),
    retry: false,
  });

export function usePublicShare(token: string, proof?: string | null) {
  return useQuery(publicShareQuery(token, proof));
}

export function useUnlockPublicShare(token: string) {
  return useMutation({
    mutationFn: (password: string) =>
      request<UnlockShareResponse>(`/public/shares/${encodeURIComponent(token)}/unlock`, {
        method: "POST",
        body: { password },
        auth: false,
      }),
  });
}

export function useCollections(enabled = true) {
  return useQuery({
    queryKey: collectionKeys.all,
    queryFn: ({ signal }) => request<MediaCollectionList>("/collections", { signal }),
    enabled,
  });
}

export function useCollection(collectionId: string, enabled = true) {
  return useQuery({
    queryKey: collectionKeys.detail(collectionId),
    queryFn: ({ signal }) =>
      request<MediaCollectionDetail>(`/collections/${encodeURIComponent(collectionId)}`, { signal }),
    enabled: enabled && Boolean(collectionId),
  });
}

export function useCreateCollection() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateMediaCollectionRequest) =>
      request<MediaCollectionSummary>("/collections", { method: "POST", body }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: collectionKeys.all }),
  });
}

export function useUpdateCollection() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateMediaCollectionRequest }) =>
      request<MediaCollectionSummary>(`/collections/${encodeURIComponent(id)}`, { method: "PATCH", body }),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: collectionKeys.all });
      void qc.invalidateQueries({ queryKey: collectionKeys.detail(vars.id) });
    },
  });
}

export function useDeleteCollection() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => request<void>(`/collections/${encodeURIComponent(id)}`, { method: "DELETE" }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: collectionKeys.all }),
  });
}

export function useAddCollectionItem() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ collectionId, body }: { collectionId: string; body: AddMediaCollectionItemRequest }) =>
      request<MediaCollectionEntry>(`/collections/${encodeURIComponent(collectionId)}/items`, {
        method: "POST",
        body,
      }),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: collectionKeys.all });
      void qc.invalidateQueries({ queryKey: collectionKeys.detail(vars.collectionId) });
    },
  });
}

export function useRemoveCollectionItem() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ collectionId, entryId }: { collectionId: string; entryId: string }) =>
      request<void>(
        `/collections/${encodeURIComponent(collectionId)}/items/${encodeURIComponent(entryId)}`,
        { method: "DELETE" },
      ),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: collectionKeys.all });
      void qc.invalidateQueries({ queryKey: collectionKeys.detail(vars.collectionId) });
    },
  });
}

export function useRelatedWorks(
  title: string | null | undefined,
  excludeKind?: MediaCollectionEntryKind,
  enabled = true,
) {
  const clean = title?.trim() ?? "";
  return useQuery({
    queryKey: collectionKeys.related(clean, excludeKind),
    queryFn: ({ signal }) =>
      request<RelatedWorkList>("/related", {
        query: { title: clean, excludeKind, limit: 12 },
        signal,
      }),
    enabled: enabled && clean.length >= 2,
  });
}

export function useGlobalSearch(query: string, enabled = true, limit = 16) {
  const clean = query.trim();
  return useQuery({
    queryKey: globalSearchKeys.search(clean, limit),
    queryFn: ({ signal }) =>
      request<GlobalSearchResponse>("/search", {
        query: { q: clean, limit },
        signal,
      }),
    enabled: enabled && clean.length >= 1,
    staleTime: 10_000,
  });
}
