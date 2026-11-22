/**
 * Shared TanStack Query keys, hooks and mutations for the music app and global player.
 * Every state mutation syncs both the Query cache and `playerStore` immediately.
 */
import { toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { isApiError } from "@/api/errors";
import { errorText } from "@/lib/errors";
import { playerStore } from "@/player/store";
import {
  lookupTracks,
  musicApi,
  newPlaylistId,
  type AlbumQuery,
  type ArtistQuery,
  type MusicTrack,
  type Playlist,
  type TrackQuery,
} from "./api";

export const musicKeys = {
  all: ["music"] as const,
  albums: (libraryId: string, params?: AlbumQuery) => ["music", "albums", libraryId, params ?? {}] as const,
  album: (libraryId: string, albumId: string) => ["music", "album", libraryId, albumId] as const,
  artists: (libraryId: string, params?: ArtistQuery) => ["music", "artists", libraryId, params ?? {}] as const,
  artist: (libraryId: string, artistId: string) => ["music", "artist", libraryId, artistId] as const,
  genres: (libraryId: string) => ["music", "genres", libraryId] as const,
  tracks: (libraryId: string, params?: TrackQuery) => ["music", "tracks", libraryId, params ?? {}] as const,
  lyrics: (trackId: string) => ["music", "lyrics", trackId] as const,
  playlists: ["music", "playlists"] as const,
  playlist: (playlistId: string) => ["music", "playlist", playlistId] as const,
  playlistTracks: (playlistId: string, version: number, idsKey: string) => ["music", "playlist-tracks", playlistId, version, idsKey] as const,
  search: (scope: string, q: string, kind: string) => ["music", "search", scope, q, kind] as const,
};

export function useTrackLyrics(trackId: string | null | undefined) {
  return useQuery({
    queryKey: musicKeys.lyrics(trackId ?? ""),
    queryFn: async ({ signal }) => {
      try {
        const res = await musicApi.lyrics(trackId!, signal);
        return res.lyrics ?? null;
      } catch (error) {
        if (isApiError(error) && error.status === 404) return null;
        throw error;
      }
    },
    enabled: Boolean(trackId),
    staleTime: 10 * 60_000,
    retry: false,
  });
}

export function usePlaylists(enabled = true) {
  return useQuery({
    queryKey: musicKeys.playlists,
    queryFn: ({ signal }) => musicApi.playlists.list(null, signal),
    enabled,
    staleTime: 30_000,
  });
}

export function usePlaylist(playlistId: string | null | undefined) {
  return useQuery({
    queryKey: musicKeys.playlist(playlistId ?? ""),
    queryFn: ({ signal }) => musicApi.playlists.get(playlistId!, signal),
    enabled: Boolean(playlistId),
  });
}

/** Hydrates full `MusicTrack` objects for a playlist (using `POST /music/tracks/lookup`). */
export function usePlaylistHydratedTracks(playlist: Playlist | undefined) {
  const ids = playlist?.trackIds ?? [];
  const idsKey = ids.join(",");
  return useQuery({
    queryKey: musicKeys.playlistTracks(playlist?.id ?? "", playlist?.version ?? 0, idsKey),
    queryFn: async ({ signal }): Promise<MusicTrack[]> => {
      if (ids.length === 0) return [];
      return lookupTracks(ids, signal);
    },
    enabled: Boolean(playlist),
    staleTime: 60_000,
  });
}

export function useToggleTrackFavorite() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ trackId, favorite }: { trackId: string; favorite: boolean }) => musicApi.favorite(trackId, favorite),
    onMutate: ({ trackId, favorite }) => {
      playerStore.patchTrackState(trackId, { favorite });
    },
    onSuccess: (state, { trackId }) => {
      playerStore.patchTrackState(trackId, {
        favorite: state.favorite,
        stars: state.stars,
        playCount: state.playCount,
        lastPlayedAt: state.lastPlayedAt,
      });
      void queryClient.invalidateQueries({ queryKey: musicKeys.all });
    },
    onError: (error, { trackId, favorite }) => {
      playerStore.patchTrackState(trackId, { favorite: !favorite });
      toast.danger(errorText(t, error));
    },
  });
}

export function useRateTrack() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ trackId, stars }: { trackId: string; stars: number }) => musicApi.rate(trackId, stars),
    onMutate: ({ trackId, stars }) => {
      playerStore.patchTrackState(trackId, { stars });
    },
    onSuccess: (state, { trackId }) => {
      playerStore.patchTrackState(trackId, {
        favorite: state.favorite,
        stars: state.stars,
        playCount: state.playCount,
        lastPlayedAt: state.lastPlayedAt,
      });
      void queryClient.invalidateQueries({ queryKey: musicKeys.all });
    },
    onError: (error) => {
      toast.danger(errorText(t, error));
    },
  });
}

export function useToggleAlbumFavorite() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ libraryId, albumId, favorite }: { libraryId: string; albumId: string; favorite: boolean }) =>
      musicApi.favoriteAlbum(libraryId, albumId, favorite),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: musicKeys.all });
    },
    onError: (error) => {
      toast.danger(errorText(t, error));
    },
  });
}

export function useToggleArtistFavorite() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ libraryId, artistId, favorite }: { libraryId: string; artistId: string; favorite: boolean }) =>
      musicApi.favoriteArtist(libraryId, artistId, favorite),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: musicKeys.all });
    },
    onError: (error) => {
      toast.danger(errorText(t, error));
    },
  });
}

export function useCreatePlaylist() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: async ({ name, trackIds = [] }: { name: string; trackIds?: string[] }) => {
      const id = newPlaylistId();
      return musicApi.playlists.save(id, { name: name.trim(), trackIds, expectedVersion: 0 });
    },
    onSuccess: (playlist) => {
      void queryClient.invalidateQueries({ queryKey: musicKeys.playlists });
      queryClient.setQueryData(musicKeys.playlist(playlist.id), playlist);
      toast.success(t("musicApp.playlists.created", { name: playlist.name }));
    },
    onError: (error) => {
      toast.danger(errorText(t, error));
    },
  });
}

export function useSavePlaylist() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ id, name, trackIds, expectedVersion }: { id: string; name: string; trackIds: string[]; expectedVersion: number }) =>
      musicApi.playlists.save(id, { name: name.trim(), trackIds, expectedVersion }),
    onSuccess: (playlist) => {
      queryClient.setQueryData(musicKeys.playlist(playlist.id), playlist);
      void queryClient.invalidateQueries({ queryKey: musicKeys.playlists });
    },
    onError: (error) => {
      if (isApiError(error) && error.status === 409) {
        toast.danger(t("musicApp.playlists.conflict"));
      } else {
        toast.danger(errorText(t, error));
      }
    },
  });
}

export function useAddTracksToPlaylist() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: async ({ playlistId, trackIds }: { playlistId: string; trackIds: string[] }) => {
      const current = await musicApi.playlists.get(playlistId);
      const merged = [...current.trackIds, ...trackIds];
      return musicApi.playlists.save(playlistId, { name: current.name, trackIds: merged, expectedVersion: current.version });
    },
    onSuccess: (playlist, { trackIds }) => {
      queryClient.setQueryData(musicKeys.playlist(playlist.id), playlist);
      void queryClient.invalidateQueries({ queryKey: musicKeys.playlists });
      toast.success(
        trackIds.length > 1
          ? t("musicApp.playlists.addedCount", { count: trackIds.length, name: playlist.name })
          : t("musicApp.playlists.addedToPlaylist", { name: playlist.name }),
      );
    },
    onError: (error) => {
      if (isApiError(error) && error.status === 409) {
        toast.danger(t("musicApp.playlists.conflict"));
      } else {
        toast.danger(errorText(t, error));
      }
    },
  });
}

export function useDeletePlaylist() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ id, version }: { id: string; version: number }) => musicApi.playlists.remove(id, version),
    onSuccess: (_, { id }) => {
      queryClient.removeQueries({ queryKey: musicKeys.playlist(id) });
      void queryClient.invalidateQueries({ queryKey: musicKeys.playlists });
      toast.success(t("musicApp.playlists.deleted"));
    },
    onError: (error) => {
      toast.danger(errorText(t, error));
    },
  });
}
