import { describe, expect, it } from "vitest";
import type { VideoEntry } from "../api";
import { findNextSeasonNumber, resolveNextEpisode } from "./next-episode";

function makeEp(id: string, season: number, episode: number): VideoEntry {
  return {
    id,
    kind: "episode",
    title: `Ep ${episode}`,
    itemId: id,
    posterItemId: null,
    overview: null,
    year: 2024,
    seasonNumber: season,
    episodeNumber: episode,
    fileCount: 1,
    libraryId: "lib-1",
    originalTitle: null,
    tagline: null,
    genres: [],
    runtimeMs: 2_700_000,
    communityRating: null,
    officialRating: null,
    premiered: null,
    backdropItemId: null,
    thumbItemId: id,
    addedAt: "2025-01-01T00:00:00Z",
    state: null,
    seriesId: "series-1",
    seriesTitle: "Harbour",
  };
}

describe("next-episode resolution", () => {
  it("picks the next episode by episodeNumber inside the current season", () => {
    const season1 = [makeEp("e3", 1, 3), makeEp("e1", 1, 1), makeEp("e2", 1, 2)];
    const target = resolveNextEpisode("e1", season1);
    expect(target?.episode.itemId).toBe("e2");
    expect(target?.seasonNumber).toBe(1);
  });

  it("advances into the next season when at the season finale", () => {
    const season1 = [makeEp("s1e1", 1, 1), makeEp("s1e2", 1, 2)];
    const season2 = [makeEp("s2e2", 2, 2), makeEp("s2e1", 2, 1)];
    const target = resolveNextEpisode("s1e2", season1, season2);
    expect(target?.episode.itemId).toBe("s2e1");
    expect(target?.seasonNumber).toBe(2);
  });

  it("returns null at the series finale", () => {
    const season2 = [makeEp("s2e1", 2, 1)];
    expect(resolveNextEpisode("s2e1", season2)).toBeNull();
  });

  it("finds the next higher season number cleanly", () => {
    const seasons = [
      { ...makeEp("sp", 0, 1), kind: "season" as const, episodeNumber: null },
      { ...makeEp("s1", 1, 1), kind: "season" as const, episodeNumber: null },
      { ...makeEp("s2", 2, 1), kind: "season" as const, episodeNumber: null },
    ];
    expect(findNextSeasonNumber(seasons, 1)).toBe(2);
    expect(findNextSeasonNumber(seasons, 2)).toBeNull();
  });
});
