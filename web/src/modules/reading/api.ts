/**
 * Reading module API client, backed by generated OpenAPI schemas.
 */
import { API_BASE, request, requestBlob } from "@/api/client";
import type { components } from "@/api/generated";

type Schemas = components["schemas"];

export type ItemId = Schemas["ItemId"];
export type LibraryId = Schemas["LibraryId"];
export type BookmarkId = Schemas["BookmarkId"];
export type ReadingKind = Schemas["ReadingKind"];
export type ReadingSort = Schemas["ReadingSort"];
export type ReadingSortDir = Schemas["ReadingSortDir"];
export type ReadingReaderMode = Schemas["ReadingReaderMode"];
export type ReadingUserState = Schemas["ReadingUserState"];
export type ReadingEntry = Schemas["ReadingEntry"];
export type ReadingSeries = Schemas["ReadingSeries"];
export type ReadingHome = Schemas["ReadingHome"];
export type ReadingPage = Schemas["ReadingPage"];
export type ReadingChapterSummary = Schemas["ReadingChapterSummary"];
export type ReadingBookmark = Schemas["ReadingBookmark"];
export type ReadingManifest = Schemas["ReadingManifest"];
export type ReadingChapter = Schemas["ReadingChapter"];
export type ReadingInline = Schemas["ReadingInline"];
export type ReadingProgressUpdate = Schemas["ReadingProgressUpdate"];
export type CreateBookmarkRequest = Schemas["CreateBookmarkRequest"];
export type BookDownload = Schemas["BookDownload"];
export type MediaState = Schemas["MediaState"];

export interface ReadingListQuery {
  kind?: ReadingKind;
  series?: string;
  author?: string;
  format?: string;
  favorite?: boolean;
  completed?: boolean;
  q?: string;
  sort?: ReadingSort;
  dir?: ReadingSortDir;
  cursor?: string | null;
  limit?: number;
}

const enc = encodeURIComponent;

export const readingImagePaths = {
  cover: (itemId: string, size?: number) =>
    `/items/${enc(itemId)}/reading-cover${size ? `?size=${size}` : ""}`,
  page: (itemId: string, page: number, size?: number) =>
    `/items/${enc(itemId)}/reading-page/${page}${size ? `?size=${size}` : ""}`,
  asset: (itemId: string, assetPath: string) =>
    `/items/${enc(itemId)}/reading-asset?path=${enc(assetPath)}`,
};

export const readingApi = {
  home: (libraryId?: string, signal?: AbortSignal) =>
    request<ReadingHome>("/reading/home", { query: { libraryId }, signal }),
  list: (libraryId: string, query: ReadingListQuery, signal?: AbortSignal) =>
    request<ReadingPage>(`/libraries/${enc(libraryId)}/reading`, { query: { ...query }, signal }),
  manifest: (itemId: string, signal?: AbortSignal) =>
    request<ReadingManifest>(`/items/${enc(itemId)}/reading-manifest`, { signal }),
  chapter: (itemId: string, index: number, signal?: AbortSignal) =>
    request<ReadingChapter>(`/items/${enc(itemId)}/reading-chapter/${index}`, { signal }),
  prepareInline: (itemId: string, signal?: AbortSignal) =>
    request<ReadingInline>(`/items/${enc(itemId)}/reading-inline`, { method: "POST", signal }),
  prepareBookDownload: (itemId: string) =>
    request<BookDownload>(`/items/${enc(itemId)}/book-download`, { method: "POST" }),
  saveProgress: (itemId: string, body: ReadingProgressUpdate) =>
    request<ReadingUserState>(`/me/items/${enc(itemId)}/reading-progress`, { method: "PUT", body }),
  setFavorite: (itemId: string, favorite: boolean) =>
    request<MediaState>(`/me/items/${enc(itemId)}/favorite`, { method: "PUT", body: { favorite } }),
  createBookmark: (itemId: string, body: CreateBookmarkRequest) =>
    request<ReadingBookmark>(`/me/items/${enc(itemId)}/bookmarks`, { method: "POST", body }),
  deleteBookmark: (bookmarkId: string) =>
    request<void>(`/me/bookmarks/${enc(bookmarkId)}`, { method: "DELETE" }),
  image: (path: string, signal?: AbortSignal) => requestBlob(path, { signal }),
};

export async function downloadOriginalReadingFile(itemId: string) {
  const prepared = await readingApi.prepareBookDownload(itemId);
  const url = new URL(prepared.url, window.location.origin);
  if (url.origin !== window.location.origin || !url.pathname.startsWith(API_BASE + "/")) {
    throw new Error("Invalid download URL");
  }
  const link = document.createElement("a");
  link.href = url.href;
  link.download = "";
  link.referrerPolicy = "no-referrer";
  document.body.append(link);
  link.click();
  link.remove();
}
