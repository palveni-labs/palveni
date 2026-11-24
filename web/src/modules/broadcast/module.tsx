import { Radio, Tv } from "lucide-react";
import type { ModuleDefinition } from "../types";

const broadcast: ModuleDefinition = {
  id: "broadcast",
  order: 30,
  group: "media",
  titleKey: "modules.broadcast.title",
  home: "/broadcast",
  icon: Radio,
  feature: "broadcast",
  nav: [
    { to: "/broadcast", titleKey: "broadcast.sources", icon: Radio, feature: "broadcast" },
    { to: "/broadcast/virtual", titleKey: "broadcast.virtualTv.nav", icon: Tv, feature: "virtualTv" },
  ],
};
export default broadcast;
