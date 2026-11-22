/** Active music library persistence store. */
import { useSyncExternalStore } from "react";
import { useLibraries } from "@/api/queries/libraries";
import type { Library } from "@/api/types";
import { readLocal, writeLocal } from "@/lib/storage";

const STORAGE_KEY = "palveni.music.libraryId";
let selectedId: string | null = readLocal(STORAGE_KEY);
const listeners = new Set<() => void>();

export function setSelectedMusicLibrary(libraryId: string) {
  if (selectedId === libraryId) return;
  selectedId = libraryId;
  writeLocal(STORAGE_KEY, libraryId);
  for (const listener of listeners) listener();
}

export function resetSelectedMusicLibrary() {
  selectedId = null;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useMusicLibraries(): {
  libraries: Library[];
  library: Library | null;
  libraryId: string | null;
  selectLibrary: (id: string) => void;
  isPending: boolean;
} {
  const preferred = useSyncExternalStore(subscribe, () => selectedId, () => null);
  const query = useLibraries();
  const libraries = (query.data?.libraries ?? []).filter((lib) => lib.kind === "music");
  const library = libraries.find((lib) => lib.id === preferred) ?? libraries[0] ?? null;
  return {
    libraries,
    library,
    libraryId: library?.id ?? null,
    selectLibrary: setSelectedMusicLibrary,
    isPending: query.isPending,
  };
}
