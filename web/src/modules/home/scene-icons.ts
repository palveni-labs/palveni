import { Clapperboard, DoorOpen, House, Moon, Sunrise, type LucideIcon } from "lucide-react";
import type { Scene } from "@/provisional/types";

export const SCENE_ICONS: Record<Scene["icon"], LucideIcon> = {
  movie: Clapperboard,
  home: House,
  night: Moon,
  morning: Sunrise,
  away: DoorOpen,
};
