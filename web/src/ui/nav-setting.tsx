/**
 * The desktop navigation dock position override (跟随皮肤 / 左侧 / 顶部 / 右侧).
 * Kept independent of the skin so users can pair any visual skin with their preferred navigation.
 */
import { ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { PanelLeft, PanelRight, PanelTop, Sparkles, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useSkin, type NavSetting as Setting } from "@/skins/store";

const OPTIONS: Array<{
  value: Setting;
  icon: LucideIcon;
  labelKey: "nav.layoutSkin" | "nav.layoutSidebar" | "nav.layoutTopbar" | "nav.layoutSidebarRight";
}> = [
  { value: "skin", icon: Sparkles, labelKey: "nav.layoutSkin" },
  { value: "sidebar", icon: PanelLeft, labelKey: "nav.layoutSidebar" },
  { value: "topbar", icon: PanelTop, labelKey: "nav.layoutTopbar" },
  { value: "sidebar-right", icon: PanelRight, labelKey: "nav.layoutSidebarRight" },
];

function isSetting(value: unknown): value is Setting {
  return value === "skin" || value === "sidebar" || value === "topbar" || value === "sidebar-right";
}

export function NavSetting({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { nav, setNav } = useSkin();
  return (
    <ToggleButtonGroup
      aria-label={t("nav.layout")}
      selectionMode="single"
      disallowEmptySelection
      selectedKeys={[nav]}
      onSelectionChange={(keys) => {
        const next = [...keys][0];
        if (isSetting(next)) setNav(next);
      }}
      className={cn("pv-segmented max-w-full flex-wrap", className)}
    >
      {OPTIONS.map(({ value, icon: Icon, labelKey }) => (
        <ToggleButton key={value} id={value}>
          <Icon className="size-4" aria-hidden />
          {t(labelKey)}
        </ToggleButton>
      ))}
    </ToggleButtonGroup>
  );
}
