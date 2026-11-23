/**
 * Music catalogue API, backed by the server's generated OpenAPI schemas.
 */
import { API_BASE, request, requestBlob } from "@/api/client";
import { isApiError } from "@/api/errors";
import type { components } from "@/api/generated";

type Schemas = components["schemas"];

export type ItemId = Schemas["ItemId"];
export type LibraryId = Schemas["LibraryId"];

export type MusicTrackState = Schemas["MusicTrackState"];
export type MusicTrack = Schemas["MusicTrack"];
export type MusicAlbum = Schemas["MusicAlbum"];
export type MusicArtist = Schemas["MusicArtist"];
export type MusicGenre = Schemas["MusicGenre"];

export interface Page {
  nextCursor: string | null;
  total: number;
}
export type AlbumPage = Schemas["MusicAlbumPage"];
export type ArtistPage = Schemas["MusicArtistPage"];
export type TrackPage = Schemas["MusicTrackPage"];
export type AlbumDetail = Schemas["MusicAlbumDetail"];
export type ArtistDetail = Schemas["MusicArtistDetail"];

export type Order = Schemas["MusicSortOrder"];
export type AlbumSort = Schemas["MusicAlbumSort"];
export type ArtistSort = Schemas["MusicArtistSort"];
export type TrackSort = Schemas["MusicTrackSort"];

interface ListParams<S> {
  sort?: S;
  order?: Order;
  favorite?: boolean;
  q?: string;
  seed?: number;
  cursor?: string | null;
  limit?: number;
}
export interface AlbumQuery extends ListParams<AlbumSort> {
  genre?: string;
  yearFrom?: number;
  yearTo?: number;
  artistId?: string;
}
export type ArtistQuery = ListParams<ArtistSort>;
export interface TrackQuery extends ListParams<TrackSort> {
  genre?: string;
  albumId?: string;
  artistId?: string;
}

export type MediaState = Schemas["MediaState"];
export type PlaybackInfo = Schemas["PlaybackInfo"];
export type PlaylistSummary = Schemas["PlaylistSummary"];
export type Playlist = Schemas["Playlist"];
export type PlaylistTrack = Schemas["PlaylistTrack"];
export type Lyrics = Schemas["Lyrics"];
export type LoudnessMeasurement = Schemas["LoudnessMeasurement"];
export type MusicSearchHit = Schemas["MusicSearchHit"];
export type MusicSearchKind = Schemas["MusicSearchKind"];
export type GlobalMusicSearchHit = Schemas["GlobalMusicSearchHit"];
export type GlobalMusicSearchPage = Schemas["GlobalMusicSearchPage"];
export type SpokenPage = Schemas["SpokenPage"];
export type SpokenEntry = Schemas["SpokenEntry"];
export type SpokenRecording = Schemas["SpokenRecording"];
export type SpokenChapter = Schemas["SpokenChapter"];
export type SpokenChapterList = Schemas["SpokenChapterList"];

export type TranscodeFormat = "mp3" | "aac" | "opus";

const enc = encodeURIComponent;
const lib = (libraryId: string) => `/libraries/${enc(libraryId)}/music`;

function query<S>(params: ListParams<S> & Record<string, unknown>) {
  const out: Record<string, string | number | boolean | undefined> = {};
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null || value === "") continue;
    if (key === "favorite") {
      if (value === true) out.favorite = true;
      continue;
    }
    out[key] = value as string | number;
  }
  return out;
}

/** 404 → null, for images and optional sub-resources. */
async function orNull<T>(promise: Promise<T>): Promise<T | null> {
  try {
    return await promise;
  } catch (error) {
    if (isApiError(error) && error.status === 404) return null;
    throw error;
  }
}

export const musicApi = {
  albums: (libraryId: string, params: AlbumQuery, signal?: AbortSignal) =>
    request<AlbumPage>(`${lib(libraryId)}/albums`, { query: query(params as AlbumQuery & Record<string, unknown>), signal }),
  album: (libraryId: string, albumId: string, signal?: AbortSignal) =>
    request<AlbumDetail>(`${lib(libraryId)}/albums/${enc(albumId)}`, { signal }),
  artists: (libraryId: string, params: ArtistQuery, signal?: AbortSignal) =>
    request<ArtistPage>(`${lib(libraryId)}/artists`, { query: query(params as ArtistQuery & Record<string, unknown>), signal }),
  artist: (libraryId: string, artistId: string, signal?: AbortSignal) =>
    request<ArtistDetail>(`${lib(libraryId)}/artists/${enc(artistId)}`, { signal }),
  genres: (libraryId: string, signal?: AbortSignal) => request<{ genres: MusicGenre[] }>(`${lib(libraryId)}/genres`, { signal }),
  tracks: (libraryId: string, params: TrackQuery, signal?: AbortSignal) =>
    request<TrackPage>(`${lib(libraryId)}/tracks`, { query: query(params as TrackQuery & Record<string, unknown>), signal }),
  /** Up to 500 ids per call; the answer keeps the request order and skips unknown ids. */
  lookup: (ids: ItemId[], signal?: AbortSignal) =>
    request<{ tracks: MusicTrack[] }>("/music/tracks/lookup", { method: "POST", body: { ids }, signal }),
  favoriteAlbum: (libraryId: string, albumId: string, favorite: boolean) =>
    request<void>(`${lib(libraryId)}/albums/${enc(albumId)}/favorite`, { method: favorite ? "PUT" : "DELETE" }),
  favoriteArtist: (libraryId: string, artistId: string, favorite: boolean) =>
    request<void>(`${lib(libraryId)}/artists/${enc(artistId)}/favorite`, { method: favorite ? "PUT" : "DELETE" }),

  /* ---- existing endpoints */
  artwork: (itemId: ItemId, size: number, signal?: AbortSignal) =>
    orNull(requestBlob(`/items/${enc(itemId)}/artwork?size=${size}`, { signal })),
  lyrics: (itemId: ItemId, signal?: AbortSignal) => request<Schemas["TrackLyrics"]>(`/items/${enc(itemId)}/lyrics`, { signal }),
  loudness: (itemId: ItemId, signal?: AbortSignal) => request<Schemas["TrackLoudness"]>(`/items/${enc(itemId)}/loudness`, { signal }),
  playback: (itemId: ItemId, options: { format?: TranscodeFormat | "original"; bitrateKbps?: number; startMs?: number } = {}, signal?: AbortSignal) =>
    request<PlaybackInfo>(`/items/${enc(itemId)}/playback`, {
      method: "POST",
      query: {
        format: options.format,
        bitrateKbps: options.format && options.format !== "original" ? options.bitrateKbps : undefined,
        startMs: options.format && options.format !== "original" && options.startMs ? Math.floor(options.startMs) : undefined,
      },
      signal,
    }),
  state: (itemId: ItemId, signal?: AbortSignal) => request<MediaState>(`/me/items/${enc(itemId)}`, { signal }),
  favorite: (itemId: ItemId, favorite: boolean) =>
    request<MediaState>(`/me/items/${enc(itemId)}/favorite`, { method: "PUT", body: { favorite } }),
  rate: (itemId: ItemId, stars: number) => request<MediaState>(`/me/items/${enc(itemId)}/rating`, { method: "PUT", body: { stars } }),
  progress: (itemId: ItemId, body: Schemas["PlaybackUpdate"]) =>
    request<MediaState>(`/me/items/${enc(itemId)}/progress`, { method: "PUT", body }),
  search: (libraryId: string, q: string, kind: MusicSearchKind, limit = 24, signal?: AbortSignal) =>
    request<Schemas["MusicSearchPage"]>(`${lib(libraryId)}/search`, { query: { q, kind, limit }, signal }),
  searchAll: (q: string, kind: MusicSearchKind, limit = 24, cursor?: string | null, signal?: AbortSignal) =>
    request<GlobalMusicSearchPage>("/music/search", { query: { q, kind, limit, cursor: cursor ?? undefined }, signal }),
  spoken: (libraryId: string, params: { collection?: string | null; cursor?: string | null; limit?: number } = {}, signal?: AbortSignal) =>
    request<SpokenPage>(`/libraries/${enc(libraryId)}/spoken`, {
      query: {
        collection: params.collection ?? undefined,
        cursor: params.cursor ?? undefined,
        limit: params.limit,
      },
      signal,
    }),
  chapters: async (itemId: ItemId, signal?: AbortSignal): Promise<SpokenChapterList> => {
    try {
      return await request<SpokenChapterList>(`/items/${enc(itemId)}/chapters`, { signal });
    } catch (error) {
      if (isApiError(error) && (error.status === 404 || error.status === 503)) {
        return { chapters: [] };
      }
      throw error;
    }
  },

  playlists: {
    list: (before?: number | null, signal?: AbortSignal) =>
      request<Schemas["PlaylistPage"]>("/me/playlists", { query: { before: before ?? undefined, limit: 200 }, signal }),
    get: (id: string, signal?: AbortSignal) => request<Playlist>(`/me/playlists/${enc(id)}`, { signal }),
    tracks: (id: string, version: number, after?: number | null, signal?: AbortSignal) =>
      request<Schemas["PlaylistTrackPage"]>(`/me/playlists/${enc(id)}/tracks`, { query: { version, after: after ?? undefined, limit: 200 }, signal }),
    save: (id: string, body: Schemas["PlaylistUpdate"]) => request<Playlist>(`/me/playlists/${enc(id)}`, { method: "PUT", body }),
    remove: (id: string, version: number) => request<void>(`/me/playlists/${enc(id)}`, { method: "DELETE", query: { version } }),
  },
};

/** Tracks for many ids, in order, in chunks of the lookup limit. */
export async function lookupTracks(ids: ItemId[], signal?: AbortSignal): Promise<MusicTrack[]> {
  const out: MusicTrack[] = [];
  for (let start = 0; start < ids.length; start += 500) {
    const { tracks } = await musicApi.lookup(ids.slice(start, start + 500), signal);
    out.push(...tracks);
  }
  return out;
}

/** Starts a browser download of the original file through a short-lived playback ticket. */
export async function downloadOriginal(itemId: ItemId) {
  const info = await musicApi.playback(itemId, { format: "original" });
  const url = new URL(info.url, window.location.origin);
  if (url.origin !== window.location.origin || !url.pathname.startsWith(API_BASE + "/")) throw new Error("Invalid download URL");
  const link = document.createElement("a");
  link.href = url.href;
  link.download = "";
  link.referrerPolicy = "no-referrer";
  document.body.append(link);
  link.click();
  link.remove();
}

/** RFC 4122 v4 id for a new playlist (the client names playlists it creates). */
export function newPlaylistId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6]! & 15) | 64;
  bytes[8] = (bytes[8]! & 63) | 128;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
