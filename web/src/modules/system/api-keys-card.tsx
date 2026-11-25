import { Button, Chip, Input, Spinner, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Copy, KeyRound, Plus, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { request } from "@/api/client";
import type { components } from "@/api/generated";
import { useCurrentUser } from "@/api/queries/auth";
import { RelativeTime } from "@/components/relative-time";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { useSecureSurface } from "@/skins/secure";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { ErrorState, LoadingRows } from "@/ui/states";

type ClientApiKeyKind = components["schemas"]["ClientApiKeyKind"];
type ClientApiKeyList = components["schemas"]["ClientApiKeyList"];
type CreatedClientApiKey = components["schemas"]["CreatedClientApiKey"];
type CreateClientApiKeyRequest = components["schemas"]["CreateClientApiKeyRequest"];

const CLIENT_KINDS: readonly ClientApiKeyKind[] = ["jellyfin", "webdav", "general"];

function apiKeysQueryKey(userId: string) {
  return ["system", "apiKeys", userId] as const;
}

async function copyText(text: string): Promise<boolean> {
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      return false;
    }
  }
  return false;
}

/** Jellyfin-compatible server URL and personal client API keys management card. */
export function ApiKeysCard() {
  const { t } = useTranslation();
  const available = useFeature("jellyfin");
  const user = useCurrentUser();
  const secure = useSecureSurface();
  const inset = useRowInset();
  const queryClient = useQueryClient();

  const [name, setName] = useState("");
  const [clientKind, setClientKind] = useState<ClientApiKeyKind>("jellyfin");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<CreatedClientApiKey | null>(null);
  const [copiedId, setCopiedId] = useState<"url" | "token" | null>(null);

  const serverUrl = typeof window !== "undefined" ? window.location.origin : "";

  const keysQuery = useQuery({
    queryKey: apiKeysQueryKey(user.id),
    queryFn: ({ signal }) => request<ClientApiKeyList>("/me/api-keys", { signal }),
    enabled: available,
    staleTime: 15_000,
  });

  const createKey = useMutation({
    mutationFn: (body: CreateClientApiKeyRequest) =>
      request<CreatedClientApiKey>("/me/api-keys", { method: "POST", body }),
    onSuccess: (created) => {
      setRevealed(created);
      setName("");
      setValidationError(null);
      void queryClient.invalidateQueries({ queryKey: apiKeysQueryKey(user.id) });
    },
  });

  const revokeKey = useMutation({
    mutationFn: (id: string) => request<void>(`/me/api-keys/${encodeURIComponent(id)}`, { method: "DELETE" }),
    onSuccess: (_data, revokedId) => {
      if (revealed?.key.id === revokedId) setRevealed(null);
      void queryClient.invalidateQueries({ queryKey: apiKeysQueryKey(user.id) });
      toast.success(t("apiKeys.revoked"));
    },
    onError: (err) => toast.danger(errorText(t, err)),
  });

  if (!available) return null;

  const handleCopy = async (target: "url" | "token", value: string) => {
    const ok = await copyText(value);
    if (ok) {
      setCopiedId(target);
      window.setTimeout(() => setCopiedId((curr) => (curr === target ? null : curr)), 2000);
    }
  };

  const handleCreate = (event: FormEvent) => {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) {
      setValidationError(t("apiKeys.nameRequired"));
      return;
    }
    setValidationError(null);
    createKey.mutate({ name: trimmed, clientKind });
  };

  const keys = keysQuery.data?.keys ?? [];

  return (
    <Section title={t("apiKeys.title")}>
      <div {...secure}>
        <SectionRows>
          {/* Jellyfin Compatible Server URL & client setup hint */}
          <li className={cn("grid gap-2 py-3.5", inset)}>
            <div className="grid gap-1">
              <span className="text-sm font-mid text-fg">{t("apiKeys.jellyfinUrl")}</span>
              <div className="flex flex-wrap items-center gap-2">
                <Input
                  readOnly
                  value={serverUrl}
                  aria-label={t("apiKeys.jellyfinUrl")}
                  onFocus={(e) => e.currentTarget.select()}
                  className="min-w-0 flex-1 font-mono text-sm"
                />
                <Button
                  variant="outline"
                  onPress={() => void handleCopy("url", serverUrl)}
                >
                  {copiedId === "url" ? <Check aria-hidden /> : <Copy aria-hidden />}
                  {copiedId === "url" ? t("apiKeys.copied") : t("apiKeys.copy")}
                </Button>
              </div>
              <p className="text-xs text-fg-muted">{t("apiKeys.jellyfinHint")}</p>
            </div>
          </li>

          {/* Create API key form */}
          <li className={cn("grid gap-3 py-3.5", inset)}>
            <form onSubmit={handleCreate} className="grid gap-2.5">
              <div className="flex flex-wrap items-end gap-2">
                <label className="grid min-w-44 flex-1 gap-1 text-xs font-mid text-fg-muted">
                  <span>{t("apiKeys.nameLabel")}</span>
                  <Input
                    value={name}
                    maxLength={128}
                    aria-label={t("apiKeys.nameLabel")}
                    placeholder={t("apiKeys.namePlaceholder")}
                    onChange={(e) => {
                      setName(e.target.value);
                      if (validationError) setValidationError(null);
                    }}
                  />
                </label>

                <label className="grid gap-1 text-xs font-mid text-fg-muted">
                  <span>{t("apiKeys.kindLabel")}</span>
                  <select
                    aria-label={t("apiKeys.kindLabel")}
                    value={clientKind}
                    onChange={(e) => setClientKind(e.target.value as ClientApiKeyKind)}
                    className="pv-select"
                  >
                    {CLIENT_KINDS.map((kind) => (
                      <option key={kind} value={kind}>
                        {t(`apiKeys.kinds.${kind}`)}
                      </option>
                    ))}
                  </select>
                </label>

                <Button type="submit" variant="primary" isPending={createKey.isPending}>
                  {createKey.isPending ? <Spinner size="sm" color="current" /> : <Plus aria-hidden />}
                  {t("apiKeys.create")}
                </Button>
              </div>

              {validationError ? (
                <p role="alert" className="text-xs text-failed">
                  {validationError}
                </p>
              ) : null}
              {createKey.isError ? (
                <p role="alert" className="text-xs text-failed">
                  {errorText(t, createKey.error)}
                </p>
              ) : null}
            </form>

            {/* One-time generated token banner */}
            {revealed ? (
              <div className="grid gap-2 rounded-control border border-line bg-sunken p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="flex items-center gap-1.5 text-sm font-strong text-fg">
                    <KeyRound className="size-4" aria-hidden />
                    {revealed.key.name}
                  </span>
                  <Button size="sm" variant="outline" onPress={() => setRevealed(null)}>
                    {t("apiKeys.dismiss")}
                  </Button>
                </div>
                <p className="text-xs text-fg-muted">{t("apiKeys.createdNotice")}</p>
                <div className="flex flex-wrap items-center gap-2">
                  <Input
                    readOnly
                    value={revealed.token}
                    aria-label={t("apiKeys.createdTokenLabel")}
                    onFocus={(e) => e.currentTarget.select()}
                    className="min-w-0 flex-1 font-mono text-xs"
                  />
                  <Button
                    size="sm"
                    variant="outline"
                    onPress={() => void handleCopy("token", revealed.token)}
                  >
                    {copiedId === "token" ? <Check className="size-3.5" aria-hidden /> : <Copy className="size-3.5" aria-hidden />}
                    {copiedId === "token" ? t("apiKeys.copied") : t("apiKeys.copy")}
                  </Button>
                </div>
              </div>
            ) : null}
          </li>

          {/* Active API keys list */}
          <li className={cn("grid gap-2 py-3.5", inset)}>
            <span className="text-sm font-mid text-fg">{t("apiKeys.activeKeys")}</span>
            {keysQuery.isPending ? (
              <LoadingRows rows={2} />
            ) : keysQuery.isError ? (
              <ErrorState compact error={keysQuery.error} onRetry={() => void keysQuery.refetch()} />
            ) : keys.length === 0 ? (
              <p className="text-sm text-fg-muted">{t("apiKeys.empty")}</p>
            ) : (
              <ul className="grid gap-2">
                {keys.map((item) => (
                  <li
                    key={item.id}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-control border border-line bg-surface px-3 py-2.5"
                  >
                    <div className="grid min-w-0 gap-0.5">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="truncate text-sm font-strong text-fg">{item.name}</span>
                        <Chip size="sm" variant="soft" className="bg-sunken text-fg">
                          {t(`apiKeys.kinds.${item.clientKind}`)}
                        </Chip>
                      </div>
                      <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-fg-muted">
                        <span className="font-mono">
                          {t("apiKeys.prefix")}: {item.keyPrefix}…
                        </span>
                        <span>
                          {t("apiKeys.lastUsed")}:{" "}
                          {item.lastUsedAt ? <RelativeTime value={item.lastUsedAt} /> : t("apiKeys.neverUsed")}
                        </span>
                      </div>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      isPending={revokeKey.isPending && revokeKey.variables === item.id}
                      onPress={() => revokeKey.mutate(item.id)}
                      aria-label={`${t("apiKeys.revoke")} ${item.name}`}
                    >
                      <Trash2 className="size-3.5" aria-hidden />
                      {t("apiKeys.revoke")}
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </li>
        </SectionRows>
      </div>
    </Section>
  );
}
