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
};
