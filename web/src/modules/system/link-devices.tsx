import { Button, Spinner } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { useSecureSurface } from "@/skins/secure";
import { useRowInset } from "@/ui/section";
import { ErrorState } from "@/ui/states";

export function LinkDevices({ userId }: { userId: string }) {
  const { t } = useTranslation();
  const enabled = useFeature("link");
  const client = useQueryClient();
  const secure = useSecureSurface();
  const inset = useRowInset();
  const [selected, setSelected] = useState<string | null>(null);
  const key = ["linkDevices", userId] as const;
  const list = useQuery({
    queryKey: key,
    queryFn: ({ signal }) => request<components["schemas"]["LinkDeviceList"]>("/link/devices", { signal }),
    enabled,
    refetchInterval: 10_000,
  });
  const revoke = useMutation({
    mutationFn: (id: string) => request<void>(`/link/devices/${encodeURIComponent(id)}`, { method: "DELETE" }),
    onSuccess: () => {
      setSelected(null);
      return client.invalidateQueries({ queryKey: key });
    },
  });
  if (!enabled) return null;
  return (
    <li {...secure} className={cn("grid gap-2 py-3.5", inset)}>
      <span className="text-base text-fg">{t("linkDevices.title")}</span>
      {list.isPending ? <Spinner size="sm" /> : list.isError ? (
        <ErrorState compact error={list.error} onRetry={() => void list.refetch()} />
      ) : list.data.devices.length === 0 ? (
        <span className="text-sm text-fg-muted">{t("linkDevices.empty")}</span>
      ) : (
        <ul className="grid divide-y divide-line">
          {list.data.devices.map((device) => (
            <li key={device.id} className="flex flex-wrap items-center justify-between gap-2 py-2">
              <div className="min-w-0">
                <span className="break-words text-sm text-fg">{device.name}</span>
                <p className="text-xs text-fg-muted">
                  {t(device.connected ? "linkDevices.connected" : "linkDevices.disconnected")} · <code>{device.nodeId.slice(0, 12)}</code>
                </p>
              </div>
              {selected === device.id ? (
                <div className="grid gap-2">
                  <p className="text-sm">{t("linkDevices.confirm", { name: device.name })}</p>
                  <div className="flex gap-2">
                    <Button size="sm" variant="outline" isDisabled={revoke.isPending} onPress={() => setSelected(null)}>{t("common.cancel")}</Button>
                    <Button size="sm" variant="danger" isPending={revoke.isPending} onPress={() => revoke.mutate(device.id)}>{t("linkDevices.revoke")}</Button>
                  </div>
                  {revoke.isError && <p role="alert">{errorText(t, revoke.error)}</p>}
                </div>
              ) : (
                <Button size="sm" variant="outline" isDisabled={revoke.isPending} onPress={() => { revoke.reset(); setSelected(device.id); }}>{t("linkDevices.revoke")}</Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}
