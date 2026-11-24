import { Lightbulb, MessageSquarePlus, MessageSquareText } from "lucide-react";
import { lazy } from "react";
import { upsertRun } from "@/provisional/queries/assistant";
import type { AssistantRun } from "@/provisional/types";
import type { ModuleDefinition } from "../types";
import { useAssistantInbox } from "./use-assistant-inbox";

/** 助手: conversation + result canvas, and the ⌘J panel (PROVISIONAL API, feature "assistant"). */
const assistant: ModuleDefinition = {
  id: "assistant",
  order: 20,
  titleKey: "modules.assistant.title",
  icon: MessageSquareText,
  home: "/assistant",
  inbox: useAssistantInbox,
  feature: "assistant",
  panel: {
    titleKey: "assistant.title",
    feature: "assistant",
    component: lazy(() => import("./assistant-panel")),
  },
  widgets: [
    {
      id: "assistant.suggestions",
      titleKey: "widgets.suggestions",
      icon: Lightbulb,
      region: "aside",
      rank: 30,
      component: lazy(() => import("./widgets/suggestions-widget")),
    },
  ],
  commands: [
    {
      id: "assistant.open",
      titleKey: "commands.askAssistant",
      icon: MessageSquarePlus,
      keywords: ["ai", "assistant", "助手"],
      run: ({ openAssistant }) => openAssistant(),
    },
  ],
  events: {
    "assistant.run.updated": (queryClient, data) => upsertRun(queryClient, data as AssistantRun),
  },
};

export default assistant;
