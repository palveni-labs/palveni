import type { TFunction } from "i18next";
import { ApiError } from "@/api/errors";

const CODES_WITH_DETAIL = new Set(["validation_failed", "path_not_found", "conflict"]);

export interface ErrorDescription {
  title: string;
  /** The server's own message when it adds something to the translated title. */
  detail?: string;
}

/** Translated, user-facing description of any thrown value. */
export function describeError(t: TFunction, error: unknown): ErrorDescription {
  if (error instanceof ApiError) {
    if (!error.isKnownCode) return { title: error.message || t("errors.unexpected_response") };
    const title = t(`errors.${error.code as "internal"}`);
    const detail = CODES_WITH_DETAIL.has(error.code) && error.message ? error.message : undefined;
    return { title, detail };
  }
  return { title: t("errors.unknown") };
}

export function errorText(t: TFunction, error: unknown): string {
  const { title, detail } = describeError(t, error);
  return detail ? t("errors.withDetail", { title, detail }) : title;
}
