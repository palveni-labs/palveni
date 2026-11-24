import { toast } from "@heroui/react";
import { Bell, Cpu, FolderCog, FolderPlus, RefreshCw, ScanSearch, Server, Settings2, ShieldCheck, Users } from "lucide-react";
import { lazy } from "react";
import { api } from "@/api/endpoints";
import { queryKeys } from "@/api/query-keys";
import type { ModuleDefinition } from "../types";
import { useStorageAlerts } from "./use-storage-alerts";

/** 系统: library management, hardware & transcoding, settings (M0 API). */
const system: ModuleDefinition = {
  id: "system",
  order: 70,
  titleKey: "modules.system.title",
  icon: Settings2,
  home: "/system",
  group: "manage",
  inbox: useStorageAlerts,
  nav: [
    { to: "/system/libraries", titleKey: "nav.manageLibraries", icon: FolderCog, adminOnly: true, feature: "libraries" },
    { to: "/system/users", titleKey: "nav.users", icon: Users, adminOnly: true, feature: "users" },
    { to: "/system/maintenance", titleKey: "nav.maintenance", icon: ShieldCheck, adminOnly: true, feature: "audit" },
    { to: "/system/hardware", titleKey: "nav.hardware", icon: Cpu, adminOnly: true, feature: "hardware" },
    { to: "/system/notifications", titleKey: "nav.notifications", icon: Bell, feature: "notifications" },
    { to: "/system/settings", titleKey: "nav.settings", icon: Settings2 },
  ],
  widgets: [
    {
      id: "system.server",
      titleKey: "widgets.server",
      icon: Server,
      region: "aside",
      rank: 10,
      more: { labelKey: "widgets.openSystem", to: "/system" },
      component: lazy(() => import("./widgets/server-widget")),
    },
  ],
  commands: [
    {
      id: "system.new-library",
      titleKey: "adminLibraries.create",
      icon: FolderPlus,
      adminOnly: true,
      feature: "libraries",
      keywords: ["library", "folder", "媒体库"],
      run: ({ navigate }) => navigate("/system/libraries", { create: true }),
    },
    {
      id: "system.scan-all",
      titleKey: "commands.scanAll",
      icon: ScanSearch,
      adminOnly: true,
      feature: "libraries",
      keywords: ["scan", "扫描"],
      run: async ({ queryClient, t }) => {
        try {
          const { libraries } = await api.libraries.list();
          await Promise.all(libraries.map((library) => api.libraries.scan(library.id)));
          void queryClient.invalidateQueries({ queryKey: queryKeys.jobs.all });
          toast.success(t("commands.scanAllStarted", { count: libraries.length }));
        } catch {
          toast.danger(t("errors.unknown"));
        }
      },
    },
    {
      id: "system.probe",
      titleKey: "hardware.refresh",
      icon: RefreshCw,
      adminOnly: true,
      feature: "hardware",
      keywords: ["ffmpeg", "hardware", "硬件"],
      run: async ({ queryClient, t, navigate }) => {
        navigate("/system/hardware");
        try {
          queryClient.setQueryData(queryKeys.system.hardware, await api.system.hardware(true));
          toast.success(t("hardware.refreshed"));
        } catch {
          toast.danger(t("errors.unknown"));
        }
      },
    },
  ],
};

export default system;
