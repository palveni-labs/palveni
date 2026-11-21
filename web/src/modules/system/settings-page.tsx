import { Button, buttonVariants, Chip, Input, Spinner, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { LogOut, Monitor, Moon, Palette, Sun } from "lucide-react";
import { useState, type ReactNode } from "react";
import type { PerfTier, RuntimeSettings } from "@/api/types";
import { errorText } from "@/lib/errors";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useCurrentUser, useIsAdmin, useLogout } from "@/api/queries/auth";
import { useRuntimeSettings, useSystemInfo, useUpdateRuntimeSettings } from "@/api/queries/system";
import { currentLanguage, SUPPORTED_LANGUAGES } from "@/i18n";
import { localized } from "@/skins/schema";
import { useSkin, type ModeSetting } from "@/skins/store";
import { UserAvatar } from "@/shell/user-menu";
import { PageFrame } from "@/ui/frame";
import { OpdsReaderAccess } from "./opds-reader-access";
import { MusicClientAccess } from "./music-client-access";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { ErrorState, LoadingRows } from "@/ui/states";

const MODES: Array<{ value: ModeSetting; icon: typeof Sun }> = [
  { value: "system", icon: Monitor },
  { value: "light", icon: Sun },
  { value: "dark", icon: Moon },
];

/** One setting: what it is on the left, its control (or value) on the right; stacked on phones. */
function SettingRow({ label, hint, children, className }: { label: ReactNode; hint?: ReactNode; children: ReactNode; className?: string }) {
  const inset = useRowInset();
  return (
    <li className={cn("flex flex-wrap items-center justify-between gap-x-6 gap-y-2.5 py-3.5", inset, className)}>
      <div className="grid min-w-0 gap-0.5">
        <span className="text-base text-fg">{label}</span>
        {hint ? <span className="text-sm text-fg-muted">{hint}</span> : null}
      </div>
      <div className="flex min-w-0 items-center gap-2">{children}</div>
    </li>
  );
}

function Display() {
  const { t, i18n } = useTranslation();
  const { mode, setMode, entry } = useSkin();
  const language = currentLanguage();
  return (
    <Section title={t("settings.display")}>
      <SectionRows>
        <SettingRow label={t("settings.mode")} hint={t("settings.modeHint")}>
          <ToggleButtonGroup
            aria-label={t("settings.mode")}
            selectionMode="single"
            disallowEmptySelection
            selectedKeys={[mode]}
            onSelectionChange={(keys) => {
              const next = [...keys][0];
              if (next === "system" || next === "light" || next === "dark") setMode(next);
            }}
            className="pv-segmented max-w-full"
          >
            {MODES.map(({ value, icon: Icon }) => (
              <ToggleButton key={value} id={value}>
                <Icon aria-hidden />
                {t(`theme.${value}`)}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </SettingRow>
        <SettingRow label={t("settings.skin")} hint={t("settings.skinHint")}>
          <span className="text-base font-strong text-fg">{localized(entry.manifest.name, i18n.resolvedLanguage ?? "en")}</span>
          <Link to="/apps/themes" className={buttonVariants({ variant: "outline", size: "sm" })}>
            <Palette aria-hidden />
            {t("settings.changeSkin")}
          </Link>
        </SettingRow>
        <SettingRow label={t("language.label")} hint={t("settings.languageHint")}>
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
                <span className="text-sm text-fg-muted">{t(`user.roles.${user.role}`)}</span>
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
      </SectionRows>
    </Section>
  );
}

const TIERS: PerfTier[] = ["tiny", "lite", "standard", "performance"];

function ServerControls({ name }: { name: string }) {
  const { t } = useTranslation();
  const query = useRuntimeSettings();
  const update = useUpdateRuntimeSettings();
  const [draft, setDraft] = useState<RuntimeSettings | null>(null);
  if (query.isPending) return <li><LoadingRows rows={1} /></li>;
  if (!query.data) return <li><ErrorState compact error={query.error} onRetry={() => void query.refetch()} /></li>;
  const settings = query.data;
  return (
    <>
      <SettingRow label={t("settings.serverName")}>
        {draft ? (
          <Input aria-label={t("settings.serverName")} placeholder={t("settings.defaultName")}
            maxLength={64} value={draft.serverName ?? ""}
            onChange={(event) => setDraft({ ...draft, serverName: event.target.value || null })} />
        ) : <span className="text-base text-fg">{name}</span>}
      </SettingRow>
      <SettingRow label={t("settings.perfTier")}>
        {draft ? (
          <select aria-label={t("settings.perfTier")} className="rounded-control border border-line bg-surface px-3 py-2 text-fg"
            value={draft.perfTier ?? ""} onChange={(event) =>
              setDraft({ ...draft, perfTier: TIERS.find((tier) => tier === event.target.value) ?? null })}>
            <option value="">{t("settings.automatic")}</option>
            {TIERS.map((tier) => <option key={tier} value={tier}>{t(`hardware.tiers.${tier}.name`)}</option>)}
          </select>
        ) : <span>{settings.perfTier ? t(`hardware.tiers.${settings.perfTier}.name`) : t("settings.automatic")}</span>}
      </SettingRow>
      <li className="flex flex-wrap items-center justify-end gap-2 px-(--pv-pad-panel) py-3">
        {draft ? (<>
          {update.isError && <span role="alert" className="text-sm text-failed">{errorText(t, update.error)}</span>}
          <Button size="sm" variant="outline" isDisabled={update.isPending} onPress={() => setDraft(null)}>{t("common.cancel")}</Button>
          <Button size="sm" isPending={update.isPending} onPress={() =>
            update.mutate({ ...draft, serverName: draft.serverName?.trim() || null }, { onSuccess: () => setDraft(null) })}>{t("settings.save")}</Button>
        </>) : <Button size="sm" variant="outline" onPress={() => { update.reset(); setDraft({ ...settings }); }}>{t("settings.editServer")}</Button>}
      </li>
    </>
  );
}

function ServerInfo() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const query = useSystemInfo();
  const info = query.data;
  return (
    <Section title={t("settings.server")}>
      {query.isPending ? (
        <LoadingRows rows={2} className="px-(--pv-pad-panel) pb-4" />
      ) : !info ? (
        <ErrorState compact error={query.error} onRetry={() => void query.refetch()} />
      ) : (
        <SectionRows>
          {isAdmin && info.features.includes("settings") ? <ServerControls name={info.name} /> : (
            <SettingRow label={t("settings.serverName")}>
              <span className="text-base text-fg">{info.name}</span>
            </SettingRow>
          )}
          <SettingRow label={t("settings.version")}>
            <span data-num className="text-base text-fg tabular-nums">
              {info.version}
            </span>
          </SettingRow>
          {isAdmin ? (
            <SettingRow label={t("settings.serverId")}>
              <span className="text-sm break-all text-fg-muted select-all">{info.serverId}</span>
            </SettingRow>
          ) : null}
          {isAdmin && info.features.length > 0 ? (
            <SettingRow label={t("settings.features")}>
              <ul className="flex flex-wrap justify-end gap-1.5">
                {info.features.map((feature) => (
                  <li key={feature}>
                    <Chip size="sm" variant="soft" className="bg-sunken text-fg">
                      {feature}
                    </Chip>
                  </li>
                ))}
              </ul>
            </SettingRow>
          ) : null}
        </SectionRows>
      )}
    </Section>
  );
}

/** 系统 › 设置: appearance, language, the signed-in account and the server's identity. */
export function SettingsPage() {
  const { t } = useTranslation();
  return (
    <PageFrame>
      <PageHeader title={t("nav.settings")} />
      <div className="grid w-full max-w-3xl gap-(--pv-gap-section)">
        <Display />
        <Account />
        <ServerInfo />
      </div>
    </PageFrame>
  );
}
