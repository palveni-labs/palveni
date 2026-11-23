import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MusicTrack } from "@/modules/music/api";
import { activeLrcIndex, parseLrc } from "./lrc";
import {
  computeLoudnessScale,
  MEDIA_EXCLUSIVE_EVENT,
  MusicPlayerStore,
  PLAYER_STORAGE_KEY,
  shouldReportPlayCount,
} from "./store";

function makeTrack(id: string, overrides: Partial<MusicTrack> = {}): MusicTrack {
  return {
    id,
    libraryId: "lib-1",
    title: `Track ${id}`,
    artist: "Artist A",
    albumArtist: "Artist A",
    album: "Album A",
    albumId: "al-1",
    artistId: "ar-1",
    trackNumber: 1,
    discNumber: 1,
    year: 2024,
    genres: ["Pop"],
    durationMs: 200_000,
    artworkItemId: id,
    codec: "flac",
    bitrateKbps: 1000,
    sampleRateHz: 44100,
    bitDepth: 16,
    channels: 2,
    sizeBytes: 25_000_000,
    addedAt: "2026-01-01T00:00:00Z",
    state: { favorite: false, stars: 0, playCount: 0, lastPlayedAt: null },
    ...overrides,
  };
}

describe("MusicPlayerStore", () => {
  let store: MusicPlayerStore;
  const tracks = [makeTrack("t1"), makeTrack("t2"), makeTrack("t3"), makeTrack("t4")];

  beforeEach(() => {
    window.localStorage.clear();
    // Deterministic PRNG for shuffle assertions.
    let s = 42;
    const rng = () => {
      s = (s * 1664525 + 1013904223) >>> 0;
      return s / 4294967296;
    };
    store = new MusicPlayerStore({ random: rng, loadPersisted: false });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("manages queue operations: playQueue, playNext, enqueue, removeAt, moveInQueue, clearQueue", () => {
    store.playQueue([tracks[0]!, tracks[1]!], 0);
    expect(store.getSnapshot().currentTrack?.id).toBe("t1");
    expect(store.getSnapshot().playing).toBe(true);
    expect(store.getSnapshot().queue.map((t) => t.id)).toEqual(["t1", "t2"]);

    store.playNext(tracks[2]!);
    expect(store.getSnapshot().queue.map((t) => t.id)).toEqual(["t1", "t3", "t2"]);

    store.enqueue(tracks[3]!);
    expect(store.getSnapshot().queue.map((t) => t.id)).toEqual(["t1", "t3", "t2", "t4"]);

    // Move currently playing track from 0 -> 2
    store.moveInQueue(0, 2);
    expect(store.getSnapshot().queue.map((t) => t.id)).toEqual(["t3", "t2", "t1", "t4"]);
    expect(store.getSnapshot().currentIndex).toBe(2);
    expect(store.getSnapshot().currentTrack?.id).toBe("t1");

    // Move track from before current (0) to after current (3)
    store.moveInQueue(0, 3);
    expect(store.getSnapshot().queue.map((t) => t.id)).toEqual(["t2", "t1", "t4", "t3"]);
    expect(store.getSnapshot().currentIndex).toBe(1);
    expect(store.getSnapshot().currentTrack?.id).toBe("t1");

    // Remove track before current -> currentIndex decrements
    store.removeAt(0);
    expect(store.getSnapshot().queue.map((t) => t.id)).toEqual(["t1", "t4", "t3"]);
    expect(store.getSnapshot().currentIndex).toBe(0);

    // Remove current track -> advances cleanly
    store.removeAt(0);
    expect(store.getSnapshot().queue.map((t) => t.id)).toEqual(["t4", "t3"]);
    expect(store.getSnapshot().currentTrack?.id).toBe("t4");

    store.clearQueue();
    expect(store.getSnapshot().queue).toHaveLength(0);
    expect(store.getSnapshot().currentTrack).toBeNull();
    expect(store.getSnapshot().playing).toBe(false);
  });

  it("handles shuffle on/off while preserving the current track and restoring original order", () => {
    store.playQueue(tracks, 1, { shuffle: false });
    expect(store.getSnapshot().currentTrack?.id).toBe("t2");

    store.toggleShuffle();
    const shuffled = store.getSnapshot();
    expect(shuffled.shuffle).toBe(true);
    expect(shuffled.currentIndex).toBe(0);
    expect(shuffled.currentTrack?.id).toBe("t2");
    expect(shuffled.queue).toHaveLength(4);

    store.toggleShuffle();
    const restored = store.getSnapshot();
    expect(restored.shuffle).toBe(false);
    expect(restored.queue.map((t) => t.id)).toEqual(["t1", "t2", "t3", "t4"]);
    expect(restored.currentIndex).toBe(1);
    expect(restored.currentTrack?.id).toBe("t2");
  });

  it("handles repeat modes (off / all / one) and previous() 3-second rewind threshold", () => {
    store.playQueue([tracks[0]!, tracks[1]!], 0, { shuffle: false });

    // Repeat off: stops at end
    store.next(false);
    expect(store.getSnapshot().currentTrack?.id).toBe("t2");
    store.next(false);
    expect(store.getSnapshot().playing).toBe(false);

    // Repeat all: wraps around forward and backward
    store.setRepeat("all");
    store.play();
    store.next(false);
    expect(store.getSnapshot().currentTrack?.id).toBe("t1");
    expect(store.getSnapshot().playing).toBe(true);

    // previous() when currentTimeSec > 3 rewinds instead of stepping back
    store.reportTime(12, 200);
    store.previous();
    expect(store.getSnapshot().currentTrack?.id).toBe("t1");
    expect(store.getSnapshot().currentTimeSec).toBe(0);

    // previous() when currentTimeSec <= 3 wraps to last track under repeat=all
    store.previous();
    expect(store.getSnapshot().currentTrack?.id).toBe("t2");

    // Repeat one: natural end restarts same track; manual skip advances
    store.setRepeat("one");
    const seqBefore = store.getSnapshot().playInstanceId;
    store.next(false);
    expect(store.getSnapshot().currentTrack?.id).toBe("t2");
    expect(store.getSnapshot().playInstanceId).toBeGreaterThan(seqBefore);

    store.next(true);
    expect(store.getSnapshot().currentTrack?.id).toBe("t2"); // at end of queue with repeat=one, stays or stops
  });

  it("persists queue, position and settings to localStorage and hydrates paused", () => {
    store.playQueue(tracks, 2, { shuffle: false });
    store.setVolume(0.6);
    store.setRepeat("all");
    store.setQuality("320");
    store.setLoudnessEnabled(false);
    store.reportTime(73.8, 200);
    store.persist(true);

    const raw = window.localStorage.getItem(PLAYER_STORAGE_KEY);
    expect(raw).not.toBeNull();

    const restored = new MusicPlayerStore({ loadPersisted: true });
    expect(restored.getSnapshot().volume).toBe(0.6);
    expect(restored.getSnapshot().repeat).toBe("all");
    expect(restored.getSnapshot().quality).toBe("320");
    expect(restored.getSnapshot().loudnessEnabled).toBe(false);

    const saved = restored.readPersistedState();
    expect(saved?.queueIds).toEqual(["t1", "t2", "t3", "t4"]);
    expect(saved?.currentIndex).toBe(2);
    expect(saved?.positionSec).toBe(73);

    restored.hydrateQueue(tracks, saved!.currentIndex, saved!.positionSec);
    expect(restored.getSnapshot().currentTrack?.id).toBe("t3");
    expect(restored.getSnapshot().currentTimeSec).toBe(73);
    expect(restored.getSnapshot().playing).toBe(false);
    expect(restored.getSnapshot().status).toBe("paused");
  });

  it("evaluates the play-count completion rule (>50% or 4 minutes)", () => {
    // 3-minute track (180s) -> 50% = 90s
    expect(shouldReportPlayCount(0, 180)).toBe(false);
    expect(shouldReportPlayCount(89.5, 180)).toBe(false);
    expect(shouldReportPlayCount(90, 180)).toBe(true);

    // 10-minute track (600s) -> capped at 4 minutes (240s)
    expect(shouldReportPlayCount(239, 600)).toBe(false);
    expect(shouldReportPlayCount(240, 600)).toBe(true);

    // Unknown duration (0) -> requires 240s
    expect(shouldReportPlayCount(200, 0)).toBe(false);
    expect(shouldReportPlayCount(240, 0)).toBe(true);
  });

  it("computes loudness attenuation safely with true-peak headroom", () => {
    const loud = computeLoudnessScale(-8, -0.5);
    expect(loud.gainDb).toBe(-6);
    expect(loud.scale).toBeCloseTo(10 ** (-6 / 20), 3);

    const quiet = computeLoudnessScale(-20, -2);
    expect(quiet.gainDb).toBe(0);
    expect(quiet.scale).toBe(1);

    const unmeasured = computeLoudnessScale(null, null);
    expect(unmeasured).toEqual({ scale: 1, gainDb: 0 });
  });

  it("pauses playback on external palveni:media-exclusive events and ignores its own", () => {
    window.addEventListener(MEDIA_EXCLUSIVE_EVENT, store.handleExclusiveEvent);
    try {
      store.playQueue(tracks, 0);
      expect(store.getSnapshot().playing).toBe(true);

      // Own music event should not pause
      window.dispatchEvent(new CustomEvent(MEDIA_EXCLUSIVE_EVENT, { detail: { source: "music" } }));
      expect(store.getSnapshot().playing).toBe(true);

      // Video exclusive event must pause music immediately
      window.dispatchEvent(new CustomEvent(MEDIA_EXCLUSIVE_EVENT, { detail: { source: "video" } }));
      expect(store.getSnapshot().playing).toBe(false);
      expect(store.getSnapshot().status).toBe("paused");
    } finally {
      window.removeEventListener(MEDIA_EXCLUSIVE_EVENT, store.handleExclusiveEvent);
    }
  });

  it("persists playbackRate across reloads and supports starting at a resume offset", () => {
    expect(store.getSnapshot().playbackRate).toBe(1);

    store.playQueue(tracks, 0, { shuffle: false, startPositionSec: 84 });
    expect(store.getSnapshot().currentTimeSec).toBe(84);

    store.setPlaybackRate(1.5);
    expect(store.getSnapshot().playbackRate).toBe(1.5);
    store.persist(true);

    const restored = new MusicPlayerStore({ loadPersisted: true });
    expect(restored.getSnapshot().playbackRate).toBe(1.5);

    restored.setPlaybackRate(0.75);
    expect(restored.getSnapshot().playbackRate).toBe(0.75);
  });

  it("auto-pauses playback when timed sleep timer expires or at end of current track", () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date("2026-11-23T12:00:00Z"));
      store.playQueue([tracks[0]!, tracks[1]!], 0, { shuffle: false });
      expect(store.getSnapshot().playing).toBe(true);

      // 15-minute countdown updates remaining minutes and pauses on expiration
      store.setSleepTimer(15);
      expect(store.getSnapshot().sleepRemainingMinutes).toBe(15);
      expect(store.getSnapshot().sleepEndOfTrack).toBe(false);

      vi.advanceTimersByTime(5 * 60_000);
      expect(store.getSnapshot().sleepRemainingMinutes).toBe(10);
      expect(store.getSnapshot().playing).toBe(true);

      vi.advanceTimersByTime(10 * 60_000);
      expect(store.getSnapshot().playing).toBe(false);
      expect(store.getSnapshot().status).toBe("paused");
      expect(store.getSnapshot().sleepUntilMs).toBeNull();
      expect(store.getSnapshot().sleepRemainingMinutes).toBeNull();

      // Resume and arm "end_of_track" sleep mode (even with repeat="one")
      store.setRepeat("one");
      store.play();
      store.setSleepTimer("end_of_track");
      expect(store.getSnapshot().sleepEndOfTrack).toBe(true);

      // Natural track end pauses instead of advancing or repeating
      store.next(false);
      expect(store.getSnapshot().playing).toBe(false);
      expect(store.getSnapshot().status).toBe("paused");
      expect(store.getSnapshot().currentTrack?.id).toBe("t1");
      expect(store.getSnapshot().sleepEndOfTrack).toBe(false);
    } finally {
      store.reset(false);
      vi.useRealTimers();
    }
  });
});

describe("LRC lyrics parser", () => {
  it("parses timestamps, expands repeated lines, sorts chronologically and finds the active line", () => {
    const sample = [
      "[ti:Sample Song]",
      "[ar:Test Artist]",
      "[00:20.00][01:05.50]Repeated chorus",
      "[00:08.25]First verse line",
      "[00:14.000]Second verse line",
    ].join("\n");

    const lines = parseLrc(sample);
    expect(lines).toEqual([
      { timeMs: 8250, text: "First verse line" },
      { timeMs: 14000, text: "Second verse line" },
      { timeMs: 20000, text: "Repeated chorus" },
      { timeMs: 65500, text: "Repeated chorus" },
    ]);

    expect(activeLrcIndex(lines, 5000)).toBe(-1);
    expect(activeLrcIndex(lines, 8250)).toBe(0);
    expect(activeLrcIndex(lines, 19999)).toBe(1);
    expect(activeLrcIndex(lines, 20000)).toBe(2);
    expect(activeLrcIndex(lines, 70000)).toBe(3);
  });
});
