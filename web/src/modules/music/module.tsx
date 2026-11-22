import { Disc3, Heart, Home, ListMusic, Music, Music2, Search, Tag, UsersRound } from "lucide-react";
import type { ModuleDefinition } from "../types";

const music: ModuleDefinition = {
  id: "music",
  order: 24,
  titleKey: "musicApp.title",
  icon: Music,
  home: "/music",
  feature: "music",
  nav: [
    { to: "/music", titleKey: "musicApp.nav.home", icon: Home },
    { to: "/music/albums", titleKey: "musicApp.nav.albums", icon: Disc3 },
    { to: "/music/artists", titleKey: "musicApp.nav.artists", icon: UsersRound },
    { to: "/music/songs", titleKey: "musicApp.nav.songs", icon: Music2 },
    { to: "/music/genres", titleKey: "musicApp.nav.genres", icon: Tag },
    { to: "/music/playlists", titleKey: "musicApp.nav.playlists", icon: ListMusic },
    { to: "/music/favorites", titleKey: "musicApp.nav.favorites", icon: Heart },
    { to: "/music/search", titleKey: "musicApp.nav.search", icon: Search },
  ],
  commands: [
    {
      id: "music.search",
      titleKey: "musicApp.search.commandLabel",
      queryTitleKey: "musicApp.search.commandQuery",
      icon: Search,
      keywords: ["music", "song", "album", "artist", "音乐", "歌曲", "专辑", "艺人"],
      run: ({ navigate }, query) => navigate("/music/search", { q: query || undefined }),
    },
  ],
};

export default music;
