import { Button, Input, Spinner } from "@heroui/react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import type { SubsonicPassword, User } from "@/api/types";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useRowInset } from "@/ui/section";
import { ErrorState } from "@/ui/states";

export function MusicClientAccess({ user }: { user: User }) {
  const { t } = useTranslation();
  const inset = useRowInset();
  const queryClient = useQueryClient();
  const key = ["musicClientAccess", user.id] as const;
  const status = useQuery({ queryKey: key, queryFn: ({ signal }) => api.subsonicPassword.status(signal) });
  const [issued, setIssued] = useState<SubsonicPassword | null>(null);
  const [confirm, setConfirm] = useState<"replace" | "revoke" | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  async function apply(action: "replace" | "revoke") {
    setBusy(true);
    setError(null);
    try {
      if (action === "replace") {
        const password = await api.subsonicPassword.issue();
        setIssued(password);
      } else {
        await api.subsonicPassword.revoke();
        setIssued(null);
      }
      queryClient.setQueryData(key, { enabled: action === "replace" });
      setConfirm(null);
    } catch (cause) {
      setError(cause);
    } finally {
      setBusy(false);
    }
  }

  return (
    <li className={cn("grid gap-2 py-3.5", inset)}>
      <span className="text-base text-fg">{t("musicClients.title")}</span>
      <span className="text-sm text-fg-muted">{t("musicClients.hint")}</span>
      {status.isPending ? <Spinner size="sm" /> : status.isError ? (
        <ErrorState compact error={status.error} onRetry={() => void status.refetch()} />
      ) : issued ? (
        <div className="grid gap-2">
          <p role="status">{t("musicClients.shown")}</p>
          {[[t("musicClients.address"), window.location.origin], [t("musicClients.username"), issued.username],
            [t("musicClients.password"), issued.password]].map(([label, value]) => (
            <label key={label} className="grid gap-1 text-sm text-fg-muted">{label}
              <Input readOnly value={value} aria-label={label} onFocus={(e) => e.currentTarget.select()} />
            </label>
          ))}
          <Button size="sm" variant="outline" className="w-fit" onPress={() => setIssued(null)}>{t("a11y.close")}</Button>
        </div>
      ) : confirm ? (
        <div className="grid gap-2">
          <p>{t(confirm === "revoke" ? "musicClients.confirmRevoke" : "musicClients.confirmReplace")}</p>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="outline" isDisabled={busy} onPress={() => setConfirm(null)}>{t("common.cancel")}</Button>
            <Button size="sm" isPending={busy} onPress={() => void apply(confirm)}>{t(confirm === "revoke" ? "musicClients.revoke" : "musicClients.replace")}</Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="outline" isPending={busy}
            onPress={() => status.data.enabled ? setConfirm("replace") : void apply("replace")}>
            {t(status.data.enabled ? "musicClients.replace" : "musicClients.create")}
          </Button>
          {status.data.enabled && <Button size="sm" variant="outline" onPress={() => setConfirm("revoke")}>{t("musicClients.revoke")}</Button>}
        </div>
      )}
      {error != null && <p role="alert">{errorText(t, error)}</p>}
    </li>
  );
}
