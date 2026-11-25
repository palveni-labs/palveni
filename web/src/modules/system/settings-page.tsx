import { Button, buttonVariants, Chip, Input, Spinner, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { ChevronDown, LogOut, Palette, Pencil } from "lucide-react";
import { useState, type ReactNode } from "react";
import type { PerfTier, RuntimeSettings } from "@/api/types";
import { errorText } from "@/lib/errors";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useCurrentUser, useIsAdmin, useLogout } from "@/api/queries/auth";
import { useRuntimeSettings, useSystemInfo, useUpdateRuntimeSettings } from "@/api/queries/system";
import { currentLanguage, SUPPORTED_LANGUAGES } from "@/i18n";
import { BUILTIN_SKINS } from "@/skins/registry";
import { localized } from "@/skins/schema";
import { useSkin } from "@/skins/store";
import { UserAvatar } from "@/shell/user-menu";
import { ModeSetting } from "@/ui/mode-setting";
import { NavCustomizer } from "@/ui/nav-customizer";
import { NavSetting } from "@/ui/nav-setting";
import { PageFrame } from "@/ui/frame";
import { LinkDevices } from "./link-devices";
import { OpdsReaderAccess } from "./opds-reader-access";
import { MusicClientAccess } from "./music-client-access";
import { AccountSecurity } from "./account-security";
import { ApiKeysCard } from "./api-keys-card";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { ErrorState, LoadingRows } from "@/ui/states";

/** One setting: what it is on the left, its control (or value) on the right; stacked on phones. */
function SettingRow({ label, hint, children, className }: { label: ReactNode; hint?: ReactNode; children: ReactNode; className?: string }) {
  const inset = useRowInset();
  return (
    <li className={cn("flex flex-wrap items-center justify-between gap-x-6 gap-y-2.5 py-3.5", inset, className)}>
      <div className="grid min-w-0 gap-0.5">
        <span className="text-sm font-mid text-fg">{label}</span>
        {hint ? <span className="text-xs text-fg-muted">{hint}</span> : null}
      </div>
      <div className="flex min-w-0 items-center gap-2">{children}</div>
    </li>
  );
}

function Display() {
  const { t, i18n } = useTranslation();
  const { entry, setSkin } = useSkin();
  const language = currentLanguage();
  const resolvedLang = i18n.resolvedLanguage ?? "en";
  const activeBuiltin = BUILTIN_SKINS.some((skin) => skin.manifest.id === entry.manifest.id);
  return (
    <Section title={t("settings.display")}>
      <SectionRows>
        <SettingRow label={t("settings.mode")}>
          <ModeSetting className="max-w-full" />
        </SettingRow>
        <SettingRow label={t("settings.skin")}>
          <div className="flex flex-wrap items-center justify-end gap-2">
            <ToggleButtonGroup
              aria-label={t("settings.skin")}
              selectionMode="single"
              disallowEmptySelection
              selectedKeys={activeBuiltin ? [entry.manifest.id] : []}
              onSelectionChange={(keys) => {
                const next = [...keys][0];
                if (typeof next === "string") void setSkin(next);
              }}
              className="pv-segmented max-w-full flex-wrap"
            >
              {BUILTIN_SKINS.map((skin) => (
                <ToggleButton key={skin.manifest.id} id={skin.manifest.id}>
                  {localized(skin.manifest.name, resolvedLang)}
                </ToggleButton>
              ))}
            </ToggleButtonGroup>
            <Link to="/apps/themes" className={buttonVariants({ variant: "outline" })}>
              <Palette aria-hidden />
              {t("settings.changeSkin")}
            </Link>
          </div>
        </SettingRow>
        <SettingRow label={t("language.label")}>
          <ToggleButtonGroup
            aria-label={t("language.label")}
            selectionMode="single"
            disallowEmptySelection
            selectedKeys={[language]}
            onSelectionChange={(keys) => {
              const next = [...keys][0];
              if (typeof next === "string") void i18n.changeLanguage(next);
            }}
            className="pv-segmented"
          >
            {SUPPORTED_LANGUAGES.map((code) => (
              <ToggleButton key={code} id={code} lang={code}>
                {t(`languages.${code}`)}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </SettingRow>
      </SectionRows>
    </Section>
  );
}

function Account() {
  const { t } = useTranslation();
  const user = useCurrentUser();
  const logout = useLogout();
  return (
    <Section title={t("settings.account")}>
      <SectionRows>
        <SettingRow
          label={
            <span className="flex items-center gap-3">
              <UserAvatar name={user.username} className="bg-sunken text-fg" />
              <span className="grid">
                <span className="font-strong text-fg">{user.username}</span>
                <span className="text-xs font-body text-fg-muted">{t(`user.roles.${user.role}`)}</span>
              </span>
            </span>
          }
        >
          <Button
            variant="outline"
            size="sm"
            isPending={logout.isPending}
            onPress={() => logout.mutate()}
          >
            {logout.isPending ? <Spinner size="sm" color="current" /> : <LogOut aria-hidden />}
            {t("user.logout")}
          </Button>
        </SettingRow>
        <MusicClientAccess key={user.id} user={user} />
        <OpdsReaderAccess key={`opds-${user.id}`} />
        <LinkDevices key={`link-${user.id}`} userId={user.id} />
      </SectionRows>
    </Section>
  );
}

const TIERS: PerfTier[] = ["tiny", "lite", "standard", "performance"];

function ServerInfo() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const query = useSystemInfo();
  const info = query.data;
  const canEdit = Boolean(isAdmin && info?.features.includes("settings"));
  const runtimeQuery = useRuntimeSettings(canEdit);
  const update = useUpdateRuntimeSettings();
  const [draft, setDraft] = useState<RuntimeSettings | null>(null);
  const inset = useRowInset();

  const runtimeData = runtimeQuery.data;
  const headerActions = canEdit && runtimeData ? (
    draft ? (
      <div className="flex items-center gap-2">
        <Button size="sm" variant="outline" isDisabled={update.isPending} onPress={() => setDraft(null)}>
          {t("common.cancel")}
        </Button>
        <Button
          size="sm"
          variant="primary"
          isPending={update.isPending}
          onPress={() =>
            update.mutate(
              { ...draft, serverName: draft.serverName?.trim() || null },
              { onSuccess: () => setDraft(null) },
            )
          }
        >
          {t("settings.save")}
        </Button>
      </div>
    ) : (
      <Button
        size="sm"
        variant="outline"
        onPress={() => {
          update.reset();
          setDraft({ ...runtimeData });
        }}
      >
        <Pencil aria-hidden />
        {t("settings.editServer")}
      </Button>
    )
  ) : undefined;

  return (
    <Section title={t("settings.server")} actions={headerActions}>
      {query.isPending ? (
        <LoadingRows rows={2} className="px-(--pv-pad-panel) pb-4" />
      ) : !info ? (
        <ErrorState compact error={query.error} onRetry={() => void query.refetch()} />
      ) : (
        <SectionRows>
          {canEdit && runtimeQuery.isPending ? (
            <li className={cn("py-3", inset)}>
              <LoadingRows rows={1} />
            </li>
          ) : canEdit && !runtimeQuery.data ? (
            <li className={cn("py-3", inset)}>
              <ErrorState compact error={runtimeQuery.error} onRetry={() => void runtimeQuery.refetch()} />
            </li>
          ) : (
            <>
              <SettingRow label={t("settings.serverName")}>
                {draft ? (
                  <Input
                    aria-label={t("settings.serverName")}
                    placeholder={t("settings.defaultName")}
                    maxLength={64}
                    value={draft.serverName ?? ""}
                    onChange={(event) => setDraft({ ...draft, serverName: event.target.value || null })}
                    className="w-52 max-w-full"
                  />
                ) : (
                  <span className="text-sm text-fg">{info.name}</span>
                )}
              </SettingRow>
              {canEdit && runtimeQuery.data ? (
                <SettingRow label={t("settings.perfTier")}>
                  {draft ? (
                    <select
                      aria-label={t("settings.perfTier")}
                      className="pv-select"
                      value={draft.perfTier ?? ""}
                      onChange={(event) =>
                        setDraft({ ...draft, perfTier: TIERS.find((tier) => tier === event.target.value) ?? null })
                      }
                    >
                      <option value="">{t("settings.automatic")}</option>
                      {TIERS.map((tier) => (
                        <option key={tier} value={tier}>
                          {t(`hardware.tiers.${tier}.name`)}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <span className="text-sm text-fg">
                      {runtimeQuery.data.perfTier
                        ? t(`hardware.tiers.${runtimeQuery.data.perfTier}.name`)
                        : t("settings.automatic")}
                    </span>
                  )}
                </SettingRow>
              ) : null}
              {draft && update.isError ? (
                <li className={cn("py-2.5", inset)}>
                  <span role="alert" className="text-sm text-failed">
                    {errorText(t, update.error)}
                  </span>
                </li>
              ) : null}
            </>
          )}
          <SettingRow label={t("settings.version")}>
            <span data-num className="font-mono text-sm text-fg tabular-nums">
              {info.version}
            </span>
          </SettingRow>
          {isAdmin ? (
            <SettingRow label={t("settings.serverId")}>
              <span className="font-mono text-xs break-all text-fg-muted select-all">{info.serverId}</span>
            </SettingRow>
          ) : null}
          {isAdmin && info.features.length > 0 ? (
            <li className={cn("py-3.5", inset)}>
              <details className="group">
                <summary className="flex cursor-pointer list-none items-center justify-between gap-4 rounded-control py-0.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-focus [&::-webkit-details-marker]:hidden">
                  <span className="font-mid text-fg">{t("settings.features")}</span>
                  <span className="inline-flex items-center gap-1.5 text-xs font-mid text-fg-muted">
                    <span data-num className="inline-flex h-6 items-center rounded-chip bg-sunken px-2 font-mono text-fg tabular-nums">
                      {info.features.length}
                    </span>
                    <ChevronDown className="size-3.5 transition-transform duration-150 group-open:rotate-180" aria-hidden />
                  </span>
                </summary>
                <ul className="mt-3 flex flex-wrap gap-1.5">
                  {info.features.map((feature) => (
                    <li key={feature}>
                      <Chip size="sm" variant="soft" className="bg-sunken font-mono text-[0.75rem] text-fg-muted">
                        {feature}
                      </Chip>
                    </li>
                  ))}
                </ul>
              </details>
            </li>
          ) : null}
        </SectionRows>
      )}
    </Section>
  );
}

function NavigationSettings() {
  const { t } = useTranslation();
  const inset = useRowInset();
  return (
    <Section title={t("nav.customizeSection")}>
      <SectionRows>
        <SettingRow label={t("nav.layout")}>
          <NavSetting className="max-w-full" />
        </SettingRow>
        <li className={cn("py-3.5", inset)}>
          <NavCustomizer />
        </li>
      </SectionRows>
    </Section>
  );
}

/** 系统 › 设置: appearance, language, the signed-in account and the server's identity. */
export function SettingsPage() {
  const { t } = useTranslation();
  return (
    <PageFrame>
      <PageHeader title={t("nav.settings")} />
      <div className="grid items-start gap-(--pv-gap-section) lg:grid-cols-2">
        <div className="grid min-w-0 gap-(--pv-gap-section)">
          <Display />
          <NavigationSettings />
        </div>
        <div className="grid min-w-0 gap-(--pv-gap-section)">
          <ServerInfo />
          <Account />
          <ApiKeysCard />
          <AccountSecurity />
        </div>
      </div>
    </PageFrame>
  );
}
