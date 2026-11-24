import { RefreshCw } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { useFeature } from "@/modules/access";
import type { InboxItem } from "@/modules/types";
import { useReconnectMount } from "@/provisional/queries/catalog";
import { useStorage } from "@/provisional/queries/storage";

/** PROVISIONAL: storage that went offline (feature "storage"), as inbox alerts with a way back. */
export function useStorageAlerts(): InboxItem[] {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const { data } = useStorage(useFeature("storage"));
  const reconnect = useReconnectMount();
  const busy = reconnect.isPending ? reconnect.variables : null;
  return useMemo(
    () =>
      (data?.mounts ?? [])
        .filter((mount) => mount.state === "offline")
        .map(
          (mount): InboxItem => ({
            id: `mount:${mount.id}`,
            kind: "alert",
            title: t("storage.mountOffline", { name: mount.label }),
            detail: t("storage.mountOfflineDetail", { count: mount.affectedLibraries }),
            state: t("storage.offline"),
            createdAt: mount.since,
            tone: "warning",
            to: "/system/libraries",
            actions: isAdmin
              ? [{ id: "reconnect", label: t("storage.reconnect"), variant: "outline", icon: RefreshCw, run: () => reconnect.mutate(mount.id), busy: busy === mount.id }]
              : [],
          }),
        ),
    [data, t, isAdmin, reconnect, busy],
  );
}
