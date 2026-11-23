import { BookOpen, BookText, Heart, Home, Layers, Search } from "lucide-react";
import type { ModuleDefinition } from "../types";

/** 阅读: Ebook, PDF & Manga/Comic shelves and immersive readers. */
const reading: ModuleDefinition = {
  id: "reading",
  order: 30,
  titleKey: "readingApp.title",
  icon: BookOpen,
  home: "/reading",
  feature: "reading",
  nav: [
    { to: "/reading", titleKey: "readingApp.nav.home", icon: Home },
    { to: "/reading/books", titleKey: "readingApp.nav.books", icon: BookText },
    { to: "/reading/comics", titleKey: "readingApp.nav.comics", icon: Layers },
    { to: "/reading/favorites", titleKey: "readingApp.nav.favorites", icon: Heart },
    { to: "/reading/search", titleKey: "readingApp.nav.search", icon: Search },
  ],
  commands: [
    {
      id: "reading.search",
      titleKey: "readingApp.search.commandLabel",
      queryTitleKey: "readingApp.search.commandQuery",
      icon: Search,
      keywords: ["reading", "book", "ebook", "epub", "pdf", "comic", "manga", "阅读", "书籍", "电子书", "漫画"],
      run: ({ navigate }, query) => navigate("/reading/search", { q: query || undefined }),
    },
  ],
};

export default reading;
