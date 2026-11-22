import type { VideoEntry } from "../api";

export interface NextEpisodeTarget {
  episode: VideoEntry;
  seasonNumber: number;
}

/**
 * Resolves the next playable episode from the current season's episode list (sorted by episodeNumber).
 * If at the end of a season and `nextSeasonEpisodes` is provided, advances to the next season's first episode.
 */
export function resolveNextEpisode(
  currentItemId: string,
  seasonEpisodes: readonly VideoEntry[],
  nextSeasonEpisodes?: readonly VideoEntry[],
): NextEpisodeTarget | null {
  const ordered = [...seasonEpisodes]
    .filter((entry) => entry.kind === "episode" && Boolean(entry.itemId))
    .sort((a, b) => (a.episodeNumber ?? 0) - (b.episodeNumber ?? 0));

  const index = ordered.findIndex((entry) => entry.itemId === currentItemId || entry.id === currentItemId);
  if (index !== -1 && index + 1 < ordered.length) {
    const next = ordered[index + 1]!;
    return { episode: next, seasonNumber: next.seasonNumber ?? 1 };
  }

  if (nextSeasonEpisodes && nextSeasonEpisodes.length > 0) {
    const nextOrdered = [...nextSeasonEpisodes]
      .filter((entry) => entry.kind === "episode" && Boolean(entry.itemId))
      .sort((a, b) => (a.episodeNumber ?? 0) - (b.episodeNumber ?? 0));
    const first = nextOrdered[0];
    if (first) {
      return { episode: first, seasonNumber: first.seasonNumber ?? 1 };
    }
  }

  return null;
}

/** Finds the next regular season number after `currentSeason` (ignoring Specials = 0 unless currently in Specials). */
export function findNextSeasonNumber(seasons: readonly VideoEntry[], currentSeason: number): number | null {
  const numbers = seasons
    .map((s) => s.seasonNumber)
    .filter((n): n is number => n !== null && n > currentSeason)
    .sort((a, b) => a - b);
  return numbers[0] ?? null;
}
