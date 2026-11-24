import { useTranslation } from "react-i18next";
import { toast } from "@heroui/react";
import type { BlockAction } from "@/blocks/types";
import { errorText } from "@/lib/errors";
import { useUpdateDevice } from "@/provisional/queries/home";

/** Maps device-card block actions to the (provisional) device API. */
export function useDeviceAction() {
  const { t } = useTranslation();
  const update = useUpdateDevice();
  return (action: BlockAction) => {
    if (action.type !== "device") return;
    update.mutate({ id: action.deviceId, patch: action.patch }, { onError: (error) => toast.danger(errorText(t, error)) });
  };
}
