import { Avatar, Dropdown, Header, Label, Separator } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import { Check, ChevronsUpDown, LogOut, Monitor, Moon, Palette, Sun } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useCurrentUser, useLogout } from "@/api/queries/auth";
import { currentLanguage, SUPPORTED_LANGUAGES } from "@/i18n";
import { useSkin, type ModeSetting } from "@/skins/store";

const MODES: Array<{ value: ModeSetting; icon: typeof Sun }> = [
  { value: "system", icon: Monitor },
  { value: "light", icon: Sun },
  { value: "dark", icon: Moon },
];

export function UserAvatar({ name, className }: { name: string; className?: string }) {
  return (
    <Avatar size="sm" className={cn("size-8 shrink-0 bg-shell-field text-shell-fg", className)}>
      <Avatar.Fallback className="bg-transparent text-[0.8125rem] font-strong text-current uppercase">{name.slice(0, 1)}</Avatar.Fallback>
    </Avatar>
  );
}

/** Account menu: appearance (light/dark, skin), language, sign out. */
export function UserMenu({ layout = "avatar" }: { layout?: "avatar" | "row" }) {
  const { t, i18n } = useTranslation();
  const user = useCurrentUser();
  const logout = useLogout();
  const navigate = useNavigate();
  const { mode, setMode } = useSkin();
  const language = currentLanguage();

  const onAction = (key: React.Key) => {
    const id = String(key);
    if (id.startsWith("mode:")) setMode(id.slice(5) as ModeSetting);
    else if (id.startsWith("lang:")) void i18n.changeLanguage(id.slice(5));
    else if (id === "appearance") void navigate({ to: "/apps/themes" });
    else if (id === "logout") logout.mutate();
  };

  return (
    <Dropdown>
      <Dropdown.Trigger
        aria-label={t("user.menu")}
        className={cn(
          "flex items-center gap-2.5 rounded-full outline-none focus-visible:ring-2 focus-visible:ring-focus",
          layout === "row" && "w-full rounded-tile px-2 py-1.5 text-start hover:bg-hover",
        )}
      >
        <UserAvatar name={user.username} />
        {layout === "row" ? (
          <>
            <span className="grid min-w-0 flex-1 leading-tight">
              <span className="truncate text-sm font-mid text-fg">{user.username}</span>
              <span className="truncate text-xs text-fg-muted">{t(`user.roles.${user.role}`)}</span>
            </span>
            <ChevronsUpDown className="size-4 text-fg-subtle" aria-hidden />
          </>
        ) : null}
      </Dropdown.Trigger>
      <Dropdown.Popover placement={layout === "row" ? "top start" : "bottom end"} className="min-w-60">
        <Dropdown.Menu aria-label={t("user.menu")} onAction={onAction}>
          <Dropdown.Section>
            <Header className="px-2 py-1.5 text-sm font-strong text-fg">
              {user.username}
              <span className="ms-2 text-xs font-body text-fg-muted">{t(`user.roles.${user.role}`)}</span>
            </Header>
          </Dropdown.Section>
          <Separator />
          <Dropdown.Section>
            <Header className="px-2 pt-1 text-xs font-mid text-fg-muted">{t("theme.label")}</Header>
            {MODES.map(({ value, icon: Icon }) => (
              <Dropdown.Item key={value} id={`mode:${value}`} textValue={t(`theme.${value}`)}>
                <Icon className="size-4 text-fg-muted" aria-hidden />
                <Label className="flex-1">{t(`theme.${value}`)}</Label>
                {mode === value ? <Check className="size-4 text-fg" aria-label={t("common.selected")} /> : null}
              </Dropdown.Item>
            ))}
            <Dropdown.Item id="appearance" textValue={t("skins.title")}>
              <Palette className="size-4 text-fg-muted" aria-hidden />
              <Label>{t("skins.title")}</Label>
            </Dropdown.Item>
          </Dropdown.Section>
          <Separator />
          <Dropdown.Section>
            <Header className="px-2 pt-1 text-xs font-mid text-fg-muted">{t("language.label")}</Header>
            {SUPPORTED_LANGUAGES.map((code) => (
              <Dropdown.Item key={code} id={`lang:${code}`} textValue={t(`languages.${code}`)} lang={code}>
                <Label className="flex-1">{t(`languages.${code}`)}</Label>
                {language === code ? <Check className="size-4 text-fg" aria-label={t("common.selected")} /> : null}
              </Dropdown.Item>
            ))}
          </Dropdown.Section>
          <Separator />
          <Dropdown.Item id="logout" textValue={t("user.logout")}>
            <LogOut className="size-4 text-fg-muted" aria-hidden />
            <Label>{t("user.logout")}</Label>
          </Dropdown.Item>
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}
