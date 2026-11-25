import { Button, Input, Spinner } from "@heroui/react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import type { SubsonicPassword, User } from "@/api/types";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useSecureSurface } from "@/skins/secure";
import { useRowInset } from "@/ui/section";
import { ErrorState } from "@/ui/states";

export function MusicClientAccess({ user }: { user: User }) {
  const { t } = useTranslation();
  const secure = useSecureSurface();
  const inset = useRowInset();
  const queryClient = useQueryClient();
  const key = ["musicClientAccess", user.id] as const;
  const status = useQuery({ queryKey: key, queryFn: ({ signal }) => api.subsonicPassword.status(signal) });
  const [issued, setIssued] = useState<SubsonicPassword | null>(null);
  const [confirm, setConfirm] = useState<"replace" | "revoke" | null>(null);
  const [personal, setPersonal] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  async function apply(action: "replace" | "revoke") {
    setBusy(true);
    setError(null);
    try {
      if (action === "replace") {
        const password = await api.subsonicPassword.issue(personal);
        setIssued(password);
      } else {
        await api.subsonicPassword.revoke();
        setIssued(null);
      }
      queryClient.setQueryData(key, { enabled: action === "replace", personal: action === "replace" && personal });
      setConfirm(null);
    } catch (cause) {
      setError(cause);
    } finally {
      setBusy(false);
    }
  }

  const permission = (
    <label className="flex cursor-pointer items-center gap-2.5 text-sm text-fg">
      <input
        type="checkbox"
        className="pv-checkbox"
        checked={personal}
        disabled={busy}
        onChange={(event) => setPersonal(event.target.checked)}
      />
      <span>{t("musicClients.grantPersonal")}</span>
    </label>
  );
  return (
    <li {...secure} className={cn("grid gap-2.5 py-3.5", inset)}>
      <div className="flex flex-wrap items-start justify-between gap-x-6 gap-y-2">
        <div className="grid min-w-0 gap-0.5">
          <span className="text-sm font-mid text-fg">{t("musicClients.title")}</span>
          <span className="text-xs text-fg-muted">{t("musicClients.hint")}</span>
        </div>
        {!status.isPending && !status.isError && !issued && !confirm ? (
          <div className="flex shrink-0 flex-wrap items-center gap-2">
            <Button
              size="sm"
              variant="outline"
              isPending={busy}
              onPress={() => {
                if (status.data.enabled) {
                  setPersonal(status.data.personal);
                  setConfirm("replace");
                } else {
                  void apply("replace");
                }
              }}
            >
              {t(status.data.enabled ? "musicClients.replace" : "musicClients.create")}
            </Button>
            {status.data.enabled ? (
              <Button size="sm" variant="outline" onPress={() => setConfirm("revoke")}>
                {t("musicClients.revoke")}
              </Button>
            ) : null}
          </div>
        ) : null}
      </div>
      {status.isPending ? (
        <Spinner size="sm" />
      ) : status.isError ? (
        <ErrorState compact error={status.error} onRetry={() => void status.refetch()} />
      ) : issued ? (
        <div className="grid gap-2.5 rounded-tile border border-line bg-sunken/70 p-3.5">
          <p role="status" className="text-sm font-mid text-fg">{t("musicClients.shown")}</p>
          {[[t("musicClients.address"), window.location.origin], [t("musicClients.username"), issued.username],
            [t("musicClients.password"), issued.password]].map(([label, value]) => (
            <label key={label} className="grid gap-1 text-xs font-mid text-fg-muted">{label}
              <Input readOnly value={value} aria-label={label} onFocus={(e) => e.currentTarget.select()} className="font-mono text-xs" />
            </label>
          ))}
          <Button size="sm" variant="outline" className="w-fit" onPress={() => setIssued(null)}>{t("a11y.close")}</Button>
        </div>
      ) : confirm ? (
        <div className="grid gap-2.5 rounded-tile border border-line bg-sunken/70 p-3.5">
          <p className="text-sm text-fg">{t(confirm === "revoke" ? "musicClients.confirmRevoke" : "musicClients.confirmReplace")}</p>
          {confirm === "replace" && permission}
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="outline" isDisabled={busy} onPress={() => setConfirm(null)}>{t("common.cancel")}</Button>
            <Button size="sm" variant="primary" isPending={busy} onPress={() => void apply(confirm)}>{t(confirm === "revoke" ? "musicClients.revoke" : "musicClients.replace")}</Button>
          </div>
        </div>
      ) : status.data.enabled ? (
        <span className="text-xs text-fg-muted">{t(status.data.personal ? "musicClients.personalEnabled" : "musicClients.browseOnly")}</span>
      ) : (
        permission
      )}
      {error != null && <p role="alert" className="text-xs text-failed">{errorText(t, error)}</p>}
    </li>
  );
}
