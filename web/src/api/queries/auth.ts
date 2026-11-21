import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";
import { api } from "../endpoints";
import { queryKeys } from "../query-keys";
import { clearSession, setSession } from "../session";
import type { Credentials, SystemInfo, User } from "../types";

export const meQuery = () =>
  queryOptions({
    queryKey: queryKeys.auth.me,
    queryFn: ({ signal }) => api.auth.me(signal),
    staleTime: 5 * 60_000,
    retry: false,
  });

export function useMe() {
  return useQuery(meQuery());
}

/** The signed-in user. Only valid below the authenticated layout, whose guard loads it. */
export function useCurrentUser(): User {
  const { data } = useMe();
  if (!data) throw new Error("useCurrentUser used outside the authenticated layout");
  return data;
}

export function useIsAdmin(): boolean {
  return useMe().data?.role === "admin";
}

/** A short label for the device list on the server, e.g. "Web · Chrome · macOS". */
export function browserDeviceName(userAgent = navigator.userAgent): string {
  const browser = /Edg\//.test(userAgent)
    ? "Edge"
    : /Firefox\//.test(userAgent)
      ? "Firefox"
      : /Chrome\//.test(userAgent)
        ? "Chrome"
        : /Safari\//.test(userAgent)
          ? "Safari"
          : null;
  const os = /iPhone|iPad/.test(userAgent)
    ? "iOS"
    : /Android/.test(userAgent)
      ? "Android"
      : /Mac OS X/.test(userAgent)
        ? "macOS"
        : /Windows/.test(userAgent)
          ? "Windows"
          : /Linux/.test(userAgent)
            ? "Linux"
            : null;
  return ["Web", browser, os].filter(Boolean).join(" · ");
}

export function useLogin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (credentials: Credentials) =>
      api.auth.login({ ...credentials, deviceName: browserDeviceName() }),
    onSuccess: (result) => {
      queryClient.clear();
      setSession({ accessToken: result.accessToken, expiresAt: result.expiresAt });
      queryClient.setQueryData(queryKeys.auth.me, result.user);
    },
  });
}

/** Creates the first admin account, then signs in with the same credentials. */
export function useSetupAdmin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (credentials: Credentials) => {
      await api.setup(credentials);
      return api.auth.login({ ...credentials, deviceName: browserDeviceName() });
    },
    onSuccess: (result) => {
      setSession({ accessToken: result.accessToken, expiresAt: result.expiresAt });
      queryClient.setQueryData(queryKeys.auth.me, result.user);
      queryClient.setQueryData<SystemInfo>(queryKeys.system.info, (info) =>
        info ? { ...info, setupCompleted: true } : info,
      );
    },
  });
}

/**
 * Signs out and goes to the login page. The cached user is dropped only after the authenticated
 * layout has left the screen: everything under it may read the signed-in user.
 */
export function useLogout() {
  const queryClient = useQueryClient();
  const router = useRouter();
  return useMutation({
    mutationFn: async () => {
      try {
        await api.auth.logout();
      } catch {
        // The token is dropped locally either way.
      }
    },
    onSettled: async () => {
      clearSession();
      await router.navigate({ to: "/login", replace: true });
      queryClient.clear();
    },
  });
}
