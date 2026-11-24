import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

/** The standard page column: centred to a comfortable desktop measure, with the skin's gutter. */
export function PageFrame({ className, children, ...props }: ComponentProps<"div">) {
  return (
    <div className={cn("pv-enter mx-auto flex w-full max-w-[76rem] flex-col gap-(--pv-gap-section) px-(--pv-gutter) pt-7 pb-14 max-md:px-4 max-md:pt-5", className)} {...props}>
      {children}
    </div>
  );
}
