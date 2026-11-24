import { toast } from "@heroui/react";
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { errorText } from "@/lib/errors";
import { videoApi, type PutVideoSegmentsRequest, type VideoListQuery } from "./api";

export const videoKeys = {
  all: ["video"] as const,
  home: (libraryId?: string) => ["video", "home", libraryId ?? "all"] as const,
  list: (libraryId: string, query: VideoListQuery) => ["video", "list", libraryId, query] as const,
  genres: (libraryId: string) => ["video", "genres", libraryId] as const,
  work: (libraryId: string, workId: string) => ["video", "work", libraryId, workId] as const,
  episodes: (libraryId: string, seriesId: string, season: number) =>
    ["video", "episodes", libraryId, seriesId, season] as const,
  mediaInfo: (itemId: string) => ["video", "mediaInfo", itemId] as const,
  subtitles: (itemId: string) => ["video", "subtitles", itemId] as const,
  segments: (itemId: string) => ["video", "segments", itemId] as const,
  personal: (itemId: string) => ["video", "personal", itemId] as const,
  image: (path: string) => ["video", "image", path] as const,
};

export function useVideoHome(libraryId?: string) {
  return useQuery({
    queryKey: videoKeys.home(libraryId),
    queryFn: ({ signal }) => videoApi.home(libraryId, signal),
    staleTime: 20_000,
  });
}

export function useVideoList(libraryId: string, query: VideoListQuery, enabled = true) {
  return useInfiniteQuery({
    queryKey: videoKeys.list(libraryId, query),
    queryFn: ({ pageParam, signal }) =>
      videoApi.list(libraryId, { ...query, cursor: pageParam ?? undefined, limit: query.limit ?? 60 }, signal),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor,
    enabled: Boolean(libraryId) && enabled,
    staleTime: 20_000,
  });
}

export function useVideoGenres(libraryId: string, enabled = true) {
  return useQuery({
    queryKey: videoKeys.genres(libraryId),
    queryFn: ({ signal }) => videoApi.genres(libraryId, signal),
    enabled: Boolean(libraryId) && enabled,
    staleTime: 60_000,
  });
}

export function useVideoWork(libraryId: string, workId: string, enabled = true) {
  return useQuery({
    queryKey: videoKeys.work(libraryId, workId),
    queryFn: ({ signal }) => videoApi.work(libraryId, workId, signal),
    enabled: Boolean(libraryId && workId) && enabled,
    staleTime: 20_000,
  });
}

export function useVideoEpisodes(libraryId: string, seriesId: string, season: number | null, enabled = true) {
  return useQuery({
    queryKey: videoKeys.episodes(libraryId, seriesId, season ?? -1),
    queryFn: async ({ signal }) => {
      const all = [];
      let cursor: string | null = null;
      for (let guard = 0; guard < 10; guard++) {
        const page = await videoApi.list(
          libraryId,
          { series: seriesId, season: season ?? 1, cursor, limit: 200 },
          signal,
        );
        all.push(...page.entries);
        if (!page.nextCursor) break;
        cursor = page.nextCursor;
      }
      return all;
    },
    enabled: Boolean(libraryId && seriesId && season !== null) && enabled,
    staleTime: 20_000,
  });
}

export function useMediaInfo(itemId: string | null | undefined, enabled = true) {
  return useQuery({
    queryKey: videoKeys.mediaInfo(itemId ?? ""),
    queryFn: ({ signal }) => videoApi.mediaInfo(itemId!, signal),
    enabled: Boolean(itemId) && enabled,
    staleTime: 5 * 60_000,
    retry: false,
  });
}

export function useToggleWorkPlayed() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ libraryId, workId, played }: { libraryId: string; workId: string; played: boolean }) =>
      videoApi.setWorkPlayed(libraryId, workId, played),
    onSuccess: (_data, vars) => {
      void queryClient.invalidateQueries({ queryKey: videoKeys.all });
      toast.success(t(vars.played ? "videoApp.toast.markedPlayed" : "videoApp.toast.markedUnplayed"));
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });
}

export function useToggleWorkFavorite() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ libraryId, workId, favorite }: { libraryId: string; workId: string; favorite: boolean }) =>
      videoApi.setWorkFavorite(libraryId, workId, favorite),
    onSuccess: (_data, vars) => {
      void queryClient.invalidateQueries({ queryKey: videoKeys.all });
      toast.success(t(vars.favorite ? "videoApp.toast.favorited" : "videoApp.toast.unfavorited"));
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });
}

export function useVideoSegments(itemId: string | null | undefined, enabled = true) {
  return useQuery({
    queryKey: videoKeys.segments(itemId ?? ""),
    queryFn: ({ signal }) => videoApi.segments(itemId!, signal),
    enabled: Boolean(itemId) && enabled,
    staleTime: 60_000,
    retry: false,
  });
}

export function useSaveVideoSegments(itemId: string) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (body: PutVideoSegmentsRequest) => videoApi.saveSegments(itemId, body),
    onSuccess: (saved) => {
      queryClient.setQueryData(videoKeys.segments(itemId), saved);
      toast.success(t("videoApp.player.segmentsSaved"));
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });
}

export function useToggleItemPlayed() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: ({ itemId, played }: { itemId: string; played: boolean }) =>
      videoApi.setItemPlayed(itemId, played),
    onSuccess: (_data, vars) => {
      void queryClient.invalidateQueries({ queryKey: videoKeys.all });
      toast.success(t(vars.played ? "videoApp.toast.markedPlayed" : "videoApp.toast.markedUnplayed"));
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });
}
