/**
 * Sign-in. Errors say what went wrong and what to do next; wrong credentials clear the password
 * and put the cursor back in it.
 */
import { Button, Spinner } from "@heroui/react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { AlertCircle, LogIn, Server } from "lucide-react";
import { useRef, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { isApiError } from "@/api/errors";
import { useLogin } from "@/api/queries/auth";
import { useSystemInfo } from "@/api/queries/system";
import { errorText } from "@/lib/errors";
import { AuthCard, AuthLayout } from "./auth-layout";
import { TextField } from "./form-field";

export function LoginPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const search = useSearch({ from: "/login" });
  const { data: info } = useSystemInfo();
  const login = useLogin();
  const form = useRef<HTMLFormElement>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [touched, setTouched] = useState(false);

  const missingUsername = touched && username.trim() === "";
  const missingPassword = touched && password === "";

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    setTouched(true);
    if (username.trim() === "" || password === "") return;
    login.mutate(
      { username: username.trim(), password },
      {
        onSuccess: () => {
          if (search.redirect) void navigate({ href: search.redirect, replace: true });
          else void navigate({ to: "/", replace: true });
        },
        onError: (error) => {
          if (!isApiError(error, "invalid_credentials")) return;
          // A fresh try: the empty password is not "missing" until the next submit.
          setTouched(false);
          setPassword("");
          form.current?.querySelector<HTMLInputElement>('input[name="password"]')?.focus();
        },
      },
    );
  };

  const wrongCredentials = isApiError(login.error, "invalid_credentials");
  const problem = login.error
    ? wrongCredentials
      ? { title: t("login.invalidCredentials"), recovery: t("login.invalidRecovery") }
      : { title: errorText(t, login.error), recovery: t("login.unreachableRecovery") }
    : null;

  return (
    <AuthLayout>
      <AuthCard>
        <div className="mb-6 grid gap-1.5">
          <h1 className="font-heading text-xl font-title text-heading">{t("login.title")}</h1>
          {info ? (
            <p className="flex min-w-0 items-center gap-1.5 text-base text-fg-muted">
              <Server className="size-4 shrink-0" aria-hidden />
              <span className="truncate">{t("login.server", { name: info.name })}</span>
            </p>
          ) : null}
        </div>
        <form ref={form} className="grid gap-4" onSubmit={onSubmit} noValidate aria-busy={login.isPending || undefined}>
          <TextField
            label={t("auth.username")}
            name="username"
            autoComplete="username"
            spellCheck="false"
            autoCorrect="off"
            autoFocus
            value={username}
            onChange={setUsername}
            error={missingUsername ? t("login.usernameRequired") : null}
          />
          <TextField
            label={t("auth.password")}
            name="password"
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={setPassword}
            error={missingPassword ? t("login.passwordRequired") : null}
          />
          <div aria-live="polite" className="empty:hidden">
            {problem ? (
              <div role="alert" className="flex items-start gap-2.5 rounded-tile bg-failed-soft px-3.5 py-2.5 text-sm text-failed">
                <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden />
                <div className="grid gap-0.5">
                  <p className="font-strong">{problem.title}</p>
                  <p>{problem.recovery}</p>
                </div>
              </div>
            ) : null}
          </div>
          <Button type="submit" size="lg" variant="primary" fullWidth className="mt-1" isPending={login.isPending}>
            {login.isPending ? <Spinner size="sm" color="current" /> : <LogIn aria-hidden />}
            {t("login.submit")}
          </Button>
        </form>
      </AuthCard>
    </AuthLayout>
  );
}
