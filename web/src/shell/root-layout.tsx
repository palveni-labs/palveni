import { Button, RouterProvider, Toast } from "@heroui/react";
import { Outlet, useRouter, useRouterState, type ErrorComponentProps } from "@tanstack/react-router";
import { RotateCw, ServerOff } from "lucide-react";
import { useTranslation } from "react-i18next";
import { describeError } from "@/lib/errors";

function NavigationProgress() {
  const loading = useRouterState({ select: (state) => state.status === "pending" });
  if (!loading) return null;
  return (
    <div className="pointer-events-none fixed inset-x-0 top-0 z-[60] h-0.5 overflow-hidden" aria-hidden>
      <div className="pv-sweep h-full w-1/3 bg-active-mark" />
    </div>
  );
}

export function RootLayout() {
  const router = useRouter();
  // HeroUI links and menu items with `href` navigate through the app router (no page reloads).
  return (
    <RouterProvider navigate={(to) => void router.navigate({ to })} useHref={(to) => router.buildLocation({ to }).href}>
      <NavigationProgress />
      <Outlet />
      <Toast.Provider placement="bottom end" />
    </RouterProvider>
  );
}

/** First paint while the server info loads. */
export function Splash() {
  const { t } = useTranslation();
  return (
    <div className="flex min-h-svh items-center justify-center bg-page" role="status" aria-label={t("common.loading")}>
      <span className="animate-pulse font-heading text-xl font-title text-fg-muted">{t("app.name")}</span>
    </div>
  );
}

/** The server could not be reached (or answered with an error) while bootstrapping. */
export function BootError({ error }: ErrorComponentProps) {
  const { t } = useTranslation();
  const router = useRouter();
  const { title, detail } = describeError(t, error);
  return (
    <div className="flex min-h-svh items-center justify-center bg-page p-6">
      <div role="alert" className="flex w-full max-w-sm flex-col items-center gap-4 rounded-panel border border-line bg-surface p-8 text-center">
        <span className="grid size-11 place-items-center rounded-tile bg-failed-soft text-failed">
          <ServerOff className="size-5" aria-hidden />
        </span>
        <div className="grid gap-1">
          <h1 className="text-lg font-strong text-fg">{t("boot.unreachable")}</h1>
          <p className="text-sm text-fg-muted">{detail ?? title}</p>
          <p className="text-sm text-fg-muted">{t("boot.recover")}</p>
        </div>
        <Button variant="primary" onPress={() => void router.invalidate()}>
          <RotateCw aria-hidden />
          {t("common.retry")}
        </Button>
      </div>
    </div>
  );
}
