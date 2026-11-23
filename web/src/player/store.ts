/**
 * Global music player state store.
 *
 * Pure reactive store consumed via `useSyncExternalStore`, decoupled from the HTMLAudioElement
 * driver (`audio-engine.ts`) so queue management, shuffle/repeat, persistence, play-count rules,
 * loudness calculation and exclusive-media coordination are deterministic and directly testable.
 */
import { useSyncExternalStore } from "react";
import { readLocal, removeLocal, writeLocal } from "@/lib/storage";
import type { LoudnessMeasurement, MusicTrack, MusicTrackState } from "@/modules/music/api";
import { clamp } from "@/modules/music/format";

export type RepeatMode = "off" | "all" | "one";
export type AudioQuality = "original" | "320" | "192" | "128";
export type PlayerStatus = "idle" | "loading" | "playing" | "paused" | "error";
export type NowPlayingTab = "lyrics" | "queue";

export const PLAYBACK_RATES = [0.75, 1, 1.25, 1.5, 1.75, 2] as const;
export type PlaybackRate = (typeof PLAYBACK_RATES)[number];

export const SLEEP_TIMER_MINUTES = [15, 30, 45, 60] as const;
export type SleepTimerMinutes = (typeof SLEEP_TIMER_MINUTES)[number];
export type SleepTimerPreset = "off" | SleepTimerMinutes | "end_of_track";

export type PlayableTrack = MusicTrack & {
  spokenKind?: "audiobook" | "podcast";
};

export const PLAYER_STORAGE_KEY = "palveni.music.player";
export const MEDIA_EXCLUSIVE_EVENT = "palveni:media-exclusive";

export interface PersistedPlayerState {
  queueIds: string[];
  currentIndex: number;
  positionSec: number;
  volume: number;
  muted: boolean;
  shuffle: boolean;
  repeat: RepeatMode;
  quality: AudioQuality;
  loudnessEnabled: boolean;
  playbackRate: PlaybackRate;
}

export interface PlayerSnapshot {
  queue: readonly PlayableTrack[];
  currentIndex: number;
  currentTrack: PlayableTrack | null;
  status: PlayerStatus;
  playing: boolean;
  /** Incremented whenever a track starts fresh or restarts so the audio engine reloads/resets play-count tracking. */
  playInstanceId: number;
  /** Pending seek target requested by UI / keyboard / lyrics; cleared by the audio engine after applying. */
  seekRequest: { seq: number; sec: number } | null;
  currentTimeSec: number;
  durationSec: number;
  bufferedSec: number;
  volume: number;
  muted: boolean;
  shuffle: boolean;
  repeat: RepeatMode;
  quality: AudioQuality;
  playbackRate: PlaybackRate;
  sleepUntilMs: number | null;
  sleepEndOfTrack: boolean;
  sleepRemainingMinutes: number | null;
  loudnessEnabled: boolean;
  loudness: LoudnessMeasurement | null;
  loudnessScale: number;
  loudnessDb: number;
  playbackMode: "direct" | "transcode" | null;
  error: string | null;
  queueOpen: boolean;
  nowPlayingOpen: boolean;
  nowPlayingTab: NowPlayingTab;
  /** Whether the current playInstanceId has already reported completed=true for play-count increment. */
  playCountReported: boolean;
}

export function isValidPlaybackRate(value: unknown): value is PlaybackRate {
  return typeof value === "number" && (PLAYBACK_RATES as readonly number[]).includes(value);
}

export function formatPlaybackRate(rate: PlaybackRate): string {
  return `${rate}×`;
}

/** Report completed=true once played past 50% or 4 minutes (240s). */
export function shouldReportPlayCount(currentTimeSec: number, durationSec: number): boolean {
  if (!Number.isFinite(currentTimeSec) || currentTimeSec <= 0) return false;
  if (!Number.isFinite(durationSec) || durationSec <= 0) return currentTimeSec >= 240;
  const threshold = Math.min(durationSec * 0.5, 240);
  return currentTimeSec >= threshold;
}

/** Computes linear attenuation factor for `<audio>.volume` relative to -14 LUFS with true-peak protection. */
export function computeLoudnessScale(
  integratedLufs: number | null | undefined,
  truePeakDbtp: number | null | undefined,
  targetLufs = -14,
): { scale: number; gainDb: number } {
  if (integratedLufs == null || !Number.isFinite(integratedLufs)) {
    return { scale: 1, gainDb: 0 };
  }
  let gainDb = targetLufs - integratedLufs;
  if (truePeakDbtp != null && Number.isFinite(truePeakDbtp)) {
    gainDb = Math.min(gainDb, -1 - truePeakDbtp);
  }
  // HTMLAudioElement volume cannot amplify above 1.0, so cap gain at 0 dB and attenuate loud tracks cleanly.
  gainDb = clamp(gainDb, -18, 0);
  const scale = clamp(10 ** (gainDb / 20), 0.125, 1);
  return { scale, gainDb: Math.round(gainDb * 10) / 10 };
}

function shuffleArray<T>(items: readonly T[], random = Math.random): T[] {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [out[i], out[j]] = [out[j]!, out[i]!];
  }
  return out;
}

function readPersisted(): PersistedPlayerState | null {
  const raw = readLocal(PLAYER_STORAGE_KEY);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<PersistedPlayerState>;
    const queueIds = Array.isArray(parsed.queueIds)
      ? parsed.queueIds.filter((id): id is string => typeof id === "string").slice(0, 500)
      : [];
    const volume = typeof parsed.volume === "number" && Number.isFinite(parsed.volume) ? clamp(parsed.volume, 0, 1) : 0.85;
    const repeat: RepeatMode = parsed.repeat === "all" || parsed.repeat === "one" ? parsed.repeat : "off";
    const quality: AudioQuality =
      parsed.quality === "320" || parsed.quality === "192" || parsed.quality === "128" ? parsed.quality : "original";
    const playbackRate: PlaybackRate = isValidPlaybackRate(parsed.playbackRate) ? parsed.playbackRate : 1;
    return {
      queueIds,
      currentIndex: typeof parsed.currentIndex === "number" && Number.isInteger(parsed.currentIndex) ? parsed.currentIndex : 0,
      positionSec: typeof parsed.positionSec === "number" && Number.isFinite(parsed.positionSec) && parsed.positionSec >= 0 ? parsed.positionSec : 0,
      volume,
      muted: Boolean(parsed.muted),
      shuffle: Boolean(parsed.shuffle),
      repeat,
      quality,
      loudnessEnabled: parsed.loudnessEnabled !== false,
      playbackRate,
    };
  } catch {
    return null;
  }
}

export class MusicPlayerStore {
  private queue: PlayableTrack[] = [];
  private unshuffledQueue: PlayableTrack[] | null = null;
  private currentIndex = -1;
  private status: PlayerStatus = "idle";
  private playing = false;
  private playInstanceId = 0;
  private seekSeq = 0;
  private seekRequest: { seq: number; sec: number } | null = null;
  private currentTimeSec = 0;
  private durationSec = 0;
  private bufferedSec = 0;
  private volume = 0.85;
  private muted = false;
  private shuffle = false;
  private repeat: RepeatMode = "off";
  private quality: AudioQuality = "original";
  private playbackRate: PlaybackRate = 1;
  private sleepUntilMs: number | null = null;
  private sleepEndOfTrack = false;
  private sleepTimeout: ReturnType<typeof setTimeout> | null = null;
  private sleepTickInterval: ReturnType<typeof setInterval> | null = null;
  private loudnessEnabled = true;
  private loudness: LoudnessMeasurement | null = null;
  private loudnessScale = 1;
  private loudnessDb = 0;
  private playbackMode: "direct" | "transcode" | null = null;
  private error: string | null = null;
  private queueOpen = false;
  private nowPlayingOpen = false;
  private nowPlayingTab: NowPlayingTab = "lyrics";
  private playCountReported = false;
  private lastPersistedAt = 0;
  private listeners = new Set<() => void>();
  private snapshot: PlayerSnapshot;
  private random: () => number;

  constructor(options: { random?: () => number; loadPersisted?: boolean } = {}) {
    this.random = options.random ?? Math.random;
    if (options.loadPersisted !== false) {
      const saved = readPersisted();
      if (saved) {
        this.volume = saved.volume;
        this.muted = saved.muted;
        this.shuffle = saved.shuffle;
        this.repeat = saved.repeat;
        this.quality = saved.quality;
        this.loudnessEnabled = saved.loudnessEnabled;
        this.playbackRate = saved.playbackRate;
      }
    }
    this.snapshot = this.buildSnapshot();
  }

  private computeSleepRemainingMinutes(nowMs = Date.now()): number | null {
    if (this.sleepUntilMs === null) return null;
    const remainingMs = this.sleepUntilMs - nowMs;
    if (remainingMs <= 0) return 0;
    return Math.max(1, Math.ceil(remainingMs / 60_000));
  }

  private clearSleepTimers() {
    if (this.sleepTimeout !== null) {
      clearTimeout(this.sleepTimeout);
      this.sleepTimeout = null;
    }
    if (this.sleepTickInterval !== null) {
      clearInterval(this.sleepTickInterval);
      this.sleepTickInterval = null;
    }
  }

  private buildSnapshot(nowMs = Date.now()): PlayerSnapshot {
    const currentTrack = this.currentIndex >= 0 ? (this.queue[this.currentIndex] ?? null) : null;
    return {
      queue: this.queue,
      currentIndex: this.currentIndex,
      currentTrack,
      status: this.status,
      playing: this.playing,
      playInstanceId: this.playInstanceId,
      seekRequest: this.seekRequest,
      currentTimeSec: this.currentTimeSec,
      durationSec: this.durationSec,
      bufferedSec: this.bufferedSec,
      volume: this.volume,
      muted: this.muted,
      shuffle: this.shuffle,
      repeat: this.repeat,
      quality: this.quality,
      playbackRate: this.playbackRate,
      sleepUntilMs: this.sleepUntilMs,
      sleepEndOfTrack: this.sleepEndOfTrack,
      sleepRemainingMinutes: this.computeSleepRemainingMinutes(nowMs),
      loudnessEnabled: this.loudnessEnabled,
      loudness: this.loudness,
      loudnessScale: this.loudnessScale,
      loudnessDb: this.loudnessDb,
      playbackMode: this.playbackMode,
      error: this.error,
      queueOpen: this.queueOpen,
      nowPlayingOpen: this.nowPlayingOpen,
      nowPlayingTab: this.nowPlayingTab,
      playCountReported: this.playCountReported,
    };
  }

  private emit(persist = false, nowMs = Date.now()) {
    this.snapshot = this.buildSnapshot(nowMs);
    if (persist) this.persist(true);
    for (const listener of this.listeners) listener();
  }

  private announceExclusive() {
    if (typeof window === "undefined") return;
    window.dispatchEvent(new CustomEvent(MEDIA_EXCLUSIVE_EVENT, { detail: { source: "music" } }));
  }

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  getSnapshot = (): PlayerSnapshot => this.snapshot;

  /** Reads persisted queue ids & position so PlayerHost can hydrate them via lookupTracks(). */
  readPersistedState(): PersistedPlayerState | null {
    return readPersisted();
  }

  persist(force = false) {
    const now = Date.now();
    if (!force && now - this.lastPersistedAt < 2500) return;
    this.lastPersistedAt = now;
    const data: PersistedPlayerState = {
      queueIds: this.queue.map((t) => t.id).slice(0, 500),
      currentIndex: Math.max(0, this.currentIndex),
      positionSec: Math.floor(this.currentTimeSec),
      volume: this.volume,
      muted: this.muted,
      shuffle: this.shuffle,
      repeat: this.repeat,
      quality: this.quality,
      loudnessEnabled: this.loudnessEnabled,
      playbackRate: this.playbackRate,
    };
    writeLocal(PLAYER_STORAGE_KEY, JSON.stringify(data));
  }

  /** Restores a previously saved queue in paused state (called after POST /music/tracks/lookup). */
  hydrateQueue(tracks: PlayableTrack[], index: number, positionSec: number) {
    if (this.queue.length > 0 || tracks.length === 0) return;
    const clampedIndex = clamp(index, 0, tracks.length - 1);
    const current = tracks[clampedIndex]!;
    this.queue = [...tracks];
    this.unshuffledQueue = this.shuffle ? [...tracks] : null;
    this.currentIndex = clampedIndex;
    this.durationSec = (current.durationMs ?? 0) / 1000;
    this.currentTimeSec = this.durationSec > 0 ? clamp(positionSec, 0, Math.max(0, this.durationSec - 1)) : Math.max(0, positionSec);
    this.playing = false;
    this.status = "paused";
    this.playCountReported = false;
    this.emit(false);
  }

  /** Replaces the queue and starts playback immediately. */
  playQueue(tracks: PlayableTrack[], startIndex = 0, options: { shuffle?: boolean; startPositionSec?: number } = {}) {
    if (tracks.length === 0) return;
    const useShuffle = options.shuffle ?? this.shuffle;
    this.shuffle = useShuffle;
    if (useShuffle) {
      this.unshuffledQueue = [...tracks];
      if (options.shuffle === true && startIndex < 0) {
        this.queue = shuffleArray(tracks, this.random);
      } else {
        const pickedIndex = clamp(startIndex, 0, tracks.length - 1);
        const picked = tracks[pickedIndex]!;
        const rest = tracks.filter((_, i) => i !== pickedIndex);
        this.queue = [picked, ...shuffleArray(rest, this.random)];
      }
      this.currentIndex = 0;
    } else {
      this.unshuffledQueue = null;
      this.queue = [...tracks];
      this.currentIndex = clamp(startIndex, 0, tracks.length - 1);
    }
    this.startCurrentTrack(options.startPositionSec ?? 0);
  }

  /** Convenience: play a single track, optionally within a surrounding track list. */
  playTrack(track: PlayableTrack, list?: PlayableTrack[], options: { startPositionSec?: number } = {}) {
    if (list && list.length > 0) {
      const idx = list.findIndex((item) => item.id === track.id);
      if (idx >= 0) {
        this.playQueue(list, idx, { startPositionSec: options.startPositionSec });
        return;
      }
    }
    this.playQueue([track], 0, { shuffle: false, startPositionSec: options.startPositionSec });
  }

  /** Starts shuffled playback of a list of tracks. */
  playShuffled(tracks: PlayableTrack[]) {
    if (tracks.length === 0) return;
    this.playQueue(tracks, -1, { shuffle: true });
  }

  /** Inserts one or more tracks right after the currently playing track (or starts playing if idle). */
  playNext(input: PlayableTrack | PlayableTrack[]) {
    const items = Array.isArray(input) ? input : [input];
    if (items.length === 0) return;
    if (this.queue.length === 0 || this.currentIndex < 0) {
      this.playQueue(items, 0, { shuffle: false });
      return;
    }
    const insertAt = this.currentIndex + 1;
    this.queue = [...this.queue.slice(0, insertAt), ...items, ...this.queue.slice(insertAt)];
    if (this.unshuffledQueue) {
      this.unshuffledQueue = [...this.unshuffledQueue, ...items];
    }
    this.emit(true);
  }

  /** Appends one or more tracks to the end of the queue (or starts playing if idle). */
  enqueue(input: PlayableTrack | PlayableTrack[]) {
    const items = Array.isArray(input) ? input : [input];
    if (items.length === 0) return;
    if (this.queue.length === 0 || this.currentIndex < 0) {
      this.playQueue(items, 0, { shuffle: false });
      return;
    }
    this.queue = [...this.queue, ...items];
    if (this.unshuffledQueue) {
      this.unshuffledQueue = [...this.unshuffledQueue, ...items];
    }
    this.emit(true);
  }

  removeAt(index: number) {
    if (index < 0 || index >= this.queue.length) return;
    const removed = this.queue[index]!;
    const nextQueue = this.queue.filter((_, i) => i !== index);
    if (this.unshuffledQueue) {
      const unshuffledIdx = this.unshuffledQueue.findIndex((t) => t.id === removed.id);
      if (unshuffledIdx >= 0) {
        this.unshuffledQueue = this.unshuffledQueue.filter((_, i) => i !== unshuffledIdx);
      }
    }
    this.queue = nextQueue;
    if (nextQueue.length === 0) {
      this.currentIndex = -1;
      this.playing = false;
      this.status = "idle";
      this.currentTimeSec = 0;
      this.durationSec = 0;
      this.bufferedSec = 0;
      this.error = null;
      this.loudness = null;
      this.loudnessScale = 1;
      this.loudnessDb = 0;
      this.nowPlayingOpen = false;
      this.emit(true);
      return;
    }
    if (index < this.currentIndex) {
      this.currentIndex -= 1;
      this.emit(true);
    } else if (index === this.currentIndex) {
      this.currentIndex = Math.min(index, nextQueue.length - 1);
      if (this.playing) this.startCurrentTrack();
      else {
        const current = this.queue[this.currentIndex]!;
        this.playInstanceId += 1;
        this.currentTimeSec = 0;
        this.durationSec = (current.durationMs ?? 0) / 1000;
        this.bufferedSec = 0;
        this.playCountReported = false;
        this.loudness = null;
        this.loudnessScale = 1;
        this.loudnessDb = 0;
        this.emit(true);
      }
    } else {
      this.emit(true);
    }
  }

  moveInQueue(fromIndex: number, toIndex: number) {
    if (fromIndex === toIndex) return;
    if (fromIndex < 0 || fromIndex >= this.queue.length || toIndex < 0 || toIndex >= this.queue.length) return;
    const nextQueue = [...this.queue];
    const [moved] = nextQueue.splice(fromIndex, 1);
    nextQueue.splice(toIndex, 0, moved!);
    this.queue = nextQueue;
    if (fromIndex === this.currentIndex) {
      this.currentIndex = toIndex;
    } else if (fromIndex < this.currentIndex && toIndex >= this.currentIndex) {
      this.currentIndex -= 1;
    } else if (fromIndex > this.currentIndex && toIndex <= this.currentIndex) {
      this.currentIndex += 1;
    }
    this.emit(true);
  }

  jumpTo(index: number) {
    if (index < 0 || index >= this.queue.length) return;
    this.currentIndex = index;
    this.startCurrentTrack();
  }

  clearQueue() {
    this.clearSleepTimers();
    this.sleepUntilMs = null;
    this.sleepEndOfTrack = false;
    this.queue = [];
    this.unshuffledQueue = null;
    this.currentIndex = -1;
    this.playing = false;
    this.status = "idle";
    this.currentTimeSec = 0;
    this.durationSec = 0;
    this.bufferedSec = 0;
    this.error = null;
    this.loudness = null;
    this.loudnessScale = 1;
    this.loudnessDb = 0;
    this.playbackMode = null;
    this.playCountReported = false;
    this.queueOpen = false;
    this.nowPlayingOpen = false;
    this.emit(true);
  }

  private startCurrentTrack(startPositionSec = 0) {
    const current = this.queue[this.currentIndex];
    if (!current) return;
    const durationSec = (current.durationMs ?? 0) / 1000;
    const initialSec =
      startPositionSec > 0
        ? durationSec > 0
          ? clamp(startPositionSec, 0, Math.max(0, durationSec - 0.5))
          : startPositionSec
        : 0;
    this.playInstanceId += 1;
    this.currentTimeSec = initialSec;
    this.durationSec = durationSec;
    this.bufferedSec = 0;
    this.playing = true;
    this.status = "loading";
    this.error = null;
    this.loudness = null;
    this.loudnessScale = 1;
    this.loudnessDb = 0;
    this.playCountReported = false;
    this.seekRequest = null;
    this.announceExclusive();
    this.emit(true);
  }

  play() {
    if (this.queue.length === 0) return;
    if (this.currentIndex < 0) {
      this.currentIndex = 0;
      this.startCurrentTrack();
      return;
    }
    this.playing = true;
    this.error = null;
    if (this.status === "idle" || this.status === "error") {
      this.playInstanceId += 1;
      this.status = "loading";
    }
    this.announceExclusive();
    this.emit(false);
  }

  pause() {
    if (!this.playing && this.status === "paused") return;
    this.playing = false;
    if (this.queue.length > 0 && this.status !== "error") this.status = "paused";
    this.emit(true);
  }

  togglePlay() {
    if (this.playing) this.pause();
    else this.play();
  }

  /** Advance to next track. `manual: true` bypasses repeat="one" so user clicks always skip. */
  next(manual = true) {
    if (this.queue.length === 0) return;
    if (!manual && this.sleepEndOfTrack) {
      this.sleepEndOfTrack = false;
      this.playing = false;
      this.status = "paused";
      this.emit(true);
      return;
    }
    if (!manual && this.repeat === "one") {
      this.startCurrentTrack();
      return;
    }
    if (this.currentIndex + 1 < this.queue.length) {
      this.currentIndex += 1;
      this.startCurrentTrack();
    } else if (this.repeat === "all") {
      this.currentIndex = 0;
      this.startCurrentTrack();
    } else {
      this.playing = false;
      this.status = "paused";
      this.currentTimeSec = 0;
      this.emit(true);
    }
  }

  /** Rewind to 0 if >3s into track, otherwise step back. */
  previous() {
    if (this.queue.length === 0) return;
    if (this.currentTimeSec > 3) {
      this.seek(0);
      return;
    }
    if (this.currentIndex > 0) {
      this.currentIndex -= 1;
      this.startCurrentTrack();
    } else if (this.repeat === "all") {
      this.currentIndex = this.queue.length - 1;
      this.startCurrentTrack();
    } else {
      this.seek(0);
    }
  }

  /** Returns the upcoming track for preloading, respecting repeat mode. */
  peekNextTrack(): PlayableTrack | null {
    if (this.queue.length === 0 || this.currentIndex < 0) return null;
    if (this.repeat === "one" || this.sleepEndOfTrack) return null;
    if (this.currentIndex + 1 < this.queue.length) return this.queue[this.currentIndex + 1] ?? null;
    if (this.repeat === "all" && this.queue.length > 1) return this.queue[0] ?? null;
    return null;
  }

  seek(sec: number) {
    if (this.currentIndex < 0) return;
    const clamped = this.durationSec > 0 ? clamp(sec, 0, this.durationSec) : Math.max(0, sec);
    this.currentTimeSec = clamped;
    this.seekSeq += 1;
    this.seekRequest = { seq: this.seekSeq, sec: clamped };
    this.emit(true);
  }

  seekBy(deltaSec: number) {
    this.seek(this.currentTimeSec + deltaSec);
  }

  clearSeekRequest(seq: number) {
    if (this.seekRequest?.seq === seq) {
      this.seekRequest = null;
      this.emit(false);
    }
  }

  setVolume(volume: number) {
    const clamped = clamp(volume, 0, 1);
    this.volume = clamped;
    if (clamped > 0 && this.muted) this.muted = false;
    this.emit(true);
  }

  toggleMute() {
    this.muted = !this.muted;
    this.emit(true);
  }

  toggleShuffle() {
    if (!this.shuffle) {
      this.shuffle = true;
      if (this.queue.length > 1) {
        this.unshuffledQueue = [...this.queue];
        const current = this.currentIndex >= 0 ? this.queue[this.currentIndex] : null;
        if (current) {
          const rest = this.queue.filter((_, i) => i !== this.currentIndex);
          this.queue = [current, ...shuffleArray(rest, this.random)];
          this.currentIndex = 0;
        } else {
          this.queue = shuffleArray(this.queue, this.random);
        }
      }
    } else {
      this.shuffle = false;
      if (this.unshuffledQueue) {
        const current = this.currentIndex >= 0 ? this.queue[this.currentIndex] : null;
        this.queue = [...this.unshuffledQueue];
        this.unshuffledQueue = null;
        if (current) {
          const restoredIdx = this.queue.findIndex((t) => t.id === current.id);
          this.currentIndex = restoredIdx >= 0 ? restoredIdx : 0;
        }
      }
    }
    this.emit(true);
  }

  cycleRepeat() {
    this.repeat = this.repeat === "off" ? "all" : this.repeat === "all" ? "one" : "off";
    this.emit(true);
  }

  setRepeat(repeat: RepeatMode) {
    this.repeat = repeat;
    this.emit(true);
  }

  setQuality(quality: AudioQuality) {
    if (this.quality === quality) return;
    this.quality = quality;
    this.emit(true);
  }

  setPlaybackRate(rate: PlaybackRate) {
    if (!isValidPlaybackRate(rate) || this.playbackRate === rate) return;
    this.playbackRate = rate;
    this.emit(true);
  }

  setSleepTimer(preset: SleepTimerPreset, nowMs = Date.now()) {
    this.clearSleepTimers();
    if (preset === "off") {
      this.sleepUntilMs = null;
      this.sleepEndOfTrack = false;
      this.emit(false, nowMs);
      return;
    }
    if (preset === "end_of_track") {
      this.sleepUntilMs = null;
      this.sleepEndOfTrack = true;
      this.emit(false, nowMs);
      return;
    }
    const durationMs = preset * 60_000;
    this.sleepEndOfTrack = false;
    this.sleepUntilMs = nowMs + durationMs;
    this.sleepTimeout = setTimeout(() => {
      this.checkSleepTimer();
    }, durationMs);
    this.sleepTickInterval = setInterval(() => {
      this.tickSleepTimer();
    }, 15_000);
    this.emit(false, nowMs);
  }

  private tickSleepTimer(nowMs = Date.now()) {
    if (this.sleepUntilMs === null) return;
    if (this.checkSleepTimer(nowMs)) return;
    const nextMinutes = this.computeSleepRemainingMinutes(nowMs);
    if (nextMinutes !== this.snapshot.sleepRemainingMinutes) {
      this.emit(false, nowMs);
    }
  }

  /** Checks whether a timed sleep countdown has expired, auto-pausing playback if so. */
  checkSleepTimer(nowMs = Date.now()): boolean {
    if (this.sleepUntilMs === null || nowMs < this.sleepUntilMs) return false;
    this.clearSleepTimers();
    this.sleepUntilMs = null;
    this.sleepEndOfTrack = false;
    if (this.playing) {
      this.pause();
    } else {
      this.emit(false, nowMs);
    }
    return true;
  }

  setLoudnessEnabled(enabled: boolean) {
    this.loudnessEnabled = enabled;
    this.recomputeLoudness();
    this.emit(true);
  }

  setLoudnessMeasurement(measurement: LoudnessMeasurement | null) {
    this.loudness = measurement;
    this.recomputeLoudness();
    this.emit(false);
  }

  private recomputeLoudness() {
    if (!this.loudnessEnabled || !this.loudness) {
      this.loudnessScale = 1;
      this.loudnessDb = 0;
      return;
    }
    const { scale, gainDb } = computeLoudnessScale(this.loudness.integratedLufs, this.loudness.truePeakDbtp);
    this.loudnessScale = scale;
    this.loudnessDb = gainDb;
  }

  /** Effective audio volume factoring in mute and loudness attenuation. */
  effectiveVolume(): number {
    if (this.muted) return 0;
    return clamp(this.volume * this.loudnessScale, 0, 1);
  }

  setQueueOpen(open: boolean) {
    if (this.queueOpen === open) return;
    this.queueOpen = open;
    this.emit(false);
  }

  toggleQueue() {
    this.setQueueOpen(!this.queueOpen);
  }

  openNowPlaying(tab?: NowPlayingTab) {
    this.nowPlayingOpen = true;
    if (tab) this.nowPlayingTab = tab;
    this.queueOpen = false;
    this.emit(false);
  }

  closeNowPlaying() {
    if (!this.nowPlayingOpen) return;
    this.nowPlayingOpen = false;
    this.emit(false);
  }

  toggleNowPlaying(tab: NowPlayingTab = "lyrics") {
    if (this.nowPlayingOpen && this.nowPlayingTab === tab) {
      this.closeNowPlaying();
    } else if (this.queue.length > 0) {
      this.openNowPlaying(tab);
    }
  }

  setNowPlayingTab(tab: NowPlayingTab) {
    if (this.nowPlayingTab === tab) return;
    this.nowPlayingTab = tab;
    this.emit(false);
  }

  /* ---- Audio engine callbacks ---- */

  reportTime(currentTimeSec: number, durationSec?: number, bufferedSec?: number) {
    this.currentTimeSec = Math.max(0, currentTimeSec);
    if (durationSec !== undefined && Number.isFinite(durationSec) && durationSec > 0) {
      this.durationSec = durationSec;
    }
    if (bufferedSec !== undefined && Number.isFinite(bufferedSec) && bufferedSec >= 0) {
      this.bufferedSec = bufferedSec;
    }
    if (this.sleepUntilMs !== null && Date.now() >= this.sleepUntilMs) {
      this.checkSleepTimer();
      return;
    }
    this.persist(false);
    this.emit(false);
  }

  reportStatus(status: PlayerStatus, options: { mode?: "direct" | "transcode" | null; error?: string | null } = {}) {
    this.status = status;
    if (status === "playing") this.playing = true;
    if (status === "paused" || status === "error") this.playing = false;
    if (options.mode !== undefined) this.playbackMode = options.mode;
    if (options.error !== undefined) this.error = options.error;
    this.emit(false);
  }

  markPlayCountReported() {
    if (this.playCountReported) return;
    this.playCountReported = true;
    this.emit(false);
  }

  /** Syncs personal state updates (favorite/stars/playCount) across all matching tracks in the queue. */
  patchTrackState(trackId: string, patch: Partial<MusicTrackState>) {
    let changed = false;
    const apply = (list: PlayableTrack[]) =>
      list.map((track) => {
        if (track.id !== trackId) return track;
        changed = true;
        return { ...track, state: { ...track.state, ...patch } };
      });
    this.queue = apply(this.queue);
    if (this.unshuffledQueue) this.unshuffledQueue = apply(this.unshuffledQueue);
    if (changed) this.emit(false);
  }

  /** Handles `palveni:media-exclusive` events from other players (e.g., fullscreen video). */
  handleExclusiveEvent = (event: Event) => {
    const detail = (event as CustomEvent<{ source?: string }>).detail;
    if (detail?.source === "music") return;
    if (this.playing) this.pause();
  };

  /** Resets store completely (used in unit tests). */
  reset(clearStorage = true) {
    this.clearSleepTimers();
    this.queue = [];
    this.unshuffledQueue = null;
    this.currentIndex = -1;
    this.status = "idle";
    this.playing = false;
    this.playInstanceId = 0;
    this.seekSeq = 0;
    this.seekRequest = null;
    this.currentTimeSec = 0;
    this.durationSec = 0;
    this.bufferedSec = 0;
    this.volume = 0.85;
    this.muted = false;
    this.shuffle = false;
    this.repeat = "off";
    this.quality = "original";
    this.playbackRate = 1;
    this.sleepUntilMs = null;
    this.sleepEndOfTrack = false;
    this.loudnessEnabled = true;
    this.loudness = null;
    this.loudnessScale = 1;
    this.loudnessDb = 0;
    this.playbackMode = null;
    this.error = null;
    this.queueOpen = false;
    this.nowPlayingOpen = false;
    this.nowPlayingTab = "lyrics";
    this.playCountReported = false;
    this.lastPersistedAt = 0;
    if (clearStorage) removeLocal(PLAYER_STORAGE_KEY);
    this.emit(false);
  }
}

export const playerStore = new MusicPlayerStore();

export function usePlayer(): PlayerSnapshot {
  return useSyncExternalStore(playerStore.subscribe, playerStore.getSnapshot, playerStore.getSnapshot);
}
