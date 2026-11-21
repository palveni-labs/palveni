import { Radio } from "lucide-react";
import type { ModuleDefinition } from "../types";

const broadcast: ModuleDefinition = {
  id: "broadcast",
  order: 35,
  titleKey: "modules.broadcast.title",
  home: "/broadcast",
  icon: Radio,
  feature: "broadcast",
};
export default broadcast;
