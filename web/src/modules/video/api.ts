/**
 * Video app endpoints, backed by the server's generated OpenAPI schemas.
 *
 * Nullable fields are handled defensively so the UI treats a missing value as "unknown".
 */
import { API_BASE, request, requestBlob } from "@/api/client";
import type { components, operations } from "@/api/generated";

type Schemas = components["schemas"];
export type ItemId = Schemas["ItemId"];
export type LibraryId = Schemas["LibraryId"];
export type MediaState = Schemas["MediaState"];
export type SubtitleList = Schemas["SubtitleList"];
export type SubtitleTrack = Schemas["SubtitleTrack"];

export type VideoKind = Schemas["VideoKind"];
export type VideoUserState = Schemas["VideoUserState"];
export type VideoEntry = Schemas["VideoEntry"];
export type VideoPerson = Schemas["VideoPerson"];
export type VideoWorkDetail = Schemas["VideoWorkDetail"];
export type VideoPage = Schemas["VideoPage"];
export type VideoHome = Schemas["VideoHome"];
export type VideoGenre = Schemas["VideoGenre"];
export type VideoOrder = NonNullable<NonNullable<operations["browseVideos"]["parameters"]["query"]>["order"]>;

export interface VideoListQuery {
  kind?: "movie" | "series";
  series?: string;
  season?: number;
  q?: string;
  genre?: string;
  yearFrom?: number;
  yearTo?: number;
  played?: boolean;
  favorite?: boolean;
  order?: VideoOrder;
  dir?: "asc" | "desc";
  seed?: number;
  startsWith?: string;
  cursor?: string | null;
  limit?: number;
}

export type HdrKind = Schemas["HdrFormat"];
export type MediaInfo = Schemas["MediaInfo"];
export type ClientCapabilities = Schemas["ClientProfile"];
export type PlaybackRequest = Schemas["VideoPlaybackRequest"];
export type PlaybackSession = Schemas["VideoPlaybackResponse"];

const enc = encodeURIComponent;
const lib = (id: string) => `/libraries/${enc(id)}/videos`;

/** Image paths, relative to the API base. */
export const imagePaths = {
  poster: (itemId: string, level: "work" | "season" | "episode" = "work", size?: number) =>
    `/items/${enc(itemId)}/poster?level=${level}${size ? `&size=${size}` : ""}`,
  backdrop: (itemId: string, size: number) => `/items/${enc(itemId)}/backdrop?size=${size}`,
  thumb: (itemId: string, size: number) => `/items/${enc(itemId)}/thumb?size=${size}`,
  /** People images come as server-relative URLs; strip the API base the client adds back. */
  person: (imageUrl: string, size: number) => {
    const path = imageUrl.startsWith(API_BASE) ? imageUrl.slice(API_BASE.length) : imageUrl;
    return `${path}${path.includes("?") ? "&" : "?"}size=${size}`;
  },
};

export const videoApi = {
  list: (libraryId: string, query: VideoListQuery, signal?: AbortSignal) =>
    request<VideoPage>(lib(libraryId), { query: { ...query }, signal }),
  work: (libraryId: string, workId: string, signal?: AbortSignal) =>
    request<VideoWorkDetail>(`${lib(libraryId)}/works/${enc(workId)}`, { signal }),
  genres: (libraryId: string, signal?: AbortSignal) =>
    request<{ genres: VideoGenre[] }>(`${lib(libraryId)}/genres`, { signal }),
  home: (libraryId?: string, signal?: AbortSignal) =>
    request<VideoHome>("/videos/home", { query: { libraryId }, signal }),
  setWorkPlayed: (libraryId: string, workId: string, played: boolean) =>
    request<void>(`${lib(libraryId)}/works/${enc(workId)}/played`, { method: played ? "PUT" : "DELETE" }),
  setWorkFavorite: (libraryId: string, workId: string, favorite: boolean) =>
    request<void>(`${lib(libraryId)}/works/${enc(workId)}/favorite`, { method: favorite ? "PUT" : "DELETE" }),
  setItemPlayed: (itemId: string, played: boolean) =>
    request<void>(`/me/items/${enc(itemId)}/played`, { method: played ? "PUT" : "DELETE" }),
  mediaInfo: (itemId: string, signal?: AbortSignal) => request<MediaInfo>(`/items/${enc(itemId)}/media-info`, { signal }),
  startPlayback: (itemId: string, body: PlaybackRequest, signal?: AbortSignal) =>
    request<PlaybackSession>(`/items/${enc(itemId)}/video-playback`, { method: "POST", body, signal }),
  endSession: (sessionId: string) => request<void>(`/video-sessions/${enc(sessionId)}`, { method: "DELETE" }),
  personal: (itemId: string, signal?: AbortSignal) => request<MediaState>(`/me/items/${enc(itemId)}`, { signal }),
  reportProgress: (itemId: string, body: Schemas["PlaybackUpdate"]) =>
    request<MediaState>(`/me/items/${enc(itemId)}/progress`, { method: "PUT", body }),
  subtitles: (itemId: string, signal?: AbortSignal) => request<SubtitleList>(`/items/${enc(itemId)}/subtitles`, { signal }),
  subtitle: (itemId: string, track: string, signal?: AbortSignal) =>
    requestBlob(`/items/${enc(itemId)}/subtitles/${enc(track)}`, { signal }),
  image: (path: string, signal?: AbortSignal) => requestBlob(path, { signal }),
};
