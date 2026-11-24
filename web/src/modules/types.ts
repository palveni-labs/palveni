import type { QueryClient } from "@tanstack/react-query";
import type { ParseKeys, TFunction } from "i18next";
import type { LucideIcon } from "lucide-react";
import type { ComponentType, ReactNode } from "react";
import type { User } from "@/api/types";
import type { FileRouteTypes } from "@/routeTree.gen";

/** Every navigable path in the app (generated from the route tree). */
export type AppPath = FileRouteTypes["to"];

/** i18n key from the zh-CN resource (the type source for all locales). */
export type MessageKey = ParseKeys;

/** Who can see a contribution and what the server must support for it. */
export interface Availability {
  /** Server feature flag from `GET /system/info` → `features`. */
  feature?: string;
  adminOnly?: boolean;
}

export interface NavMenuProps {
  active: boolean;
  className: string;
  children: ReactNode;
}

export interface NavEntry extends Availability {
  to: AppPath;
  titleKey: MessageKey;
  icon?: LucideIcon;
}

/**
 * Where an overview section sits. The overview's order is fixed by "what needs me first"
 *: the main column holds the work, the aside the state of the house.
 */
export type WidgetRegion = "main" | "aside";

export interface WidgetDefinition extends Availability {
  /** Unique across all modules; used to persist the dashboard layout. */
  id: string;
  titleKey: MessageKey;
  icon: LucideIcon;
  region: WidgetRegion;
  /** Position within the region (lower first). */
  rank: number;
  component: ComponentType;
  /** The section's "more" link. */
  more?: { labelKey: MessageKey; to: AppPath; search?: Record<string, string>; adminOnly?: boolean };
  /** Instead of a link, a header button (e.g. open the inbox). */
  moreAction?: { labelKey: MessageKey; run: () => void };
  /** Hook returning the number shown after the title (undefined hides it). */
  count?: () => number | undefined;
  /** The section holds things that need the user (raised where the `attention` slot is `lamp`). */
  needsYou?: boolean;
  /** The section's items are tiles (never framed in a panel). */
  tiles?: boolean;
}

export interface CommandContext {
  navigate: (to: AppPath, search?: Record<string, unknown>) => void;
  openAssistant: (prompt?: string) => void;
  queryClient: QueryClient;
  t: TFunction;
}

export interface CommandDefinition extends Availability {
  id: string;
  titleKey: MessageKey;
  /** Label for an action that accepts the current palette text as its query. */
  queryTitleKey?: MessageKey;
  icon: LucideIcon;
  keywords?: string[];
  run: (context: CommandContext, query: string) => void | Promise<void>;
}

/** A button an inbox item offers where it is listed (overview "需要你处理"). */
export interface InboxAction {
  id: string;
  label: string;
  variant: "confirm" | "outline" | "ghost";
  icon?: LucideIcon;
  run?: () => void;
  to?: AppPath;
  search?: Record<string, string>;
  busy?: boolean;
}

export interface InboxItem {
  id: string;
  /** approval: waits for a decision; alert: a degraded state needing action; notification: news. */
  kind: "approval" | "alert" | "notification";
  title: string;
  detail?: string;
  createdAt: string;
  tone?: "default" | "success" | "warning" | "danger";
  to: AppPath;
  search?: Record<string, string>;
  /** Where it applies ("无损音乐"). */
  scope?: string;
  /** Covers of what an approval would change. */
  previews?: string[];
  /** A short state word for alerts ("已掉线"). */
  state?: string;
  actions?: InboxAction[];
}

/** A hook returning inbox entries; each source is mounted in its own component. */
export type InboxSource = () => InboxItem[];

/** What a module is doing right now, for the shell's status display. */
export interface ActivitySummary {
  /** e.g. "转码" */
  label: string;
  /** e.g. "The Velvet Horizon" */
  title: string;
  /** 0–1, or null when unknown. */
  progress: number | null;
  /** Other work running besides this one. */
  more: number;
  to: AppPath;
}

export interface PanelDefinition extends Availability {
  titleKey: MessageKey;
  component: ComponentType<{ onClose: () => void }>;
}

/** Handles one server-sent event name for this module. */
export type EventHandler = (queryClient: QueryClient, data: unknown) => void;

export interface ModuleDefinition extends Availability {
  /** Matches the folder name; also the URL prefix (the overview module owns "/"). */
  id: string;
  /** Position in the navigation. */
  order: number;
  titleKey: MessageKey;
  icon: LucideIcon;
  /** Path the module's navigation item opens. */
  home: AppPath;
  /** Sub-pages shown as tabs above the module's pages; omit for single-page modules. */
  nav?: NavEntry[];
  widgets?: WidgetDefinition[];
  commands?: CommandDefinition[];
  inbox?: InboxSource;
  /** Hook returning a count shown on the module's navigation item (0 hides it). */
  badge?: () => number;
  /** Navigation group in a sidebar: main destinations, media, space (home/tasks), or management. */
  group?: "main" | "media" | "space" | "manage";
  /** Hook summarising the most important running work (shown by the shell's status display). */
  activity?: () => ActivitySummary | null;
  /** A titled list the module adds to a sidebar navigation (e.g. the libraries). */
  sidebar?: { titleKey: MessageKey; component: ComponentType; menu?: ComponentType<NavMenuProps> };
  /** Side panel the shell can open on every page (⌘J). At most one module provides it. */
  panel?: PanelDefinition;
  events?: Record<string, EventHandler>;
}

export interface AccessContext {
  features: readonly string[];
  user: Pick<User, "role"> | null | undefined;
}
