import { buttonVariants, Dropdown, Label } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { Check, ChevronDown, FolderPlus, Library as LibraryIcon, Music } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useIsAdmin } from "@/api/queries/auth";
import type { Library } from "@/api/types";
import { EmptyState, LoadingRows } from "@/ui/states";
import { useMusicLibraries } from "./library-store";

/** Compact dropdown switcher shown only when there are 2+ music libraries. */
export function MusicLibrarySwitcher({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { libraries, library, selectLibrary } = useMusicLibraries();
  if (libraries.length < 2 || !library) return null;

  return (
    <Dropdown>
      <Dropdown.Trigger
        aria-label={t("musicApp.library.switch")}
        className={`inline-flex h-(--pv-control-h-sm) items-center gap-1.5 rounded-control border border-line-control bg-surface px-2.5 text-sm font-mid text-fg outline-none transition-colors hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus ${className ?? ""}`}
      >
        <LibraryIcon className="size-3.5 text-icon" aria-hidden />
        <span className="max-w-36 truncate">{library.name}</span>
        <ChevronDown className="size-3.5 text-fg-muted" aria-hidden />
      </Dropdown.Trigger>
      <Dropdown.Popover placement="bottom end" className="min-w-48">
        <Dropdown.Menu
          aria-label={t("musicApp.library.switch")}
          onAction={(key) => selectLibrary(String(key))}
        >
          {libraries.map((lib) => (
            <Dropdown.Item key={lib.id} id={lib.id} textValue={lib.name}>
              <Music className="size-4 text-fg-muted" aria-hidden />
              <Label className="min-w-0 flex-1 truncate">{lib.name}</Label>
              {lib.id === library.id ? <Check className="size-3.5 text-fg" aria-hidden /> : null}
            </Dropdown.Item>
          ))}
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}

/** Renders loading or empty state when no music library exists, otherwise hands the active library to children. */
export function MusicLibraryGuard({ children }: { children: (library: Library) => ReactNode }) {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const { library, isPending } = useMusicLibraries();

  if (isPending) return <LoadingRows rows={6} />;
  if (!library) {
    return (
      <EmptyState
        icon={Music}
        title={t("musicApp.library.empty")}
        detail={isAdmin ? t("musicApp.library.emptyAdmin") : t("musicApp.library.emptyUser")}
        action={
          isAdmin ? (
            <Link to="/system/libraries" search={{ create: true }} className={buttonVariants({ variant: "primary", size: "sm" })}>
              <FolderPlus aria-hidden />
              {t("adminLibraries.create")}
            </Link>
          ) : null
        }
      />
    );
  }
  return <>{children(library)}</>;
}
