import { request, requestBlob } from "@/api/client";
import type { components } from "@/api/generated";

type Schemas = components["schemas"];

export type PhotoTimelineItem = Schemas["PhotoTimelineItem"];
export type PhotoMonthBucket = Schemas["PhotoMonthBucket"];
export type PhotoTimelinePage = Schemas["PhotoTimelinePage"];
export type PhotoPlaceCluster = Schemas["PhotoPlaceCluster"];
export type PhotoCameraBucket = Schemas["PhotoCameraBucket"];
export type PhotoMemories = Schemas["PhotoMemories"];
export type PhotoAlbumRef = Schemas["PhotoAlbumRef"];
export type PhotoInfo = Schemas["PhotoInfo"];
export type PhotoAlbumSummary = Schemas["PhotoAlbumSummary"];
export type PhotoAlbumList = Schemas["PhotoAlbumList"];
export type CreatePhotoAlbumRequest = Schemas["CreatePhotoAlbumRequest"];
export type UpdatePhotoAlbumRequest = Schemas["UpdatePhotoAlbumRequest"];
export type PhotoUploadRequest = Schemas["PhotoUploadRequest"];
export type MediaState = Schemas["MediaState"];

export interface PhotoQueryFilter {
  year: number | null;
  month: number | null;
  season: string | null;
  timeOfDay: string | null;
  color: string | null;
  orientation: string | null;
  cameraBrand: string | null;
  geotagged: boolean | null;
  keywords: string[];
}

export interface PhotoAnalyzeResponse {
  analyzedCount: number;
  totalTagged: number;
  remaining: number;
}

export interface PhotoPersonRef {
  id: string;
  name: string;
}

export interface PhotoItemPeopleResponse {
  people: PhotoPersonRef[];
}

export interface PhotoPersonSummary {
  id: string;
  name: string;
  coverItemId: string | null;
  previewItemIds: string[];
  photoCount: number;
  createdAt: string;
}

export interface PhotoPersonList {
  people: PhotoPersonSummary[];
}

export interface CreatePhotoPersonRequest {
  name: string;
  coverItemId?: string;
}

export interface UpdatePhotoPersonRequest {
  name?: string;
  coverItemId?: string;
}

export interface PhotoSearchHit extends PhotoTimelineItem {
  dominantColor: string | null;
  dominantHex: string | null;
  timeOfDay: string | null;
  season: string | null;
  sceneTags: string[];
  people: PhotoPersonRef[];
}

export interface PhotoSearchResponse {
  query: string;
  parsedFilters: PhotoQueryFilter;
  items: PhotoSearchHit[];
}

export type PhotoDuplicateKind = "exact" | "similar";

export interface PhotoDuplicateItem extends PhotoTimelineItem {
  fileName: string;
  sizeBytes: number | null;
  recommendedKeep: boolean;
  distance: number;
}

export interface PhotoDuplicateCluster {
  clusterId: string;
  kind: PhotoDuplicateKind;
  keepItemId: string;
  reclaimableBytes: number;
  maxDistance: number;
  items: PhotoDuplicateItem[];
}

export interface PhotoDuplicatesResponse {
  threshold: number;
  totalClusters: number;
  totalDuplicatePhotos: number;
  totalReclaimableBytes: number;
  clusters: PhotoDuplicateCluster[];
}

export type PhotoThumbSize = 96 | 240 | 480 | 960 | 1920;

export interface PhotoTimelineFilter {
  libraryId?: string;
  albumId?: string;
  favorite?: boolean;
  hasLocation?: boolean;
  camera?: string;
  month?: string;
  place?: string;
  q?: string;
  cursor?: string;
  limit?: number;
}

export const photosApi = {
  timeline: (filter: PhotoTimelineFilter = {}, signal?: AbortSignal) =>
    request<PhotoTimelinePage>("/photos/timeline", {
      query: {
        libraryId: filter.libraryId,
        albumId: filter.albumId,
        favorite: filter.favorite,
        hasLocation: filter.hasLocation,
        camera: filter.camera,
        month: filter.month,
        place: filter.place,
        q: filter.q,
        cursor: filter.cursor,
        limit: filter.limit,
      },
      signal,
    }),

  memories: (signal?: AbortSignal) => request<PhotoMemories>("/photos/memories", { signal }),

  info: (itemId: string, signal?: AbortSignal) =>
    request<PhotoInfo>(`/items/${encodeURIComponent(itemId)}/photo-info`, { signal }),

  photoBlob: (itemId: string, size: PhotoThumbSize = 480, signal?: AbortSignal) =>
    requestBlob(`/items/${encodeURIComponent(itemId)}/photo?size=${size}`, { signal }),

  originalBlob: (itemId: string, signal?: AbortSignal) =>
    requestBlob(`/items/${encodeURIComponent(itemId)}/photo?original=true`, { signal }),

  listAlbums: (signal?: AbortSignal) => request<PhotoAlbumList>("/me/photo-albums", { signal }),

  getAlbum: (albumId: string, signal?: AbortSignal) =>
    request<PhotoAlbumSummary>(`/me/photo-albums/${encodeURIComponent(albumId)}`, { signal }),

  createAlbum: (body: CreatePhotoAlbumRequest) =>
    request<PhotoAlbumSummary>("/me/photo-albums", { method: "POST", body }),

  updateAlbum: (albumId: string, body: UpdatePhotoAlbumRequest) =>
    request<PhotoAlbumSummary>(`/me/photo-albums/${encodeURIComponent(albumId)}`, { method: "PATCH", body }),

  deleteAlbum: (albumId: string) =>
    request<void>(`/me/photo-albums/${encodeURIComponent(albumId)}`, { method: "DELETE" }),

  addAlbumItems: (albumId: string, itemIds: string[]) =>
    request<PhotoAlbumSummary>(`/me/photo-albums/${encodeURIComponent(albumId)}/items`, {
      method: "POST",
      body: { itemIds },
    }),

  removeAlbumItems: (albumId: string, itemIds: string[]) =>
    request<PhotoAlbumSummary>(`/me/photo-albums/${encodeURIComponent(albumId)}/items`, {
      method: "DELETE",
      body: { itemIds },
    }),

  setFavorite: (itemId: string, favorite: boolean) =>
    request<MediaState>(`/me/items/${encodeURIComponent(itemId)}/favorite`, {
      method: "PUT",
      body: { favorite },
    }),

  upload: (libraryId: string, body: PhotoUploadRequest) =>
    request<PhotoInfo>(`/libraries/${encodeURIComponent(libraryId)}/photos/upload`, {
      method: "POST",
      body,
    }),

  analyze: () => request<PhotoAnalyzeResponse>("/photos/analyze", { method: "POST" }),

  search: (params: { q?: string; personId?: string; limit?: number }, signal?: AbortSignal) =>
    request<PhotoSearchResponse>("/photos/search", {
      query: {
        q: params.q,
        personId: params.personId,
        limit: params.limit,
      },
      signal,
    }),

  duplicates: (threshold?: number, signal?: AbortSignal) =>
    request<PhotoDuplicatesResponse>("/photos/duplicates", {
      query: { threshold },
      signal,
    }),

  listPeople: (signal?: AbortSignal) => request<PhotoPersonList>("/me/photo-people", { signal }),

  createPerson: (body: CreatePhotoPersonRequest) =>
    request<PhotoPersonSummary>("/me/photo-people", { method: "POST", body }),

  updatePerson: (personId: string, body: UpdatePhotoPersonRequest) =>
    request<PhotoPersonSummary>(`/me/photo-people/${encodeURIComponent(personId)}`, {
      method: "PATCH",
      body,
    }),

  deletePerson: (personId: string) =>
    request<void>(`/me/photo-people/${encodeURIComponent(personId)}`, { method: "DELETE" }),

  listItemPeople: (itemId: string, signal?: AbortSignal) =>
    request<PhotoItemPeopleResponse>(`/items/${encodeURIComponent(itemId)}/photo-people`, { signal }),

  tagPerson: (itemId: string, personId: string, bbox?: Record<string, unknown>) =>
    request<PhotoPersonSummary>(
      `/items/${encodeURIComponent(itemId)}/photo-people/${encodeURIComponent(personId)}`,
      {
        method: "POST",
        body: bbox ? { bbox } : undefined,
      },
    ),

  untagPerson: (itemId: string, personId: string) =>
    request<void>(
      `/items/${encodeURIComponent(itemId)}/photo-people/${encodeURIComponent(personId)}`,
      { method: "DELETE" },
    ),
};
