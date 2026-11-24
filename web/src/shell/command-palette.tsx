import { Header, Kbd, Label, ListBox, Modal, SearchField, useFilter } from "@heroui/react";
import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  BookOpen,
  CornerDownLeft,
  Disc3,
  Film,
  Home,
  Image as ImageIcon,
  Layers,
  MessageSquareText,
  Music2,
  Radio,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState, type Key } from "react";
import { Autocomplete } from "react-aria-components";
import { useTranslation } from "react-i18next";
import {
  useGlobalSearch,
  type GlobalSearchHit,
  type GlobalSearchKind,
} from "@/api/queries/shares-collections";
import { useAccess } from "@/modules/access";
import { isEnabled, isVisible, MODULES } from "@/modules/registry";
import type { AppPath, CommandContext } from "@/modules/types";
import { shell, useShellState } from "./shell-store";

const ASSISTANT_FEATURE = "assistant";
const GLOBAL_SEARCH_FEATURE = "globalSearch";
const ASK = "ask-assistant";
const SEARCH_HIT_PREFIX = "hit:";
const SERVER_HIT_SENTINEL = "__pv_hit__:";

const HIT_ICONS: Record<GlobalSearchKind, LucideIcon> = {
  video_work: Film,
  music_album: Disc3,
  track: Music2,
  book: BookOpen,
  comic: BookOpen,
  photo: ImageIcon,
  broadcast: Radio,
  collection: Layers,
  home_device: Home,
  home_scene: Sparkles,
};

/** ⌘K: live cross-domain omnibox (`GET /api/v1/search`), module destinations & actions, plus "ask the assistant". */
export function CommandPalette() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const access = useAccess();
  const { paletteOpen } = useShellState();
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const { contains } = useFilter({ sensitivity: "base" });

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        shell.setPalette(!paletteOpen);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [paletteOpen]);

  useEffect(() => {
    const trimmed = query.trim();
    if (!trimmed) return;
    const timer = setTimeout(() => setDebouncedQuery(trimmed), 150);
    return () => clearTimeout(timer);
  }, [query]);

  const close = () => {
    shell.setPalette(false);
    setQuery("");
    setDebouncedQuery("");
  };

  const context: CommandContext = {
    navigate: (to: AppPath, search?: Record<string, unknown>) => void navigate({ to, search: search as never }),
    openAssistant: (prompt) => shell.openAssistant(prompt),
    queryClient,
    t,
  };

  const globalSearchEnabled = access.features.includes(GLOBAL_SEARCH_FEATURE);
  const effectiveSearchQuery = query.trim().length >= 1 ? debouncedQuery : "";
  const searchQuery = useGlobalSearch(
    effectiveSearchQuery,
    paletteOpen && globalSearchEnabled && effectiveSearchQuery.length >= 1,
    12,
  );
  const searchHits = effectiveSearchQuery.length >= 1 ? (searchQuery.data?.hits ?? []) : [];

  const modules = MODULES.filter((module) => isEnabled(module, access));
  const destinations = modules.flatMap((module) => {
    const entries = (module.nav ?? []).filter((entry) => isEnabled(entry, access));
    return entries.length > 0
      ? entries.map((entry) => ({
          id: `go:${entry.to}`,
          to: entry.to,
          icon: entry.icon ?? module.icon,
          label: `${t(module.titleKey)} › ${t(entry.titleKey)}`,
        }))
      : [{ id: `go:${module.home}`, to: module.home, icon: module.icon, label: t(module.titleKey) }];
  });
  const actions = modules.flatMap((module) => (module.commands ?? []).filter((command) => isEnabled(command, access)));
  const assistantReady =
    access.features.includes(ASSISTANT_FEATURE) && MODULES.some((module) => module.panel && isVisible(module.panel, access));
  const asking = assistantReady && query.trim().length > 0;

  const navigateHit = (hit: GlobalSearchHit) => {
    switch (hit.kind) {
      case "video_work":
        if (hit.libraryId) {
          void navigate({
            to: "/video/$libraryId/works/$workId",
            params: { libraryId: hit.libraryId, workId: hit.id },
          });
        }
        return;
      case "music_album":
        if (hit.libraryId) {
          void navigate({
            to: "/music/albums/$libraryId/$albumId",
            params: { libraryId: hit.libraryId, albumId: hit.id },
          });
        }
        return;
      case "track":
        void navigate({
          to: "/music/songs",
          search: { q: hit.title },
        });
        return;
      case "book":
        void navigate({ to: "/reading/book/$itemId", params: { itemId: hit.id } });
        return;
      case "comic":
        void navigate({ to: "/reading/comic/$itemId", params: { itemId: hit.id } });
        return;
      case "photo":
        void navigate({
          to: "/photos",
          search: { libraryId: hit.libraryId ?? undefined, q: hit.title },
        });
        return;
      case "broadcast":
        void navigate({
          to: "/broadcast",
          search: hit.sourceId ? ({ source: hit.sourceId } as never) : undefined,
        });
        return;
      case "collection":
        void navigate({
          to: "/library/collections/$collectionId",
          params: { collectionId: hit.id },
        });
        return;
      case "home_device":
      case "home_scene":
        void navigate({ to: "/home" });
        return;
    }
  };

  const onAction = (key: Key) => {
    const id = String(key);
    if (id === ASK) {
      shell.openAssistant(query.trim());
      close();
      return;
    }
    if (id.startsWith(SEARCH_HIT_PREFIX)) {
      const hit = searchHits.find((entry) => `${SEARCH_HIT_PREFIX}${entry.kind}:${entry.id}` === id);
      if (hit) {
        close();
        navigateHit(hit);
        return;
      }
    }
    const command = actions.find((entry) => `do:${entry.id}` === id);
    if (command) {
      close();
      void command.run(context, query.trim());
      return;
    }
    const destination = destinations.find((entry) => entry.id === id);
    if (destination) {
      close();
      void navigate({ to: destination.to });
    }
  };

  // Server-matched hits and the "ask" row match any query; everything else is filtered by its words.
  const filter = (text: string, input: string) =>
    text.startsWith(SERVER_HIT_SENTINEL) ||
    text.startsWith(`${t("palette.assistant")}:`) ||
    contains(text, input.trim());

  return (
    <Modal.Backdrop isOpen={paletteOpen} onOpenChange={(open) => (open ? shell.setPalette(true) : close())}>
      <Modal.Container placement="top" size="lg" className="pt-[14vh]">
        <Modal.Dialog aria-label={t("palette.title")} className="overflow-hidden p-0">
          <Autocomplete inputValue={query} onInputChange={setQuery} filter={filter}>
            <SearchField aria-label={t("palette.title")} autoFocus className="border-b border-line">
              <SearchField.Group className="h-13 rounded-none border-0 bg-transparent px-4 shadow-none ring-0 focus-within:ring-0 data-[focus-within=true]:ring-0">
                <SearchField.SearchIcon />
                <SearchField.Input placeholder={t("palette.placeholder")} className="text-base" />
                <Kbd className="ms-2">esc</Kbd>
              </SearchField.Group>
            </SearchField>
            <ListBox
              aria-label={t("palette.title")}
              selectionMode="none"
              onAction={onAction}
              className="max-h-[min(28rem,65vh)] overflow-y-auto p-2"
              renderEmptyState={() => <p className="px-3 py-8 text-center text-sm text-fg-muted">{t("palette.empty")}</p>}
            >
              {searchHits.length > 0 ? (
                <ListBox.Section>
                  <Header className="px-2 pt-1 pb-1 text-xs font-mid text-fg-muted">
                    {t("globalSearch.sectionTitle")}
                  </Header>
                  {searchHits.map((hit) => {
                    const Icon = HIT_ICONS[hit.kind];
                    const itemKey = `${SEARCH_HIT_PREFIX}${hit.kind}:${hit.id}`;
                    return (
                      <ListBox.Item
                        key={itemKey}
                        id={itemKey}
                        textValue={`${SERVER_HIT_SENTINEL}${hit.title} ${hit.subtitle ?? ""}`}
                      >
                        <Icon className="size-4 shrink-0 text-fg-muted" aria-hidden />
                        <Label className="min-w-0 flex-1 truncate">{hit.title}</Label>
                        {hit.subtitle ? (
                          <span className="max-w-44 truncate text-xs text-fg-muted">{hit.subtitle}</span>
                        ) : null}
                        <span className="rounded-chip bg-sunken px-2 py-0.5 text-[0.7rem] font-mid text-fg-muted">
                          {t(`globalSearch.kinds.${hit.kind}`)}
                        </span>
                      </ListBox.Item>
                    );
                  })}
                </ListBox.Section>
              ) : null}
              {asking ? (
                <ListBox.Section>
                  <Header className="px-2 pt-1 pb-1 text-xs font-mid text-fg-muted">{t("palette.assistant")}</Header>
                  <ListBox.Item id={ASK} textValue={`${t("palette.assistant")}: ${query}`}>
                    <MessageSquareText className="size-4 text-fg-muted" aria-hidden />
                    <Label className="flex-1 truncate">{t("palette.ask", { query: query.trim() })}</Label>
                    <CornerDownLeft className="size-3.5 text-fg-subtle" aria-hidden />
                  </ListBox.Item>
                </ListBox.Section>
              ) : null}
              {actions.length > 0 ? (
                <ListBox.Section>
                  <Header className="px-2 pt-2 pb-1 text-xs font-mid text-fg-muted">{t("palette.actions")}</Header>
                  {actions.map((command) => {
                    const Icon = command.icon;
                    const label =
                      command.queryTitleKey && query.trim()
                        ? t(command.queryTitleKey, { query: query.trim() })
                        : t(command.titleKey);
                    return (
                      <ListBox.Item
                        key={command.id}
                        id={`do:${command.id}`}
                        textValue={`${label} ${(command.keywords ?? []).join(" ")}`}
                      >
                        <Icon className="size-4 text-fg-muted" aria-hidden />
                        <Label>{label}</Label>
                      </ListBox.Item>
                    );
                  })}
                </ListBox.Section>
              ) : null}
              <ListBox.Section>
                <Header className="px-2 pt-2 pb-1 text-xs font-mid text-fg-muted">{t("palette.goTo")}</Header>
                {destinations.map((destination) => {
                  const Icon = destination.icon;
                  return (
                    <ListBox.Item key={destination.id} id={destination.id} textValue={destination.label}>
                      <Icon className="size-4 text-fg-muted" aria-hidden />
                      <Label>{destination.label}</Label>
                    </ListBox.Item>
                  );
                })}
              </ListBox.Section>
            </ListBox>
          </Autocomplete>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}
