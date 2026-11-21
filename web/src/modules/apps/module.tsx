import { LayoutGrid, Palette, Puzzle } from "lucide-react";
import type { ModuleDefinition } from "../types";

/** 应用: plugins (M0 API) and theme packs (client side). */
const apps: ModuleDefinition = {
  id: "apps",
  order: 60,
  titleKey: "modules.apps.title",
  icon: LayoutGrid,
  home: "/apps",
  group: "manage",
  nav: [
    { to: "/apps/themes", titleKey: "nav.appearance", icon: Palette },
    { to: "/apps/plugins", titleKey: "nav.plugins", icon: Puzzle, adminOnly: true, feature: "plugins" },
  ],
  commands: [
    {
      id: "apps.themes",
      titleKey: "commands.changeTheme",
      icon: Palette,
      keywords: ["theme", "dark", "light", "主题", "外观"],
      run: ({ navigate }) => navigate("/apps/themes"),
    },
  ],
};

export default apps;
