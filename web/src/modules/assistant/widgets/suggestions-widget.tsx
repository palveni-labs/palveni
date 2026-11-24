import { Button } from "@heroui/react";
import { Lightbulb } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSuggestions } from "@/provisional/queries/assistant";
import { shell } from "@/shell/shell-store";
import { SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, LoadingRows } from "@/ui/states";

/** PROVISIONAL: things the assistant noticed (feature "assistant"), each with the action it offers. */
export default function SuggestionsWidget() {
  const { t } = useTranslation();
  const inset = useRowInset();
  const { data, isPending } = useSuggestions(true);
  if (isPending) return <LoadingRows rows={2} className="px-5 pb-4" />;
  const suggestions = (data?.suggestions ?? []).slice(0, 2);
  if (suggestions.length === 0) return <EmptyState compact icon={Lightbulb} title={t("widgets.noSuggestions")} />;
  return (
    <SectionRows>
      {suggestions.map((suggestion) => (
        <li key={suggestion.id} className={`flex items-center gap-3 py-3 ${inset}`}>
          <p className="min-w-0 flex-1 text-base text-fg">{suggestion.text}</p>
          <Button size="sm" variant="outline" className="shrink-0" onPress={() => shell.openAssistant(suggestion.prompt)}>
            {suggestion.actionLabel ?? t("assistant.askAbout")}
          </Button>
        </li>
      ))}
    </SectionRows>
  );
}
