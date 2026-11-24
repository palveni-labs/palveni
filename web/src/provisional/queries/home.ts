import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import type { DeviceState, DevicePatch } from "@/blocks/types";
import { provisionalApi } from "../api";
import { provisionalKeys } from "../keys";
import type {
  AutomationRule,
  CreateHomeDeviceRequest,
  CreateHomeRuleRequest,
  HomeBridgeSummary,
  HomeDevice,
  HomeSnapshot,
  SaveHomeBridgeRequest,
} from "../types";

export function useHome(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.home,
    queryFn: ({ signal }) => provisionalApi.home.snapshot(signal),
    enabled,
    staleTime: 30_000,
  });
}

/** Applies a `home.device.updated` event. */
export function upsertDevice(queryClient: QueryClient, device: HomeDevice) {
  queryClient.setQueryData<HomeSnapshot>(provisionalKeys.home, (home) =>
    home ? { ...home, devices: home.devices.map((entry) => (entry.id === device.id ? device : entry)) } : home,
  );
}

export function applyPatch(state: DeviceState, patch: DevicePatch): DeviceState {
  return { ...state, ...patch };
}

/** Device control with an optimistic update, rolled back if the call fails. */
export function useUpdateDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: DevicePatch }) => provisionalApi.home.updateDevice(id, patch),
    onMutate: ({ id, patch }) => {
      const previous = queryClient.getQueryData<HomeSnapshot>(provisionalKeys.home);
      queryClient.setQueryData<HomeSnapshot>(provisionalKeys.home, (home) =>
        home
          ? {
              ...home,
              devices: home.devices.map((device) => (device.id === id ? { ...device, state: applyPatch(device.state, patch) } : device)),
            }
          : home,
      );
      return { previous };
    },
    onError: (_error, _variables, context) => {
      if (context?.previous) queryClient.setQueryData(provisionalKeys.home, context.previous);
    },
    onSuccess: (device) => upsertDevice(queryClient, device),
  });
}

export function useActivateScene() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.home.activateScene(id),
    onSuccess: (home) => queryClient.setQueryData(provisionalKeys.home, home),
  });
}

export function useSetRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) => provisionalApi.home.setRule(id, enabled),
    onSuccess: (rule: AutomationRule) =>
      queryClient.setQueryData<HomeSnapshot>(provisionalKeys.home, (home) =>
        home ? { ...home, rules: home.rules.map((entry) => (entry.id === rule.id ? rule : entry)) } : home,
      ),
  });
}

export function useHomeBridges(enabled: boolean) {
  return useQuery({
    queryKey: provisionalKeys.homeBridges,
    queryFn: ({ signal }) => provisionalApi.home.bridges(signal),
    enabled,
    staleTime: 30_000,
  });
}

export function useSaveHomeBridge() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: SaveHomeBridgeRequest) => provisionalApi.home.saveBridge(body),
    onSuccess: (bridge: HomeBridgeSummary) =>
      queryClient.setQueryData<{ bridges: HomeBridgeSummary[] }>(provisionalKeys.homeBridges, (prev) => {
        const list = prev?.bridges ?? [];
        const exists = list.some((entry) => entry.id === bridge.id);
        return { bridges: exists ? list.map((entry) => (entry.id === bridge.id ? bridge : entry)) : [...list, bridge] };
      }),
  });
}

export function useDeleteHomeBridge() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.home.deleteBridge(id),
    onSuccess: (_res, id) => {
      queryClient.setQueryData<{ bridges: HomeBridgeSummary[] }>(provisionalKeys.homeBridges, (prev) =>
        prev ? { bridges: prev.bridges.filter((entry) => entry.id !== id) } : prev,
      );
      void queryClient.invalidateQueries({ queryKey: provisionalKeys.home });
    },
  });
}

export function useSyncHomeBridge() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.home.syncBridge(id),
    onSuccess: (result) => {
      queryClient.setQueryData(provisionalKeys.home, result.snapshot);
      queryClient.setQueryData<{ bridges: HomeBridgeSummary[] }>(provisionalKeys.homeBridges, (prev) =>
        prev ? { bridges: prev.bridges.map((entry) => (entry.id === result.bridge.id ? result.bridge : entry)) } : prev,
      );
    },
  });
}

export function useCreateHomeDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateHomeDeviceRequest) => provisionalApi.home.createDevice(body),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: provisionalKeys.home }),
  });
}

export function useDeleteHomeDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.home.deleteDevice(id),
    onSuccess: (_res, id) =>
      queryClient.setQueryData<HomeSnapshot>(provisionalKeys.home, (home) =>
        home ? { ...home, devices: home.devices.filter((entry) => entry.id !== id) } : home,
      ),
  });
}

export function useCreateHomeRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateHomeRuleRequest) => provisionalApi.home.createRule(body),
    onSuccess: (rule) =>
      queryClient.setQueryData<HomeSnapshot>(provisionalKeys.home, (home) =>
        home ? { ...home, rules: [...home.rules, rule] } : home,
      ),
  });
}

export function useDeleteHomeRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => provisionalApi.home.deleteRule(id),
    onSuccess: (_res, id) =>
      queryClient.setQueryData<HomeSnapshot>(provisionalKeys.home, (home) =>
        home ? { ...home, rules: home.rules.filter((entry) => entry.id !== id) } : home,
      ),
  });
}
