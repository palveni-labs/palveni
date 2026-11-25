import { Button, Drawer, Input, Label, Spinner, TextField, toast } from "@heroui/react";
import { Check, Copy, Plus, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import {
  useAddAssistantMemory,
  useAssistantConfig,
  useAssistantMemories,
  useDeleteAssistantMemory,
  useMcpInfo,
  useUpdateAssistantConfig,
} from "@/provisional/queries/assistant";
import type { AssistantConfigResponse } from "@/provisional/types";

const SELECT_CLASS = "pv-select w-full";

type McpClientTab = "claudeDesktop" | "cursor" | "openclaw";

function ProviderForm({ config, isAdmin }: { config: AssistantConfigResponse; isAdmin: boolean }) {
  const { t } = useTranslation();
  const update = useUpdateAssistantConfig();
  const [preset, setPreset] = useState(config.preset);
  const [baseUrl, setBaseUrl] = useState(config.baseUrl);
  const [model, setModel] = useState(config.model);
  const [apiKey, setApiKey] = useState("");
  const [redactCloudPaths, setRedactCloudPaths] = useState(config.redactCloudPaths ?? true);

  const onPresetChange = (nextPreset: string) => {
    setPreset(nextPreset);
    const info = config.presets.find((entry) => entry.id === nextPreset);
    if (info) {
      setBaseUrl(info.defaultBaseUrl);
      setModel(info.defaultModel);
    }
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (!isAdmin) return;
    update.mutate(
      {
        preset,
        baseUrl: baseUrl.trim(),
        model: model.trim(),
        apiKey: apiKey.trim() || undefined,
        redactCloudPaths,
      },
      {
        onSuccess: () => {
          setApiKey("");
          toast.success(t("assistant.settings.saved"));
        },
        onError: (error) => toast.danger(errorText(t, error)),
      },
    );
  };

  const isBuiltin = preset === "builtin";

  return (
    <form onSubmit={onSubmit} className="grid gap-3">
      <div className="grid gap-1.5">
        <label htmlFor="assistant-preset" className="text-sm font-mid text-fg">
          {t("assistant.settings.preset")}
        </label>
        <select
          id="assistant-preset"
          value={preset}
          disabled={!isAdmin || update.isPending}
          onChange={(event) => onPresetChange(event.target.value)}
          className={SELECT_CLASS}
        >
          {config.presets.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.label}
            </option>
          ))}
        </select>
      </div>

      {!isBuiltin ? (
        <>
          <TextField value={baseUrl} onChange={setBaseUrl} isDisabled={!isAdmin || update.isPending} className="grid gap-1.5">
            <Label className="text-sm font-mid text-fg">{t("assistant.settings.baseUrl")}</Label>
            <Input placeholder="http://127.0.0.1:11434/v1" />
          </TextField>
          <TextField value={model} onChange={setModel} isDisabled={!isAdmin || update.isPending} className="grid gap-1.5">
            <Label className="text-sm font-mid text-fg">{t("assistant.settings.model")}</Label>
            <Input placeholder="qwen2.5:7b" />
          </TextField>
          <TextField value={apiKey} onChange={setApiKey} isDisabled={!isAdmin || update.isPending} className="grid gap-1.5">
            <Label className="text-sm font-mid text-fg">{t("assistant.settings.apiKey")}</Label>
            <Input
              type="password"
              autoComplete="off"
              placeholder={
                config.apiKeyConfigured
                  ? t("assistant.settings.apiKeySavedPlaceholder")
                  : t("assistant.settings.apiKeyPlaceholder")
              }
            />
          </TextField>
        </>
      ) : null}

      <label className="flex cursor-pointer items-start gap-2.5 rounded-tile border border-line bg-surface px-3.5 py-3 text-sm">
        <input
          type="checkbox"
          checked={redactCloudPaths}
          disabled={!isAdmin || update.isPending}
          onChange={(event) => setRedactCloudPaths(event.target.checked)}
          className="pv-checkbox mt-0.5"
        />
        <span className="grid min-w-0 gap-0.5">
          <span className="font-mid text-fg">{t("assistant.settings.redactCloudPaths")}</span>
          <span className="text-xs text-fg-muted">{t("assistant.settings.redactCloudPathsHint")}</span>
        </span>
      </label>

      {isAdmin ? (
        <div className="flex justify-end pt-1">
          <Button type="submit" size="sm" variant="primary" isDisabled={update.isPending}>
            {update.isPending ? <Spinner size="sm" color="current" /> : null}
            {t("assistant.settings.saveProvider")}
          </Button>
        </div>
      ) : (
        <p className="text-xs text-fg-muted">{t("assistant.settings.adminOnlyNote")}</p>
      )}
    </form>
  );
}

function McpAccessCard({ enabled }: { enabled: boolean }) {
  const { t } = useTranslation();
  const mcpQuery = useMcpInfo(enabled);
  const [clientTab, setClientTab] = useState<McpClientTab>("cursor");
  const [copied, setCopied] = useState(false);

  if (mcpQuery.isPending) {
    return (
      <div className="py-3">
        <Spinner size="sm" />
      </div>
    );
  }

  const info = mcpQuery.data;
  if (!info) return null;

  const snippet = info.clientConfigs[clientTab];

  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(snippet);
    } catch {
      // Clipboard API can be unavailable in restricted contexts; ignore silently and still confirm UI action.
    }
    setCopied(true);
    toast.success(t("assistant.settings.mcpCopied"));
    window.setTimeout(() => setCopied(false), 1800);
  };

  const tabs: Array<{ id: McpClientTab; label: string }> = [
    { id: "claudeDesktop", label: t("assistant.settings.mcpClientClaude") },
    { id: "cursor", label: t("assistant.settings.mcpClientCursor") },
    { id: "openclaw", label: t("assistant.settings.mcpClientOpenclaw") },
  ];

  return (
    <div className="grid gap-3 rounded-tile border border-line bg-surface p-3.5">
      <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-fg-muted">
        <span className="font-mono text-fg">{info.endpointPath}</span>
        <span>
          MCP {info.protocolVersion} · {t("assistant.settings.mcpToolsCount", { count: info.tools.length })}
        </span>
      </div>

      <div
        role="tablist"
        aria-label={t("assistant.settings.mcpSection")}
        className="inline-flex w-fit items-center gap-0.5 rounded-control border border-line bg-sunken p-0.5"
      >
        {tabs.map((tab) => {
          const active = clientTab === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => setClientTab(tab.id)}
              className={cn(
                "h-7 rounded-[calc(var(--pv-radius-control)-2px)] px-2.5 text-xs transition-all",
                active
                  ? "bg-raised font-strong text-heading shadow-panel"
                  : "font-mid text-fg-muted hover:text-fg",
              )}
            >
              {tab.label}
            </button>
          );
        })}
      </div>

      <pre className="max-h-48 overflow-auto rounded-control border border-line bg-sunken p-3 font-mono text-[11px] leading-relaxed text-fg">
        {snippet}
      </pre>

      <div className="flex justify-end">
        <Button size="sm" variant="outline" onPress={() => void onCopy()}>
          {copied ? <Check aria-hidden /> : <Copy aria-hidden />}
          {t("assistant.settings.mcpCopy")}
        </Button>
      </div>
    </div>
  );
}

export function AssistantSettingsDrawer({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const hasMcp = useFeature("mcp");
  const configQuery = useAssistantConfig(open);
  const memoriesQuery = useAssistantMemories(open);
  const addMemory = useAddAssistantMemory();
  const deleteMemory = useDeleteAssistantMemory();
  const [memoryDraft, setMemoryDraft] = useState("");

  const onAddMemory = (event: FormEvent) => {
    event.preventDefault();
    const content = memoryDraft.trim();
    if (!content) return;
    addMemory.mutate(content, {
      onSuccess: () => {
        setMemoryDraft("");
        toast.success(t("assistant.settings.memoryAdded"));
      },
      onError: (error) => toast.danger(errorText(t, error)),
    });
  };

  const config = configQuery.data;
  const memories = memoriesQuery.data?.memories ?? [];

  return (
    <Drawer.Backdrop isOpen={open} onOpenChange={onOpenChange}>
      <Drawer.Content placement="right">
        <Drawer.Dialog
          aria-label={t("assistant.settings.title")}
          className="flex h-full w-[min(31rem,94vw)] max-w-none flex-col gap-0 overflow-hidden border-s border-line p-0"
        >
          <Drawer.Header className="flex flex-row items-center justify-between gap-3 border-b border-line px-6 py-4">
            <Drawer.Heading className="font-heading text-lg font-title text-heading">
              {t("assistant.settings.title")}
            </Drawer.Heading>
            <Drawer.CloseTrigger aria-label={t("a11y.close")} className="static shrink-0" />
          </Drawer.Header>
          <Drawer.Body className="m-0 grid content-start gap-6 overflow-y-auto px-6 py-5">
            <section className="grid gap-3">
              <h2 className="text-sm font-strong text-heading">{t("assistant.settings.providerSection")}</h2>
              {config ? (
                <ProviderForm
                  key={`${config.preset}:${config.baseUrl}:${config.model}:${String(config.apiKeyConfigured)}:${String(config.redactCloudPaths)}`}
                  config={config}
                  isAdmin={isAdmin}
                />
              ) : (
                <div className="py-4">
                  <Spinner size="sm" />
                </div>
              )}
            </section>

            {hasMcp ? (
              <section className="grid gap-3 border-t border-line pt-5">
                <h2 className="text-sm font-strong text-heading">{t("assistant.settings.mcpSection")}</h2>
                <McpAccessCard enabled={open && hasMcp} />
              </section>
            ) : null}

            <section className="grid gap-3 border-t border-line pt-5">
              <h2 className="text-sm font-strong text-heading">{t("assistant.settings.memorySection")}</h2>
              <form onSubmit={onAddMemory} className="flex items-center gap-2">
                <Input
                  aria-label={t("assistant.settings.memorySection")}
                  value={memoryDraft}
                  onChange={(event) => setMemoryDraft(event.target.value)}
                  placeholder={t("assistant.settings.memoryPlaceholder")}
                  className="min-w-0 flex-1"
                />
                <Button
                  type="submit"
                  variant="secondary"
                  isDisabled={!memoryDraft.trim() || addMemory.isPending}
                >
                  <Plus aria-hidden />
                  {t("assistant.settings.addMemory")}
                </Button>
              </form>

              {memories.length === 0 ? (
                <p className="text-sm text-fg-muted">{t("assistant.settings.memoryEmpty")}</p>
              ) : (
                <ul className="divide-y divide-line rounded-tile border border-line bg-surface">
                  {memories.map((entry) => (
                    <li key={entry.id} className="flex items-center gap-2 px-3.5 py-2.5">
                      <span className="min-w-0 flex-1 break-words text-sm text-fg">{entry.content}</span>
                      <Button
                        size="sm"
                        variant="ghost"
                        isIconOnly
                        aria-label={t("assistant.settings.deleteMemory")}
                        isDisabled={deleteMemory.isPending}
                        onPress={() =>
                          deleteMemory.mutate(entry.id, {
                            onError: (error) => toast.danger(errorText(t, error)),
                          })
                        }
                      >
                        <Trash2 className="size-4" aria-hidden />
                      </Button>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </Drawer.Body>
        </Drawer.Dialog>
      </Drawer.Content>
    </Drawer.Backdrop>
  );
}
