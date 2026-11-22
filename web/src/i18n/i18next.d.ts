import "i18next";
import type zhCN from "./locales/zh-CN.json";
import type broadcastZh from "@/modules/broadcast/locales/zh-CN.json";
import type musicZh from "@/modules/music/locales/zh-CN.json";
import type videoZh from "@/modules/video/locales/zh-CN.json";

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    returnNull: false;
    resources: {
      translation: typeof zhCN & { broadcast: typeof broadcastZh; musicApp: typeof musicZh; videoApp: typeof videoZh };
    };
  }
}
