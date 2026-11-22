/**
 * Global player host mounted once inside `AppShell`.
 * Coordinates audio playback, queue persistence restoration (`POST /music/tracks/lookup`),
 * keyboard shortcuts (Space, ←/→, Shift+←/→, M, L), `palveni:media-exclusive` events,
 * the persistent bottom bar, queue drawer and full-screen Now Playing overlay.
 */
import { useEffect, useRef } from "react";
import { lookupTracks } from "@/modules/music/api";
import { useFeature } from "@/modules/access";
import { useAudioEngine } from "./audio-engine";
import { NowPlayingOverlay } from "./now-playing";
import { PlayerBar } from "./player-bar";
import { QueueDrawer } from "./queue-drawer";
import { MEDIA_EXCLUSIVE_EVENT, playerStore } from "./store";

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

function isInteractiveControl(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return Boolean(target.closest("button, a[href], summary, [role='button'], [role='menuitem'], [role='option'], [role='tab'], [role='checkbox'], [role='switch']"));
}

export function PlayerHost() {
  const musicEnabled = useFeature("music");
  const hydratedRef = useRef(false);
  useAudioEngine();

  // Restore saved queue + position from localStorage via POST /music/tracks/lookup.
  useEffect(() => {
    if (!musicEnabled || hydratedRef.current) return;
    hydratedRef.current = true;
    const saved = playerStore.readPersistedState();
    if (!saved || saved.queueIds.length === 0) return;
    const controller = new AbortController();
    void lookupTracks(saved.queueIds, controller.signal)
      .then((tracks) => {
        if (!controller.signal.aborted && tracks.length > 0) {
          playerStore.hydrateQueue(tracks, saved.currentIndex, saved.positionSec);
        }
      })
      .catch(() => {
        // Ignore hydration errors (e.g. offline or empty catalogue).
      });
    return () => controller.abort();
  }, [musicEnabled]);

  // Pause music when fullscreen video (or another exclusive media player) starts.
  useEffect(() => {
    window.addEventListener(MEDIA_EXCLUSIVE_EVENT, playerStore.handleExclusiveEvent);
    return () => window.removeEventListener(MEDIA_EXCLUSIVE_EVENT, playerStore.handleExclusiveEvent);
  }, []);

  // Persist current position cleanly before tab unload.
  useEffect(() => {
    const onUnload = () => playerStore.persist(true);
    window.addEventListener("beforeunload", onUnload);
    return () => window.removeEventListener("beforeunload", onUnload);
  }, []);

  // Global keyboard shortcuts (Space, ←/→, Shift+←/→, M, L).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
      if (isEditableTarget(event.target)) return;
      const snap = playerStore.getSnapshot();
      if (!snap.currentTrack) return;

      if (event.code === "Space") {
        if (isInteractiveControl(event.target)) return;
        event.preventDefault();
        playerStore.togglePlay();
        return;
      }
      if (event.key === "ArrowLeft") {
        if (isInteractiveControl(event.target) && !event.shiftKey) return;
        event.preventDefault();
        if (event.shiftKey) playerStore.previous();
        else playerStore.seekBy(-5);
        return;
      }
      if (event.key === "ArrowRight") {
        if (isInteractiveControl(event.target) && !event.shiftKey) return;
        event.preventDefault();
        if (event.shiftKey) playerStore.next(true);
        else playerStore.seekBy(5);
        return;
      }
      if (!event.shiftKey && (event.key === "m" || event.key === "M")) {
        event.preventDefault();
        playerStore.toggleMute();
        return;
      }
      if (!event.shiftKey && (event.key === "l" || event.key === "L")) {
        event.preventDefault();
        playerStore.toggleNowPlaying("lyrics");
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <>
      <PlayerBar />
      <QueueDrawer />
      <NowPlayingOverlay />
    </>
  );
}
