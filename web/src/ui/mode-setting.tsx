/**
 * The light/dark setting (跟随系统 / 浅色 / 深色) as a segmented control. It is independent of the
 * skin: switching skins keeps it.
 */
import { ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { Monitor, Moon, Sun, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useSkin, type ModeSetting as Setting } from "@/skins/store";

const OPTIONS: Array<{ value: Setting; icon: LucideIcon }> = [
  { value: "system", icon: Monitor },
  { value: "light", icon: Sun },
  { value: "dark", icon: Moon },
];

function isSetting(value: unknown): value is Setting {
  return value === "system" || value === "light" || value === "dark";
}

export function ModeSetting({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { mode, setMode } = useSkin();
  return (
    <ToggleButtonGroup
      aria-label={t("appearance.mode.title")}
      selectionMode="single"
      disallowEmptySelection
      selectedKeys={[mode]}
      onSelectionChange={(keys) => {
        const next = [...keys][0];
        if (isSetting(next)) setMode(next);
      }}
      className={cn("pv-segmented", className)}
    >
      {OPTIONS.map(({ value, icon: Icon }) => (
        <ToggleButton key={value} id={value}>
          <Icon className="size-4" aria-hidden />
          {t(`theme.${value}`)}
        </ToggleButton>
      ))}
    </ToggleButtonGroup>
  );
}
