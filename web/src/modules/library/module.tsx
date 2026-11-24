import { FolderOpen, Images, Layers, Library, Link2, Search } from "lucide-react";
import { lazy } from "react";
import { useLibraries } from "@/api/queries/libraries";
import type { ModuleDefinition } from "../types";

function useLibraryCount(): number | undefined {
  return useLibraries().data?.libraries.length;
}

/** 资料: libraries grouped by kind and file-level browsing (M0 API). */
const library: ModuleDefinition = {
  id: "library",
  order: 32,
  group: "media",
  titleKey: "modules.library.title",
  icon: FolderOpen,
  home: "/library",
  feature: "libraries",
  nav: [
    { to: "/library", titleKey: "nav.allLibraries", icon: FolderOpen },
    { to: "/library/collections", titleKey: "nav.collections", icon: Layers, feature: "collections" },
    { to: "/library/shares", titleKey: "nav.shares", icon: Link2, feature: "shares" },
  ],
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
    {
      id: "library.recent",
      titleKey: "widgets.recent",
      icon: Images,
      region: "main",
      rank: 40,
      feature: "artwork",
      tiles: true,
      more: { labelKey: "widgets.viewInLibrary", to: "/library" },
      component: lazy(() => import("./widgets/recent-widget")),
    },
  ],
};

export default library;
