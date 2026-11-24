import { Film, Heart, Home, Search } from "lucide-react";
import type { ModuleDefinition } from "../types";

/** 影视: Emby-level home, poster wall, work details, search, favorites and fullscreen cinema player. */
const video: ModuleDefinition = {
  id: "video",
  order: 22,
  group: "media",
  titleKey: "kinds.video",
  icon: Film,
  home: "/video",
  feature: "video",
  nav: [
    { to: "/video", titleKey: "videoApp.nav.home", icon: Home },
    { to: "/video/favorites", titleKey: "videoApp.nav.favorites", icon: Heart },
    { to: "/video/search", titleKey: "videoApp.nav.search", icon: Search },
  ],
  commands: [
    {
      id: "video.search",
      titleKey: "videoApp.search.commandLabel",
      queryTitleKey: "videoApp.search.commandQuery",
      icon: Search,
      keywords: ["video", "movie", "series", "tv", "film", "影视", "电影", "剧集"],
      run: ({ navigate }, query) => navigate("/video/search", { q: query || undefined }),
    },
  ],
};

export default video;
