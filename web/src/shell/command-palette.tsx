import { Header, Kbd, Label, ListBox, Modal, SearchField, useFilter } from "@heroui/react";
import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState, type Key } from "react";
import { Autocomplete } from "react-aria-components";
import { useTranslation } from "react-i18next";
import { useAccess } from "@/modules/access";
import { isEnabled, MODULES } from "@/modules/registry";
import type { AppPath, CommandContext } from "@/modules/types";
import { shell, useShellState } from "./shell-store";

export function CommandPalette() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const access = useAccess();
  const { paletteOpen } = useShellState();
  const [query, setQuery] = useState("");
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

  const close = () => {
    shell.setPalette(false);
    setQuery("");
  };
  const context: CommandContext = {
    navigate: (to: AppPath, search?: Record<string, unknown>) => void navigate({ to, search: search as never }),
    queryClient,
    t,
  };

  const modules = MODULES.filter((module) => isEnabled(module, access));
  const destinations = modules.flatMap((module) => {
    const entries = (module.nav ?? []).filter((entry) => isEnabled(entry, access));
    return entries.length > 0
      ? entries.map((entry) => ({ id: `go:${entry.to}`, to: entry.to, icon: entry.icon ?? module.icon, label: `${t(module.titleKey)} › ${t(entry.titleKey)}` }))
      : [{ id: `go:${module.home}`, to: module.home, icon: module.icon, label: t(module.titleKey) }];
  });
  const actions = modules.flatMap((module) => (module.commands ?? []).filter((command) => isEnabled(command, access)));

  const onAction = (key: Key) => {
    const id = String(key);
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

  return (
    <Modal.Backdrop isOpen={paletteOpen} onOpenChange={(open) => (open ? shell.setPalette(true) : close())}>
      <Modal.Container placement="top" size="lg" className="pt-[14vh]">
        <Modal.Dialog aria-label={t("palette.title")} className="overflow-hidden p-0">
          <Autocomplete inputValue={query} onInputChange={setQuery} filter={(text, input) => contains(text, input.trim())}>
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
              className="max-h-[min(26rem,60vh)] overflow-y-auto p-2"
              renderEmptyState={() => <p className="px-3 py-8 text-center text-sm text-fg-muted">{t("palette.empty")}</p>}
            >
              {actions.length > 0 ? (
                <ListBox.Section>
                  <Header className="px-2 pt-2 pb-1 text-xs font-mid text-fg-muted">{t("palette.actions")}</Header>
                  {actions.map((command) => {
                    const Icon = command.icon;
                    const label = command.queryTitleKey && query.trim()
                      ? t(command.queryTitleKey, { query: query.trim() })
                      : t(command.titleKey);
                    return (
                      <ListBox.Item key={command.id} id={`do:${command.id}`} textValue={`${label} ${(command.keywords ?? []).join(" ")}`}>
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
