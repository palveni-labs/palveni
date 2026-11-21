import { useTranslation } from "react-i18next";
import { MODULES } from "@/modules/registry";
import type { ActivitySummary } from "@/modules/types";
import { useAttentionCount } from "@/shell/inbox-store";

const useNoActivity = (): ActivitySummary | null => null;
/** The registry is fixed at build time, so the hook choice never changes between renders. */
const useModuleActivity = MODULES.find((module) => module.activity)?.activity ?? useNoActivity;

/** "3 件事等你处理，3 项工作正在进行" — the overview's one-line state. */
export function useOverviewSummary(): string {
  const { t } = useTranslation();
  const attention = useAttentionCount();
  const activity = useModuleActivity();
  const running = activity ? activity.more + 1 : 0;
  if (attention === 0 && running === 0) return t("overview.summary.calm");
  const parts = [
    attention > 0 ? t("overview.summary.attention", { count: attention }) : null,
    running > 0 ? t("overview.summary.running", { count: running }) : null,
  ].filter(Boolean);
  return parts.join(t("overview.summary.separator"));
}
