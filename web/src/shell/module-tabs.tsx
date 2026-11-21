import { Link } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useAccess } from "@/modules/access";
import { isVisible } from "@/modules/registry";
import { useActiveModule } from "./use-shell-modules";

/**
 * Sub-pages of the active module on small screens only. Desktop navigation exposes secondary pages
 * directly through collapsible sidebar groups or topbar dropdown menus, avoiding double top bars.
 */
export function ModuleTabs() {
  const { t } = useTranslation();
  const access = useAccess();
  const { module, entry: active } = useActiveModule();
  const entries = (module?.nav ?? []).filter((entry) => isVisible(entry, access));
  if (entries.length < 2) return null;
  return (
    <nav aria-label={module ? t(module.titleKey) : undefined} className="mx-auto w-full max-w-[76rem] px-4 pt-4 md:hidden">
      <ul className="no-scrollbar flex w-full gap-1 overflow-x-auto border-b border-line">
        {entries.map((entry) => {
          const Icon = entry.icon;
          const selected = active?.to === entry.to;
          return (
            <li key={entry.to}>
              <Link
                to={entry.to}
                aria-current={selected ? "page" : undefined}
                className={cn(
                  "relative flex h-10 shrink-0 items-center gap-1.5 px-3 text-sm whitespace-nowrap outline-none transition-colors focus-visible:ring-2 focus-visible:ring-focus",
                  selected ? "font-strong text-fg" : "text-fg-muted hover:text-fg",
                )}
              >
                {Icon ? <Icon className="size-4" aria-hidden /> : null}
                {t(entry.titleKey)}
                {selected ? <span aria-hidden className="absolute inset-x-2 -bottom-px h-0.5 bg-fg" /> : null}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
