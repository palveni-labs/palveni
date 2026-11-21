import type { EncoderCapability } from "@/api/types";
import type { StatusTone } from "@/ui/marks";

export type EncoderStatus = "verified" | "listed" | "unlisted";

/**
 * "Listed" only means `ffmpeg -encoders` knows the encoder; "verified" means a trial encode
 * actually worked. Listed-but-unverified is the case users trip over, so it stands out.
 */
export function encoderStatus(encoder: EncoderCapability): EncoderStatus {
  if (encoder.verified) return "verified";
  return encoder.listed ? "listed" : "unlisted";
}

/** How each status is drawn: always with its icon and its word, never by colour alone. */
export const ENCODER_TONE: Record<EncoderStatus, StatusTone> = {
  verified: "ok",
  listed: "degraded",
  unlisted: "idle",
};

export const ENCODER_STATUSES: readonly EncoderStatus[] = ["verified", "listed", "unlisted"];
