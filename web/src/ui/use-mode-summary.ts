import { useTranslation } from "react-i18next";
import { useSkin } from "@/skins/store";

/** "跟随系统，现在是深色" / "浅色": what the setting means right now, in words. */
export function useModeSummary(): string {
  const { t } = useTranslation();
  const { mode, resolved } = useSkin();
  return mode === "system" ? t("appearance.mode.systemNow", { mode: t(`theme.${resolved}`) }) : t(`theme.${mode}`);
}
