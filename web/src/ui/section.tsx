/**
 * Sections: a titled block of a page. How sections are separated is the `section` slot:
 * `panel` draws each one as its own surface, `rule` puts a line above it, `plain` relies on
 * space and zebra rows. Rows inside a section ask `useSectionVariant` to match.
 */
import { Link, type LinkProps } from "@tanstack/react-router";
import { ChevronRight } from "lucide-react";
import { createContext, use, type ComponentProps, type ReactNode } from "react";
import { cn } from "@/lib/cn";
import { useSlot } from "@/skins/scope";
import type { Slots } from "@/skins/schema";

const SectionContext = createContext<Slots["section"]>("panel");

export function useSectionVariant(): Slots["section"] {
  return use(SectionContext);
}

export interface SectionLink {
  label: string;
  to: LinkProps["to"];
  search?: LinkProps["search"];
}

export function SectionMore({ link, className }: { link: SectionLink; className?: string }) {
  return (
    <Link
      to={link.to}
      search={link.search}
      className={cn(
        "-my-1 inline-flex shrink-0 items-center gap-0.5 rounded-control py-1 ps-2 pe-1 text-sm text-link transition-colors hover:bg-hover hover:text-fg",
        className,
      )}
    >
      {link.label}
      <ChevronRight className="size-3.5" aria-hidden />
    </Link>
  );
}

interface SectionProps extends Omit<ComponentProps<"section">, "title"> {
  title: ReactNode;
  /** Shown after the title (how many things the section holds). */
  count?: number;
  more?: SectionLink;
  /** Extra controls in the header, before the "more" link. */
  actions?: ReactNode;
  /** Visually hide the header (the region keeps its accessible name). */
  bare?: boolean;
  /** Lift the section off the page (a raised surface): what needs the user stands out. */
  raised?: boolean;
  /** The items are tiles of their own: never frame them in a panel (no cards in cards). */
  unframed?: boolean;
  bodyClassName?: string;
}

export function Section({ title, count, more, actions, bare = false, raised = false, unframed = false, className, bodyClassName, children, ...props }: SectionProps) {
  const slot = useSlot("section");
  // A raised section is drawn as its own surface whatever the separation style; tiles never sit in a panel.
  const variant = raised ? "panel" : unframed && slot === "panel" ? "plain" : slot;
  const label = typeof title === "string" ? title : undefined;
  return (
    <SectionContext value={variant}>
      <section
        aria-label={label}
        data-section={variant}
        className={cn(
          "min-w-0",
          variant === "panel" && !raised && "overflow-hidden rounded-panel border border-line bg-surface shadow-panel",
          raised && "overflow-hidden rounded-panel border border-line bg-raised shadow-raised",
          variant === "rule" && "border-t border-line-strong",
          className,
        )}
        {...props}
      >
        <header
          className={cn(
            "flex min-h-11 items-center gap-2",
            variant === "panel" && "px-(--pv-pad-panel) pt-4 pb-3",
            variant === "rule" && "pt-3.5 pb-2.5",
            variant === "plain" && "pb-2",
            bare && "sr-only",
          )}
        >
          <h2 className="truncate font-heading text-lg font-title text-heading">{title}</h2>
          {count !== undefined ? (
            <span className="text-sm font-mid text-fg-subtle tabular-nums">
              {count}
            </span>
          ) : null}
          <span className="flex-1" />
          {actions}
          {more ? <SectionMore link={more} /> : null}
        </header>
        <div className={bodyClassName}>{children}</div>
      </section>
    </SectionContext>
  );
}

/** Horizontal padding a row needs inside the current section variant. */
export function useRowInset(): string {
  const variant = useSectionVariant();
  return variant === "panel" ? "px-(--pv-pad-panel)" : variant === "plain" ? "px-3" : "";
}

/** A list of rows separated the way the section variant separates them. */
export function SectionRows({ className, children, ...props }: ComponentProps<"ul">) {
  const variant = useSectionVariant();
  return (
    <ul
      className={cn(
        variant === "plain" ? "grid gap-0.5 [&>li:nth-child(odd)]:bg-sunken [&>li]:rounded-tile" : "divide-y divide-line",
        variant === "panel" && "border-t border-line",
        className,
      )}
      {...props}
    >
      {children}
    </ul>
  );
}

/** A small label above a group of rows (e.g. "最近完成"). */
export function RowGroupLabel({ children, className }: { children: ReactNode; className?: string }) {
  const inset = useRowInset();
  return <div className={cn("pt-3 pb-1 text-xs font-medium text-fg-subtle", inset, className)}>{children}</div>;
}
