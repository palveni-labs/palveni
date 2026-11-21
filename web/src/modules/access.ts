import { useMe } from "@/api/queries/auth";
import { useSystemInfo } from "@/api/queries/system";
import type { AccessContext } from "./types";

const NO_FEATURES: readonly string[] = [];

/** Server features and the signed-in user, for availability checks. */
export function useAccess(): AccessContext {
  const { data: info } = useSystemInfo();
  const { data: user } = useMe();
  return { features: info?.features ?? NO_FEATURES, user };
}

export function useFeature(feature: string | undefined): boolean {
  const { features } = useAccess();
  return !feature || features.includes(feature);
}
