import { Button, Input } from "@heroui/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { useSecureSurface } from "@/skins/secure";
import { useRowInset } from "@/ui/section";

type OpdsAccess = components["schemas"]["OpdsAccess"];

export function OpdsReaderAccess() {
  const { t } = useTranslation();
  const available = useFeature("opds");
  const secure = useSecureSurface();
  const inset = useRowInset();
  const [access, setAccess] = useState<OpdsAccess | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  async function reveal() {
    setBusy(true);
    setError(null);
    try {
      setAccess(await request<OpdsAccess>("/me/opds-access"));
    } catch (cause) {
      setError(cause);
    } finally {
      setBusy(false);
    }
  }

  if (!available) return null;
  return (
    <li {...secure} className={cn("grid gap-2.5 py-3.5", inset)}>
      <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2">
        <span className="text-sm font-mid text-fg">{t("opdsReaders.title")}</span>
        {access ? (
          <Button size="sm" variant="outline" onPress={() => setAccess(null)}>
            {t("a11y.close")}
          </Button>
        ) : (
          <Button isPending={busy} size="sm" variant="outline" onPress={() => void reveal()}>
            {t("opdsReaders.show")}
          </Button>
        )}
      </div>
      {access ? (
        <div className="grid gap-2.5 rounded-tile border border-line bg-sunken/70 p-3.5">
          {[[t("musicClients.address"), new URL("/opds/", window.location.origin).href],
            [t("musicClients.username"), access.username], [t("opdsReaders.password"), access.password]].map(([label, value]) => (
            <label key={label} className="grid gap-1 text-xs font-mid text-fg-muted">{label}
              <Input readOnly value={value} aria-label={label} onFocus={(e) => e.currentTarget.select()} className="font-mono text-xs" />
            </label>
          ))}
          <p className="text-xs text-fg-muted">{t("opdsReaders.session")}</p>
        </div>
      ) : null}
      {error != null && <p role="alert" className="text-xs text-failed">{errorText(t, error)}</p>}
    </li>
  );
}
