import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

/** Unified page column: container-query root aligned to the app's fluid width and horizontal gutter. */
export function PageFrame({ className, children, ...props }: ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "pv-enter pv-page-frame mx-auto flex w-full max-w-(--pv-layout-max) flex-col gap-(--pv-gap-section) px-(--pv-layout-gutter) pt-(--pv-layout-top) pb-(--pv-layout-bottom)",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
