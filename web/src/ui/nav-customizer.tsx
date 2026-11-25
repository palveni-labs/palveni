/**
 * User navigation item organizer: drag-and-drop or step buttons to reorder modules, plus
 * show/hide toggles (core modules stay locked visible so settings remain reachable).
 */
import { Button } from "@heroui/react";
import { ArrowDown, ArrowUp, Eye, EyeOff, GripVertical, Lock, RotateCcw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import {
  isNavCustomized,
  LOCKED_NAV_MODULES,
  moveNavModule,
  reorderNavModule,
  resetNavPrefs,
  toggleNavModuleHidden,
  useNavPrefs,
} from "@/shell/nav-prefs";
import { useAccessibleModules } from "@/shell/use-shell-modules";

export function NavCustomizer({ className }: { className?: string }) {
  const { t } = useTranslation();
  const modules = useAccessibleModules();
  const prefs = useNavPrefs();
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);
  const allIds = modules.map((module) => module.id);
  const hidden = new Set(prefs.hidden);
  const customized = isNavCustomized(prefs);

  return (
    <div className={cn("grid gap-2.5", className)}>
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-mid text-fg-muted">{t("nav.customizerTitle")}</span>
        {customized ? (
          <Button size="sm" variant="ghost" onPress={resetNavPrefs}>
            <RotateCcw className="size-3.5" aria-hidden />
            {t("nav.resetOrder")}
          </Button>
        ) : null}
      </div>
      <ul
        className="divide-y divide-line/65 overflow-hidden rounded-tile border border-line bg-surface"
        aria-label={t("nav.customizerTitle")}
      >
        {modules.map((module, index) => {
          const Icon = module.icon;
          const locked = LOCKED_NAV_MODULES.has(module.id);
          const isHidden = !locked && hidden.has(module.id);
          const label = t(module.titleKey);
          return (
            <li
              key={module.id}
              draggable
              onDragStart={(event) => {
                setDraggingId(module.id);
                event.dataTransfer.effectAllowed = "move";
                event.dataTransfer.setData("text/plain", module.id);
              }}
              onDragOver={(event) => {
                event.preventDefault();
                event.dataTransfer.dropEffect = "move";
                if (overId !== module.id) setOverId(module.id);
              }}
              onDragLeave={() => {
                if (overId === module.id) setOverId(null);
              }}
              onDrop={(event) => {
                event.preventDefault();
                const from = draggingId;
                if (from && from !== module.id) reorderNavModule(from, module.id, allIds);
                setDraggingId(null);
                setOverId(null);
              }}
              onDragEnd={() => {
                setDraggingId(null);
                setOverId(null);
              }}
              className={cn(
                "flex items-center gap-2.5 px-3 py-2 text-sm transition-colors hover:bg-hover/55",
                draggingId === module.id && "opacity-50",
                overId === module.id && draggingId !== module.id && "bg-active-soft/70",
              )}
            >
              <GripVertical className="size-3.5 shrink-0 cursor-grab text-fg-subtle" aria-hidden />
              <Icon className="size-4 shrink-0 text-icon" aria-hidden />
              <span className={cn("min-w-0 flex-1 truncate font-mid text-fg", isHidden && "line-through text-fg-muted")}>{label}</span>
              <div className="flex shrink-0 items-center gap-0.5">
                <button
                  type="button"
                  disabled={index === 0}
                  onClick={() => moveNavModule(module.id, -1, allIds)}
                  aria-label={t("nav.moveUp", { name: label })}
                  className="grid size-7 place-items-center rounded-control text-fg-muted outline-none transition-colors hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus disabled:pointer-events-none disabled:opacity-30"
                >
                  <ArrowUp className="size-3.5" aria-hidden />
                </button>
                <button
                  type="button"
                  disabled={index === modules.length - 1}
                  onClick={() => moveNavModule(module.id, 1, allIds)}
                  aria-label={t("nav.moveDown", { name: label })}
                  className="grid size-7 place-items-center rounded-control text-fg-muted outline-none transition-colors hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus disabled:pointer-events-none disabled:opacity-30"
                >
                  <ArrowDown className="size-3.5" aria-hidden />
                </button>
                {locked ? (
                  <span
                    className="grid size-7 place-items-center text-fg-subtle"
                    title={t("nav.lockedItem")}
                    aria-label={t("nav.lockedItem")}
                  >
                    <Lock className="size-3.5" aria-hidden />
                  </span>
                ) : (
                  <button
                    type="button"
                    onClick={() => toggleNavModuleHidden(module.id)}
                    aria-pressed={!isHidden}
                    aria-label={isHidden ? t("nav.showItem", { name: label }) : t("nav.hideItem", { name: label })}
                    className="grid size-7 place-items-center rounded-control text-fg-muted outline-none transition-colors hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-focus"
                  >
                    {isHidden ? <EyeOff className="size-3.5" aria-hidden /> : <Eye className="size-3.5" aria-hidden />}
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
