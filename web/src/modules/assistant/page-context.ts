import { useTranslation } from "react-i18next";
import { useActiveModule } from "@/shell/use-shell-modules";

/** What the user is looking at, in words, so "this" means something to the assistant. */
export function usePageContextLabel(): string {
  const { t } = useTranslation();
  const { module, entry } = useActiveModule();
  if (!module) return "";
  return entry ? `${t(module.titleKey)} › ${t(entry.titleKey)}` : t(module.titleKey);
}
