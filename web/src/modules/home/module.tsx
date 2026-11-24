import { Clapperboard, House } from "lucide-react";
import { lazy } from "react";
import { provisionalApi } from "@/provisional/api";
import { provisionalKeys } from "@/provisional/keys";
import { upsertDevice } from "@/provisional/queries/home";
import type { HomeDevice } from "@/provisional/types";
import type { ModuleDefinition } from "../types";

/** 家居: rooms, devices, scenes and playback-driven automations (PROVISIONAL API, feature "home"). */
const home: ModuleDefinition = {
  id: "home",
  order: 40,
  group: "space",
  titleKey: "modules.home.title",
  icon: House,
  home: "/home",
  feature: "home",
  widgets: [
    {
      id: "home.quick",
      titleKey: "widgets.homeQuick",
      icon: House,
      region: "aside",
      rank: 20,
      more: { labelKey: "widgets.openHome", to: "/home" },
      component: lazy(() => import("./widgets/quick-controls-widget")),
    },
  ],
  commands: [
    {
      id: "home.movie-scene",
      titleKey: "commands.movieScene",
      icon: Clapperboard,
      feature: "home",
      keywords: ["scene", "movie", "观影", "灯光"],
      run: async ({ queryClient }) => {
        const snapshot = await queryClient.ensureQueryData({ queryKey: provisionalKeys.home, queryFn: () => provisionalApi.home.snapshot() });
        const scene = snapshot.scenes.find((entry) => entry.icon === "movie");
        if (scene) queryClient.setQueryData(provisionalKeys.home, await provisionalApi.home.activateScene(scene.id));
      },
    },
  ],
  events: {
    "home.device.updated": (queryClient, data) => upsertDevice(queryClient, data as HomeDevice),
  },
};

export default home;
