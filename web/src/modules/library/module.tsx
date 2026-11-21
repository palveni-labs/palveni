import { FolderOpen, Library, Search } from "lucide-react";
import { lazy } from "react";
import { useLibraries } from "@/api/queries/libraries";
import type { ModuleDefinition } from "../types";

function useLibraryCount(): number | undefined {
  return useLibraries().data?.libraries.length;
}

/** 资料: libraries grouped by kind and file-level browsing (M0 API). */
const library: ModuleDefinition = {
  id: "library",
  order: 30,
  titleKey: "modules.library.title",
  icon: FolderOpen,
  home: "/library",
  feature: "libraries",
  commands: [{
    id: "library.searchMusic",
    feature: "globalMusicSearch",
    titleKey: "musicSearch.globalLabel",
    queryTitleKey: "musicSearch.searchFor",
    icon: Search,
    run: ({ navigate }, query) => navigate("/library/music-search", { q: query || undefined }),
  }],
  sidebar: {
    titleKey: "nav.libraries",
    component: lazy(() => import("./sidebar-libraries")),
    menu: lazy(() => import("./sidebar-libraries").then((module) => ({ default: module.LibraryMenu }))),
  },
  widgets: [
    {
      id: "library.libraries",
      titleKey: "widgets.libraries",
      icon: Library,
      region: "main",
      rank: 30,
      count: useLibraryCount,
      more: { labelKey: "widgets.manageLibraries", to: "/system/libraries", adminOnly: true },
      component: lazy(() => import("./widgets/libraries-widget")),
    },
  ],
};

export default library;
