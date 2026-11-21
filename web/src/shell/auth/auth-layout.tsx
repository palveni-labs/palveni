import { Button, Dropdown, Label } from "@heroui/react";
import { Check, Languages, Monitor, Moon, Sun } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useSystemInfo } from "@/api/queries/system";
import { currentLanguage, SUPPORTED_LANGUAGES } from "@/i18n";
import { useSecureSurface } from "@/skins/secure";
import { useSkin, type ModeSetting } from "@/skins/store";

const MODES: Array<{ value: ModeSetting; icon: typeof Sun }> = [
  { value: "system", icon: Monitor },
  { value: "light", icon: Sun },
  { value: "dark", icon: Moon },
];

/** Language and light/dark, available before signing in. */
function Preferences() {
  const { t, i18n } = useTranslation();
  const secure = useSecureSurface();
  const { mode, resolved, setMode } = useSkin();
  const language = currentLanguage();
  const ModeIcon = resolved === "dark" ? Moon : Sun;
  return (
    <div className="flex items-center gap-0.5">
      <Dropdown>
        <Button isIconOnly variant="ghost" aria-label={t("language.label")}>
          <Languages aria-hidden />
        </Button>
        <Dropdown.Popover {...secure} placement="bottom end">
          <Dropdown.Menu aria-label={t("language.label")} onAction={(key) => void i18n.changeLanguage(String(key))}>
            {SUPPORTED_LANGUAGES.map((code) => (
              <Dropdown.Item key={code} id={code} textValue={t(`languages.${code}`)} lang={code}>
                <Label className="flex-1">{t(`languages.${code}`)}</Label>
                {language === code ? <Check className="size-4" aria-label={t("common.selected")} /> : null}
              </Dropdown.Item>
            ))}
          </Dropdown.Menu>
        </Dropdown.Popover>
      </Dropdown>
      <Dropdown>
        <Button isIconOnly variant="ghost" aria-label={t("theme.label")}>
          <ModeIcon aria-hidden />
        </Button>
        <Dropdown.Popover {...secure} placement="bottom end">
          <Dropdown.Menu aria-label={t("theme.label")} onAction={(key) => setMode(String(key) as ModeSetting)}>
            {MODES.map(({ value, icon: Icon }) => (
              <Dropdown.Item key={value} id={value} textValue={t(`theme.${value}`)}>
                <Icon className="size-4 text-fg-muted" aria-hidden />
                <Label className="flex-1">{t(`theme.${value}`)}</Label>
                {mode === value ? <Check className="size-4" aria-label={t("common.selected")} /> : null}
              </Dropdown.Item>
            ))}
          </Dropdown.Menu>
        </Dropdown.Popover>
      </Dropdown>
    </div>
  );
}

/** Full-screen frame for the setup wizard and the login page. */
export function AuthLayout({ children, footer }: { children: ReactNode; footer?: ReactNode }) {
  const { t } = useTranslation();
  const secure = useSecureSurface();
  const { data: info } = useSystemInfo();
  return (
    <div {...secure} className="flex min-h-svh flex-col bg-page text-fg">
      <header className="flex h-(--pv-shell-h) items-center justify-between px-4 sm:px-6">
        <span className="font-heading text-[1.1875rem] font-title text-heading">{t("app.name")}</span>
        <Preferences />
      </header>
      <main className="flex flex-1 items-start justify-center px-4 pt-[6vh] pb-16 sm:items-center sm:pt-0">{children}</main>
      <footer className="flex h-12 items-center justify-center gap-2 text-xs text-fg-muted">
        {footer ?? (info ? <span>{t("auth.serverVersion", { name: info.name, version: info.version })}</span> : null)}
      </footer>
    </div>
  );
}

export function AuthCard({ children }: { children: ReactNode }) {
  return <div className="w-full max-w-[26rem] rounded-panel border border-line bg-surface p-6 shadow-raised sm:p-8">{children}</div>;
}
