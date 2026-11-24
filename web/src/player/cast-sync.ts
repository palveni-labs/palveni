/**
 * Multi-device playback session synchronization, heartbeat throttling,
 * cover-color ambient accent derivation, and SSE remote command handler.
 */
import { useEffect, useRef } from "react";
import { request } from "@/api/client";
import { subscribePlaybackCommands, type PlaybackCommandEvent } from "@/api/events";
import { useFeature } from "@/modules/access";
import { playerStore, usePlayer, type MusicPlayerStore } from "./store";

export type PlaybackClientKind = "web" | "ios" | "tvos" | "macos" | "android" | "desktop";
export type PlaybackMediaKind = "audio" | "video" | "audiobook" | "none";
export type PlaybackSessionState = "playing" | "paused" | "stopped" | "idle";
export type PlaybackCommandKind = "play" | "pause" | "toggle" | "seek" | "next" | "prev" | "volume" | "play_item";

export interface PlaybackSession {
  id: string;
  userId: string;
  deviceName: string;
  clientKind: PlaybackClientKind;
  mediaKind: PlaybackMediaKind;
  itemId?: string | null;
  libraryId?: string | null;
  title?: string | null;
  subtitle?: string | null;
  artworkId?: string | null;
  state: PlaybackSessionState;
  positionMs: number;
  durationMs: number;
  volume: number;
  ambientHex?: string | null;
  updatedAt: string;
}

export interface PlaybackSessionList {
  sessions: PlaybackSession[];
}

export interface PlaybackHeartbeatRequest {
  sessionId: string;
  deviceName: string;
  clientKind: PlaybackClientKind;
  mediaKind: PlaybackMediaKind;
  itemId?: string;
  libraryId?: string;
  title?: string;
  subtitle?: string;
  artworkId?: string;
  state: PlaybackSessionState;
  positionMs: number;
  durationMs: number;
  volume: number;
  ambientHex?: string;
}

export interface PlaybackCommandRequest {
  command: PlaybackCommandKind;
  positionMs?: number;
  volume?: number;
  itemId?: string;
}

export interface PlaybackCommandAck {
  accepted: boolean;
  targetSessionId: string;
  command: PlaybackCommandKind;
}

export const CAST_SESSION_STORAGE_KEY = "palveni.cast.sessionId";
const HEARTBEAT_INTERVAL_MS = 25_000;
const HEARTBEAT_THROTTLE_MS = 800;

/** Curated warm & cool ambient accents (no purple/indigo by design). */
const AMBIENT_PALETTE = [
  "#D97706", // warm amber
  "#EA580C", // sunset orange
  "#CA8A04", // soft gold
  "#B45309", // bronze
  "#0F766E", // deep teal
  "#0284C7", // clear sky
  "#0369A1", // ocean blue
  "#15803D", // forest sage
  "#C2410C", // terracotta
  "#2563EB", // cobalt
] as const;

/** Derives a deterministic `#RRGGBB` ambient accent from artwork/track identity. */
export function deriveAmbientHex(seed: string): string {
  let hash = 2166136261;
  for (let i = 0; i < seed.length; i++) {
    hash ^= seed.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  const index = Math.abs(hash) % AMBIENT_PALETTE.length;
  return AMBIENT_PALETTE[index] ?? "#D97706";
}

let fallbackSessionId: string | null = null;

/** Returns a stable session identifier for this browser tab (stored in `sessionStorage`). */
export function getLocalPlaybackSessionId(): string {
  try {
    const existing = window.sessionStorage.getItem(CAST_SESSION_STORAGE_KEY);
    if (existing && existing.trim().length > 0) return existing;
    const generated =
      typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
        ? `web-${crypto.randomUUID().slice(0, 12)}`
        : `web-${Math.random().toString(16).slice(2, 14)}`;
    window.sessionStorage.setItem(CAST_SESSION_STORAGE_KEY, generated);
    return generated;
  } catch {
    fallbackSessionId ??= `web-${Math.random().toString(16).slice(2, 14)}`;
    return fallbackSessionId;
  }
}

export function detectBrowserDeviceName(): string {
  if (typeof navigator === "undefined") return "Palveni Web";
  const ua = navigator.userAgent;
  if (/iPad/i.test(ua)) return "Palveni Web (iPad)";
  if (/iPhone/i.test(ua)) return "Palveni Web (iPhone)";
  if (/Mac OS X|Macintosh/i.test(ua)) return "Palveni Web (macOS)";
  if (/Windows/i.test(ua)) return "Palveni Web (Windows)";
  if (/Android/i.test(ua)) return "Palveni Web (Android)";
  if (/Linux/i.test(ua)) return "Palveni Web (Linux)";
  return "Palveni Web";
}

/** Executes a remote `playback.command` event against the local music player store. */
export function applyRemotePlaybackCommand(
  event: PlaybackCommandEvent,
  localSessionId: string,
  store: MusicPlayerStore = playerStore,
): boolean {
  if (event.targetSessionId !== localSessionId) return false;
  switch (event.command) {
    case "play":
      store.play();
      return true;
    case "pause":
      store.pause();
      return true;
    case "toggle":
      store.togglePlay();
      return true;
    case "seek":
      if (typeof event.positionMs === "number" && Number.isFinite(event.positionMs)) {
        store.seek(Math.max(0, event.positionMs / 1000));
      }
      return true;
    case "volume":
      if (typeof event.volume === "number" && Number.isFinite(event.volume)) {
        store.setVolume(Math.max(0, Math.min(1, event.volume / 100)));
      }
      return true;
    case "next":
      store.next(true);
      return true;
    case "prev":
      store.previous();
      return true;
    default:
      return false;
  }
}

export function sendPlaybackHeartbeat(body: PlaybackHeartbeatRequest): Promise<PlaybackSession> {
  return request<PlaybackSession>("/sessions/playback/heartbeat", { method: "POST", body });
}

export function listPlaybackSessions(signal?: AbortSignal): Promise<PlaybackSessionList> {
  return request<PlaybackSessionList>("/sessions/playback", { signal });
}

export function sendPlaybackCommand(id: string, body: PlaybackCommandRequest): Promise<PlaybackCommandAck> {
  return request<PlaybackCommandAck>(`/sessions/playback/${encodeURIComponent(id)}/command`, {
    method: "POST",
    body,
  });
}

function buildHeartbeatPayload(sessionId: string): PlaybackHeartbeatRequest {
  const snap = playerStore.getSnapshot();
  const track = snap.currentTrack;
  const effectiveVolume = snap.muted ? 0 : Math.round(snap.volume * 100);
  const state: PlaybackSessionState = !track
    ? "idle"
    : snap.playing
      ? "playing"
      : snap.status === "paused"
        ? "paused"
        : "idle";
  const mediaKind: PlaybackMediaKind = !track
    ? "none"
    : track.spokenKind === "audiobook"
      ? "audiobook"
      : "audio";
  const durationSec = snap.durationSec || (track?.durationMs ?? 0) / 1000;
  const ambientSeed = track ? (track.artworkItemId ?? track.albumId ?? track.id) : null;
  return {
    sessionId,
    deviceName: detectBrowserDeviceName(),
    clientKind: "web",
    mediaKind,
    itemId: track?.id,
    libraryId: track?.libraryId,
    title: track?.title,
    subtitle: track?.artist ?? track?.album ?? undefined,
    artworkId: track ? (track.artworkItemId ?? track.id) : undefined,
    state,
    positionMs: Math.max(0, Math.round(snap.currentTimeSec * 1000)),
    durationMs: Math.max(0, Math.round(durationSec * 1000)),
    volume: Math.max(0, Math.min(100, effectiveVolume)),
    ambientHex: ambientSeed ? deriveAmbientHex(ambientSeed) : undefined,
  };
}

/**
 * Mounts once in `PlayerHost` to:
 * 1. Report throttled playback heartbeats on track/play-state/volume changes and every 25s while playing.
 * 2. Listen for `playback.command` SSE events matching this browser tab's `sessionId`.
 */
export function usePlaybackSessionSync(forceEnabled?: boolean) {
  const featureCast = useFeature("cast");
  const enabled = forceEnabled ?? featureCast;
  const player = usePlayer();
  const hasSentRef = useRef(false);
  const lastSentAtRef = useRef(0);
  const throttleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const trackId = player.currentTrack?.id ?? null;
  const playing = player.playing;
  const status = player.status;
  const volumePct = player.muted ? 0 : Math.round(player.volume * 100);

  useEffect(() => {
    if (!enabled) return;
    const localSessionId = getLocalPlaybackSessionId();
    return subscribePlaybackCommands((event) => {
      applyRemotePlaybackCommand(event, localSessionId, playerStore);
    });
  }, [enabled]);

  useEffect(() => {
    if (!enabled) return;
    if (!trackId && !hasSentRef.current) return;

    const sessionId = getLocalPlaybackSessionId();
    const fire = () => {
      throttleTimerRef.current = null;
      lastSentAtRef.current = Date.now();
      hasSentRef.current = true;
      void sendPlaybackHeartbeat(buildHeartbeatPayload(sessionId)).catch(() => {
        // Ignore transient network errors on background heartbeats.
      });
    };

    const elapsed = Date.now() - lastSentAtRef.current;
    if (elapsed >= HEARTBEAT_THROTTLE_MS) {
      fire();
    } else {
      if (throttleTimerRef.current !== null) clearTimeout(throttleTimerRef.current);
      throttleTimerRef.current = setTimeout(fire, HEARTBEAT_THROTTLE_MS - elapsed);
    }

    return () => {
      if (throttleTimerRef.current !== null) {
        clearTimeout(throttleTimerRef.current);
        throttleTimerRef.current = null;
      }
    };
  }, [enabled, trackId, playing, status, volumePct]);

  useEffect(() => {
    if (!enabled || !playing || !trackId) return;
    const sessionId = getLocalPlaybackSessionId();
    const interval = setInterval(() => {
      lastSentAtRef.current = Date.now();
      hasSentRef.current = true;
      void sendPlaybackHeartbeat(buildHeartbeatPayload(sessionId)).catch(() => {
        // Ignore transient errors.
      });
    }, HEARTBEAT_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [enabled, playing, trackId]);
}
