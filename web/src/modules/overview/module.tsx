import { CircleDot, LayoutDashboard } from "lucide-react";
import { lazy } from "react";
import { shell } from "@/shell/shell-store";
import { useAttentionCount } from "@/shell/inbox-store";
import type { ModuleDefinition } from "../types";

/** 概览: assembled from every module's sections, in the order "what needs me first". */
const overview: ModuleDefinition = {
  id: "overview",
  order: 10,
  titleKey: "modules.overview.title",
  icon: LayoutDashboard,
  home: "/",
  widgets: [
    {
      id: "overview.attention",
      titleKey: "widgets.attention",
      icon: CircleDot,
      region: "main",
      rank: 10,
      needsYou: true,
      count: useAttentionCount,
      moreAction: { labelKey: "widgets.openInbox", run: () => shell.setInbox(true) },
      component: lazy(() => import("./attention-widget")),
    },
  ],
};

export default overview;
