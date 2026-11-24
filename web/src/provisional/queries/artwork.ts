import { useQuery } from "@tanstack/react-query";
import { useFeature } from "@/modules/access";
import { extractWorkHue, type WorkHue } from "@/skins/work-color";
import { provisionalApi } from "../api";
import { provisionalKeys } from "../keys";

export interface LoadedArtwork {
  /** Object URL of the image (kept for the session; the same cover is shared everywhere). */
  url: string;
  /** Main colour of the cover; null if the image could not be measured. */
  hue: WorkHue | null;
}

const SAMPLE = 48;

/** Reads the cover's pixels at a small size and extracts its main colour. */
async function measure(blob: Blob): Promise<WorkHue | null> {
  if (typeof createImageBitmap !== "function" || typeof document === "undefined") return null;
  try {
    const bitmap = await createImageBitmap(blob);
    const canvas = document.createElement("canvas");
    const scale = SAMPLE / Math.max(bitmap.width, bitmap.height);
    canvas.width = Math.max(1, Math.round(bitmap.width * scale));
    canvas.height = Math.max(1, Math.round(bitmap.height * scale));
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) return null;
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    bitmap.close();
    return extractWorkHue(context.getImageData(0, 0, canvas.width, canvas.height).data);
  } catch {
    return null;
  }
}

/** Environments without object URLs (tests) render covers as placeholders. */
const canShowImages = typeof URL !== "undefined" && typeof URL.createObjectURL === "function" && typeof createImageBitmap === "function";

/** PROVISIONAL: a cover image and its work colour, loaded once per artwork id. */
export function useArtwork(artworkId: string | null | undefined) {
  const enabled = useFeature("artwork") && Boolean(artworkId) && canShowImages;
  return useQuery({
    queryKey: provisionalKeys.artwork(artworkId ?? ""),
    queryFn: async ({ signal }): Promise<LoadedArtwork> => {
      const blob = await provisionalApi.artwork.image(artworkId ?? "", signal);
      return { url: URL.createObjectURL(blob), hue: await measure(blob) };
    },
    enabled,
    staleTime: Infinity,
    gcTime: Infinity,
    retry: false,
  });
}
