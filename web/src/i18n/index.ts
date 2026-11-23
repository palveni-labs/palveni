import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";
import broadcastEn from "@/modules/broadcast/locales/en.json";
import broadcastZh from "@/modules/broadcast/locales/zh-CN.json";
import musicEn from "@/modules/music/locales/en.json";
import musicZh from "@/modules/music/locales/zh-CN.json";
import photosEn from "@/modules/photos/locales/en.json";
import photosZh from "@/modules/photos/locales/zh-CN.json";
import videoEn from "@/modules/video/locales/en.json";
import videoZh from "@/modules/video/locales/zh-CN.json";

export const SUPPORTED_LANGUAGES = ["zh-CN", "en"] as const;
export type Language = (typeof SUPPORTED_LANGUAGES)[number];

export const LANGUAGE_STORAGE_KEY = "palveni.lang";

/** Maps any browser language tag onto a supported locale (zh-TW, zh-HK → zh-CN; en-GB → en). */
export function normalizeLanguage(tag: string): string {
  const lower = tag.toLowerCase();
  if (lower.startsWith("zh")) return "zh-CN";
  if (lower.startsWith("en")) return "en";
  return tag;
}

export function currentLanguage(): Language {
  const resolved = i18n.resolvedLanguage ?? i18n.language;
  return resolved === "zh-CN" ? "zh-CN" : "en";
}

function syncDocument(language: string) {
  if (typeof document === "undefined") return;
  document.documentElement.lang = language === "zh-CN" ? "zh-CN" : "en";
}

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      "zh-CN": { translation: { ...zhCN, broadcast: broadcastZh, musicApp: musicZh, photosApp: photosZh, videoApp: videoZh } },
      en: { translation: { ...en, broadcast: broadcastEn, musicApp: musicEn, photosApp: photosEn, videoApp: videoEn } },
    },
    supportedLngs: [...SUPPORTED_LANGUAGES],
    fallbackLng: "en",
    load: "currentOnly",
    initAsync: false,
    returnNull: false,
    interpolation: { escapeValue: false },
    detection: {
      order: ["localStorage", "navigator"],
      lookupLocalStorage: LANGUAGE_STORAGE_KEY,
      caches: ["localStorage"],
      convertDetectedLanguage: normalizeLanguage,
    },
    react: { useSuspense: false },
  });

syncDocument(currentLanguage());
i18n.on("languageChanged", () => syncDocument(currentLanguage()));

export default i18n;
