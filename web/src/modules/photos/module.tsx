import { Clock, FolderHeart, Heart, Images, MapPin, Search, Sparkles } from "lucide-react";
import type { ModuleDefinition } from "../types";

/** 相册: Chronological timeline, personal albums, offline coordinate map, memories and EXIF lightbox. */
const photos: ModuleDefinition = {
  id: "photos",
  order: 32,
  titleKey: "photosApp.title",
  icon: Images,
  home: "/photos",
  feature: "photos",
  nav: [
    { to: "/photos", titleKey: "photosApp.nav.timeline", icon: Clock },
    { to: "/photos/albums", titleKey: "photosApp.nav.albums", icon: FolderHeart },
    { to: "/photos/places", titleKey: "photosApp.nav.places", icon: MapPin },
    { to: "/photos/memories", titleKey: "photosApp.nav.memories", icon: Sparkles },
    { to: "/photos/favorites", titleKey: "photosApp.nav.favorites", icon: Heart },
  ],
  commands: [
    {
      id: "photos.search",
      titleKey: "photosApp.search.commandLabel",
      queryTitleKey: "photosApp.search.commandQuery",
      icon: Search,
      keywords: ["photo", "photos", "album", "exif", "camera", "相册", "照片", "相簿", "回忆", "地点"],
      run: ({ navigate }, query) => navigate("/photos", { q: query || undefined }),
    },
  ],
};

export default photos;
