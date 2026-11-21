import "i18next";
import type zhCN from "./locales/zh-CN.json";
import type broadcastZh from "@/modules/broadcast/locales/zh-CN.json";

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    returnNull: false;
    resources: {
      translation: typeof zhCN & { broadcast: typeof broadcastZh };
    };
  }
}
