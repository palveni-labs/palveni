import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { forgetSkin } from "@/skins/store";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";
import type { RegistrySkin } from "../types";

export const MAX_SKIN_BYTES = 16 * 1024 * 1024;

export function useInstalledSkins(enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.skins.installed,
    queryFn: ({ signal }) => api.skins.installed(signal),
    enabled,
    staleTime: 30_000,
  });
}

export function useSkinRegistry(enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.skins.registry,
    queryFn: ({ signal }) => api.skins.registry(signal),
    enabled,
    staleTime: 5 * 60_000,
  });
}

function useRefreshPackages() {
  const client = useQueryClient();
  return () => {
    void client.invalidateQueries({ queryKey: queryKeys.skins.all });
    void client.invalidateQueries({ queryKey: queryKeys.plugins.list });
  };
}

async function fetchOfficialArchive(url: string): Promise<Blob | undefined> {
  if (!url.startsWith("https://")) return undefined;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 8_000);
  try {
    const response = await fetch(url, { credentials: "omit", referrerPolicy: "no-referrer", signal: controller.signal });
    if (!response.ok || !response.body) return undefined;
    const chunks: ArrayBuffer[] = [];
    const reader = response.body.getReader();
    let size = 0;
    while (true) {
      const { value, done } = await reader.read();
      if (done) return new Blob(chunks, { type: "application/x-tar" });
      size += value.byteLength;
      if (size > MAX_SKIN_BYTES) throw new RangeError("Skin archive exceeds 16 MiB");
      chunks.push(new Uint8Array(value).buffer);
    }
  } catch (error) {
    if (error instanceof RangeError) throw error;
    // An offline installation uses the same version-pinned package shipped with the server.
    return undefined;
  } finally {
    controller.abort();
    clearTimeout(timer);
  }
}

export function useInstallSkin() {
  const refresh = useRefreshPackages();
  return useMutation({
    mutationFn: async (skin: RegistrySkin) => api.skins.install(skin.id, await fetchOfficialArchive(skin.downloadUrl)),
    onSuccess: refresh,
  });
}

export function useUploadSkin() {
  return useMutation({ mutationFn: api.skins.upload, onSuccess: useRefreshPackages() });
}

export function useUninstallSkin() {
  const refresh = useRefreshPackages();
  return useMutation({
    mutationFn: api.skins.uninstall,
    onSuccess: (_data, id) => {
      forgetSkin(id);
      refresh();
    },
  });
}
