import { request } from "@/api/client";
import type { components } from "@/api/generated";

type Schemas = components["schemas"];

export type NotificationChannelKind = Schemas["NotificationChannelKind"];
export type NotificationChannelConfig = Schemas["NotificationChannelConfig"];
export type NotificationChannel = Schemas["NotificationChannel"];
export type NotificationChannelList = Schemas["NotificationChannelList"];
export type CreateNotificationChannelRequest = Schemas["CreateNotificationChannelRequest"];
export type UpdateNotificationChannelRequest = Schemas["UpdateNotificationChannelRequest"];
export type NotificationDeliveryStatus = Schemas["NotificationDeliveryStatus"];
export type NotificationDelivery = Schemas["NotificationDelivery"];
export type NotificationList = Schemas["NotificationList"];
export type ReadAllNotificationsResponse = Schemas["ReadAllNotificationsResponse"];
export type TestChannelResponse = Schemas["TestChannelResponse"];

export type InboundWebhookSourceKind = Schemas["InboundWebhookSourceKind"];
export type InboundWebhook = Schemas["InboundWebhook"];
export type InboundWebhookList = Schemas["InboundWebhookList"];
export type CreateInboundWebhookRequest = Schemas["CreateInboundWebhookRequest"];
export type CreatedInboundWebhook = Schemas["CreatedInboundWebhook"];

export type RoutineScheduleKind = Schemas["RoutineScheduleKind"];
export type RoutineActionKind = Schemas["RoutineActionKind"];
export type ScheduledRoutine = Schemas["ScheduledRoutine"];
export type ScheduledRoutineList = Schemas["ScheduledRoutineList"];
export type CreateRoutineRequest = Schemas["CreateRoutineRequest"];
export type UpdateRoutineRequest = Schemas["UpdateRoutineRequest"];
export type RunRoutineResponse = Schemas["RunRoutineResponse"];

const enc = encodeURIComponent;

export const notificationsApi = {
  list: (limit = 50, signal?: AbortSignal) =>
    request<NotificationList>("/notifications", { query: { limit }, signal }),
  markAllRead: () =>
    request<ReadAllNotificationsResponse>("/notifications/read-all", { method: "POST" }),
  listChannels: (signal?: AbortSignal) =>
    request<NotificationChannelList>("/notifications/channels", { signal }),
  createChannel: (body: CreateNotificationChannelRequest) =>
    request<NotificationChannel>("/notifications/channels", { method: "POST", body }),
  updateChannel: (id: string, body: UpdateNotificationChannelRequest) =>
    request<NotificationChannel>(`/notifications/channels/${enc(id)}`, { method: "PATCH", body }),
  deleteChannel: (id: string) =>
    request<void>(`/notifications/channels/${enc(id)}`, { method: "DELETE" }),
  testChannel: (id: string) =>
    request<TestChannelResponse>(`/notifications/channels/${enc(id)}/test`, { method: "POST" }),

  listInboundWebhooks: (signal?: AbortSignal) =>
    request<InboundWebhookList>("/webhooks/inbound", { signal }),
  createInboundWebhook: (body: CreateInboundWebhookRequest) =>
    request<CreatedInboundWebhook>("/webhooks/inbound", { method: "POST", body }),
  deleteInboundWebhook: (id: string) =>
    request<void>(`/webhooks/inbound/${enc(id)}`, { method: "DELETE" }),

  listRoutines: (signal?: AbortSignal) =>
    request<ScheduledRoutineList>("/routines", { signal }),
  createRoutine: (body: CreateRoutineRequest) =>
    request<ScheduledRoutine>("/routines", { method: "POST", body }),
  updateRoutine: (id: string, body: UpdateRoutineRequest) =>
    request<ScheduledRoutine>(`/routines/${enc(id)}`, { method: "PATCH", body }),
  deleteRoutine: (id: string) =>
    request<void>(`/routines/${enc(id)}`, { method: "DELETE" }),
  runRoutine: (id: string) =>
    request<RunRoutineResponse>(`/routines/${enc(id)}/run`, { method: "POST" }),
};

export const notificationKeys = {
  all: ["notifications"] as const,
  inbox: () => ["notifications", "inbox"] as const,
  channels: () => ["notifications", "channels"] as const,
  routines: () => ["notifications", "routines"] as const,
  webhooks: () => ["notifications", "webhooks"] as const,
};
