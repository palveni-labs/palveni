/**
 * First-run wizard (the server has no users yet): welcome → administrator account → appearance →
 * done. The account step is the only one that talks to the server; the appearance step applies
 * at once and can be changed later in 应用 › 外观.
 */
import { Button, buttonVariants, Spinner } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { AlertCircle, ArrowLeft, ArrowRight, Check, FolderPlus, LayoutDashboard, LogIn } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { isApiError } from "@/api/errors";
import { useSetupAdmin } from "@/api/queries/auth";
import { useSystemInfo } from "@/api/queries/system";
import { currentLanguage } from "@/i18n";
import { errorText } from "@/lib/errors";
import { ModeSetting } from "@/ui/mode-setting";
import { useModeSummary } from "@/ui/use-mode-summary";
import { SkinPreviewPair } from "@/ui/skin-preview";
import { BUILTIN_SKIN } from "@/skins/registry";
import { localized } from "@/skins/schema";
import { useSkin } from "@/skins/store";
import { StatusLabel } from "@/ui/marks";
import { AuthCard, AuthLayout } from "./auth-layout";
import { TextField } from "./form-field";
import { checkPassword, checkUsername, PASSWORD_MIN, USERNAME_MAX } from "./validation";

type Step = "welcome" | "account" | "appearance" | "done";
const STEPS: Step[] = ["welcome", "account", "appearance", "done"];

/** Numbered steps with their names; finished steps carry a check, the current one is announced as the current step. */
function Stepper({ current }: { current: Step }) {
  const { t } = useTranslation();
  const currentIndex = STEPS.indexOf(current);
  return (
    <ol className="mb-7 grid grid-cols-4" aria-label={t("setup.progress", { current: currentIndex + 1, total: STEPS.length })}>
      {STEPS.map((step, index) => {
        const finished = current === "done";
        const done = index < currentIndex || finished;
        const active = index === currentIndex && !finished;
        return (
          <li key={step} className="relative flex min-w-0 flex-col items-center gap-1.5 text-center" aria-current={index === currentIndex ? "step" : undefined}>
            {index > 0 ? (
              <span aria-hidden className={cn("absolute top-3 right-1/2 h-px w-full", index <= currentIndex ? "bg-selected" : "bg-line-strong")} />
            ) : null}
            <span
              className={cn(
                "relative grid size-6 shrink-0 place-items-center rounded-full text-xs font-strong tabular-nums",
                done && "bg-selected text-selected-fg",
                active && "border-2 border-selected bg-surface text-fg",
                !done && !active && "border border-line-strong bg-surface text-fg-muted",
              )}
            >
              {done ? <Check className="size-3.5" strokeWidth={3} aria-hidden /> : index + 1}
            </span>
            <span className={cn("max-w-full truncate text-xs", active ? "font-strong text-fg" : done ? "text-fg" : "text-fg-muted")}>
              {t(`setup.steps.${step}`)}
              {done ? <span className="sr-only"> {t("setup.stepDone")}</span> : null}
            </span>
          </li>
        );
      })}
    </ol>
  );
}

/** The step's title. Later steps take focus when they appear, so the change is announced. */
function StepHeading({ title, lead, focusOnMount = false }: { title: string; lead: ReactNode; focusOnMount?: boolean }) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (focusOnMount) heading.current?.focus();
  }, [focusOnMount]);
  return (
    <div className="grid gap-1.5">
      <h1 ref={heading} tabIndex={-1} className="font-heading text-xl font-title text-heading outline-none">
        {title}
      </h1>
      <p className="text-base text-fg-muted">{lead}</p>
    </div>
  );
}

function WelcomeStep({ onNext }: { onNext: () => void }) {
  const { t } = useTranslation();
  const { data: info } = useSystemInfo();
  return (
    <div className="grid grid-cols-[minmax(0,1fr)] gap-6">
      <StepHeading title={t("setup.welcome.title")} lead={t("setup.welcome.lead")} />
      {info ? (
        <dl className="grid divide-y divide-line rounded-tile border border-line text-sm">
          {[
            { label: t("setup.welcome.serverName"), value: info.name, id: false },
            { label: t("setup.welcome.version"), value: info.version, id: false },
            { label: t("setup.welcome.serverId"), value: info.serverId, id: true },
          ].map((row) => (
            <div key={row.label} className="flex items-center justify-between gap-4 px-3.5 py-2.5">
              <dt className="shrink-0 text-fg-muted">{row.label}</dt>
              <dd className={cn("min-w-0 truncate text-end text-fg", row.id ? "font-mono text-xs" : "tabular-nums")} title={row.value}>
                {row.value}
              </dd>
            </div>
          ))}
        </dl>
      ) : null}
      <Button size="lg" variant="primary" fullWidth onPress={onNext}>
        {t("setup.welcome.start")}
        <ArrowRight aria-hidden />
      </Button>
    </div>
  );
}

function AccountStep({ onBack, onDone }: { onBack: () => void; onDone: (username: string) => void }) {
  const { t } = useTranslation();
  const setup = useSetupAdmin();
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [submitted, setSubmitted] = useState(false);

  const usernameProblem = checkUsername(username);
  const passwordProblem = checkPassword(password);
  const mismatch = confirm !== password;

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    setSubmitted(true);
    if (usernameProblem || passwordProblem || mismatch) return;
    setup.mutate({ username, password }, { onSuccess: () => onDone(username) });
  };

  const alreadyDone = isApiError(setup.error, "setup_already_completed");

  return (
    <form className="grid grid-cols-[minmax(0,1fr)] gap-6" onSubmit={onSubmit} noValidate>
      <StepHeading title={t("setup.account.title")} lead={t("setup.account.lead")} />
      <div className="grid gap-4">
        <TextField
          label={t("auth.username")}
          name="username"
          autoComplete="username"
          spellCheck="false"
          autoCorrect="off"
          value={username}
          maxLength={USERNAME_MAX * 2}
          onChange={setUsername}
          error={submitted && usernameProblem ? t(`validation.username.${usernameProblem}`, { max: USERNAME_MAX }) : null}
          hint={t("setup.account.usernameHint", { max: USERNAME_MAX })}
        />
        <TextField
          label={t("auth.password")}
          name="new-password"
          type="password"
          autoComplete="new-password"
          autoFocus
          value={password}
          onChange={setPassword}
          error={submitted && passwordProblem ? t("validation.password.tooShort", { min: PASSWORD_MIN }) : null}
          hint={t("setup.account.passwordHint", { min: PASSWORD_MIN })}
        />
        <TextField
          label={t("setup.account.confirmPassword")}
          name="confirm-password"
          type="password"
          autoComplete="new-password"
          value={confirm}
          onChange={setConfirm}
          error={submitted && mismatch ? t("validation.password.mismatch") : null}
        />
      </div>
      {setup.error ? (
        <div role="alert" className="flex items-start gap-2.5 rounded-tile bg-failed-soft px-3.5 py-2.5 text-sm text-failed">
          <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden />
          <div className="grid gap-1.5">
            <p>{alreadyDone ? t("setup.account.alreadyDone") : t("setup.account.failed", { reason: errorText(t, setup.error) })}</p>
            {alreadyDone ? (
              <Link to="/login" className="inline-flex w-fit items-center gap-1 font-strong underline underline-offset-4">
                <LogIn className="size-3.5" aria-hidden />
                {t("setup.account.goToLogin")}
              </Link>
            ) : null}
          </div>
        </div>
      ) : null}
      <div className="flex items-center gap-2">
        <Button type="button" variant="ghost" size="lg" onPress={onBack} isDisabled={setup.isPending}>
          <ArrowLeft aria-hidden />
          {t("common.back")}
        </Button>
        <Button type="submit" variant="primary" size="lg" className="flex-1" isPending={setup.isPending}>
          {setup.isPending ? <Spinner size="sm" color="current" /> : null}
          {t("setup.account.submit")}
        </Button>
      </div>
    </form>
  );
}

/** Nothing is installed on a fresh server: the built-in skin, and the light/dark choice. */
function AppearanceStep({ onNext }: { onNext: () => void }) {
  const { t } = useTranslation();
  const language = currentLanguage();
  const { active, mode, setSkin } = useSkin();
  const modeSummary = useModeSummary();
  const manifest = BUILTIN_SKIN.manifest;
  const name = localized(manifest.name, language);
  const inUse = active.manifest.id === manifest.id;
  return (
    <div className="grid grid-cols-[minmax(0,1fr)] gap-6">
      <StepHeading title={t("setup.appearance.title")} lead={t("setup.appearance.lead")} focusOnMount />
      <div className="grid gap-3">
        <SkinPreviewPair entry={BUILTIN_SKIN} />
        <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
          <div className="grid min-w-0 flex-1 basis-48 gap-0.5">
            <p className="font-strong text-fg">{name}</p>
            <p className="text-sm text-fg-muted">{localized(manifest.description, language)}</p>
          </div>
          {inUse ? (
            <StatusLabel tone="ok" className="text-sm">
              {t("appearance.inUse")}
            </StatusLabel>
          ) : (
            <Button size="sm" variant="outline" onPress={() => void setSkin(manifest.id).catch(() => undefined)}>
              {t("setup.appearance.useBuiltin", { name })}
            </Button>
          )}
        </div>
      </div>
      <div className="grid gap-2">
        <p className="text-sm font-mid text-fg">{t("appearance.mode.title")}</p>
        <ModeSetting className="w-full [&>.toggle-button]:flex-1" />
        {mode === "system" ? <p className="text-sm text-fg-muted">{modeSummary}</p> : null}
      </div>
      <p className="text-sm text-fg-muted">{t("setup.appearance.later")}</p>
      <Button size="lg" variant="primary" fullWidth onPress={onNext}>
        {t("setup.appearance.next")}
        <ArrowRight aria-hidden />
      </Button>
    </div>
  );
}

function DoneStep({ username }: { username: string }) {
  const { t } = useTranslation();
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => heading.current?.focus(), []);
  return (
    <div className="grid grid-cols-[minmax(0,1fr)] gap-6">
      <div className="flex flex-col items-center gap-3 pt-2 text-center">
        <span className="grid size-12 place-items-center rounded-full bg-ok-soft text-ok">
          <Check className="size-6" strokeWidth={2.5} aria-hidden />
        </span>
        <div className="grid gap-1">
          <h1 ref={heading} tabIndex={-1} className="font-heading text-xl font-title text-heading outline-none">
            {t("setup.done.title")}
          </h1>
          <p className="text-base text-fg-muted">{t("setup.done.signedInAs", { username })}</p>
        </div>
      </div>
      <div className="grid gap-2">
        <Link to="/system/libraries" search={{ create: true }} className={buttonVariants({ variant: "primary", size: "lg", fullWidth: true })}>
          <FolderPlus aria-hidden />
          {t("setup.done.addLibrary")}
        </Link>
        <Link to="/" className={buttonVariants({ variant: "ghost", size: "lg", fullWidth: true })}>
          <LayoutDashboard aria-hidden />
          {t("setup.done.openOverview")}
        </Link>
      </div>
    </div>
  );
}

export function SetupPage() {
  const [step, setStep] = useState<Step>("welcome");
  const [username, setUsername] = useState("");
  return (
    <AuthLayout>
      <AuthCard>
        <Stepper current={step} />
        {step === "welcome" ? <WelcomeStep onNext={() => setStep("account")} /> : null}
        {step === "account" ? (
          <AccountStep
            onBack={() => setStep("welcome")}
            onDone={(name) => {
              setUsername(name);
              setStep("appearance");
            }}
          />
        ) : null}
        {step === "appearance" ? <AppearanceStep onNext={() => setStep("done")} /> : null}
        {step === "done" ? <DoneStep username={username} /> : null}
      </AuthCard>
    </AuthLayout>
  );
}
