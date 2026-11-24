import { Button, Chip, FieldError, Input, Label, Modal, Spinner, Switch, TextField, toast } from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertCircle, KeyRound, Laptop } from "lucide-react";
import { useId, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import { ApiError } from "@/api/errors";
import { useCurrentUser } from "@/api/queries/auth";
import { queryKeys } from "@/api/query-keys";
import { RelativeTime } from "@/components/relative-time";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { useFeature } from "@/modules/access";
import { useSecureSurface } from "@/skins/secure";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { ErrorState, LoadingRows } from "@/ui/states";

const MIN_PASSWORD_LENGTH = 8;

interface PasswordErrors {
  current?: string;
  next?: string;
  confirm?: string;
  form?: string;
}

function ChangePasswordDialog({
  open,
  onOpenChange,
  userId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  userId: string;
}) {
  const { t } = useTranslation();
  const formId = useId();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [revokeOtherSessions, setRevokeOtherSessions] = useState(true);
  const [errors, setErrors] = useState<PasswordErrors>({});

  const change = useMutation({
    mutationFn: () =>
      api.auth.changePassword({
        currentPassword,
        newPassword,
        revokeOtherSessions,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.auth.sessions(userId) });
      void queryClient.invalidateQueries({ queryKey: queryKeys.users.all });
    },
  });

  const reset = () => {
    setCurrentPassword("");
    setNewPassword("");
    setConfirmPassword("");
    setRevokeOtherSessions(true);
    setErrors({});
    change.reset();
  };

  const close = () => {
    onOpenChange(false);
    reset();
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    const nextErrors: PasswordErrors = {};
    if (!currentPassword) {
      nextErrors.current = t("accountSecurity.currentPasswordRequired");
    }
    if ([...newPassword].length < MIN_PASSWORD_LENGTH) {
      nextErrors.next = t("validation.password.tooShort", { min: MIN_PASSWORD_LENGTH });
    }
    if (newPassword !== confirmPassword) {
      nextErrors.confirm = t("validation.password.mismatch");
    }
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;

    change.mutate(undefined, {
      onSuccess: () => {
        toast.success(t("accountSecurity.passwordChanged"));
        close();
      },
      onError: (error) => {
        const text = errorText(t, error);
        if (error instanceof ApiError && (error.code === "invalid_credentials" || error.details?.field === "currentPassword")) {
          setErrors({ current: t("login.invalidCredentials") });
          return;
        }
        if (error instanceof ApiError && error.details?.field === "newPassword") {
          setErrors({ next: text });
          return;
        }
        setErrors({ form: text });
      },
    });
  };

  return (
    <Modal.Backdrop
      isOpen={open}
      onOpenChange={(next) => {
        onOpenChange(next);
        if (!next) reset();
      }}
      isDismissable={!change.isPending}
    >
      <Modal.Container size="sm" scroll="inside">
        <Modal.Dialog {...secure} className="gap-0 p-0">
          <Modal.Header className="grid gap-1 px-6 pt-6 pb-4">
            <Modal.Heading className="font-heading text-lg font-title text-heading">
              {t("accountSecurity.changePassword")}
            </Modal.Heading>
          </Modal.Header>
          <Modal.Body className="px-6 pb-2">
            <form id={formId} className="grid gap-4" onSubmit={onSubmit} noValidate>
              <TextField
                value={currentPassword}
                onChange={setCurrentPassword}
                isInvalid={Boolean(errors.current)}
                autoFocus
                className="grid gap-1.5"
              >
                <Label className="text-sm font-mid text-fg">{t("accountSecurity.currentPassword")}</Label>
                <Input type="password" autoComplete="current-password" />
                {errors.current ? <FieldError className="text-sm text-failed">{errors.current}</FieldError> : null}
              </TextField>

              <TextField
                value={newPassword}
                onChange={setNewPassword}
                isInvalid={Boolean(errors.next)}
                className="grid gap-1.5"
              >
                <Label className="text-sm font-mid text-fg">{t("accountSecurity.newPassword")}</Label>
                <Input type="password" autoComplete="new-password" />
                {errors.next ? <FieldError className="text-sm text-failed">{errors.next}</FieldError> : null}
              </TextField>

              <TextField
                value={confirmPassword}
                onChange={setConfirmPassword}
                isInvalid={Boolean(errors.confirm)}
                className="grid gap-1.5"
              >
                <Label className="text-sm font-mid text-fg">{t("accountSecurity.confirmPassword")}</Label>
                <Input type="password" autoComplete="new-password" />
                {errors.confirm ? <FieldError className="text-sm text-failed">{errors.confirm}</FieldError> : null}
              </TextField>

              <Switch isSelected={revokeOtherSessions} onChange={setRevokeOtherSessions}>
                <Switch.Content className="gap-3 text-sm font-body text-fg">
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  {t("accountSecurity.revokeOtherSessions")}
                </Switch.Content>
              </Switch>

              {errors.form ? (
                <p role="alert" className="flex items-start gap-2 rounded-tile bg-failed-soft px-3 py-2.5 text-sm text-fg">
                  <AlertCircle className="mt-0.5 size-4 shrink-0 text-failed" aria-hidden />
                  {errors.form}
                </p>
              ) : null}
            </form>
          </Modal.Body>
          <Modal.Footer className="flex justify-end gap-2 px-6 pt-4 pb-6">
            <Button variant="secondary" onPress={close} isDisabled={change.isPending}>
              {t("common.cancel")}
            </Button>
            <Button type="submit" form={formId} variant="primary" isPending={change.isPending}>
              {change.isPending ? <Spinner size="sm" color="current" /> : null}
              {t("settings.save")}
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

export function AccountSecurity() {
  const { t } = useTranslation();
  const enabled = useFeature("users");
  const user = useCurrentUser();
  const secure = useSecureSurface();
  const inset = useRowInset();
  const queryClient = useQueryClient();
  const [passwordOpen, setPasswordOpen] = useState(false);

  const sessionsKey = queryKeys.auth.sessions(user.id);
  const sessionsQuery = useQuery({
    queryKey: sessionsKey,
    queryFn: ({ signal }) => api.auth.sessions(signal),
    enabled,
  });

  const revoke = useMutation({
    mutationFn: (sessionId: string) => api.auth.revokeSession(sessionId),
    onSuccess: () => {
      toast.success(t("accountSecurity.sessionRevoked"));
      void queryClient.invalidateQueries({ queryKey: sessionsKey });
      void queryClient.invalidateQueries({ queryKey: queryKeys.users.all });
    },
    onError: (error) => {
      toast.danger(errorText(t, error));
    },
  });

  if (!enabled) return null;

  const sessions = sessionsQuery.data?.sessions ?? [];

  return (
    <Section title={t("accountSecurity.title")}>
      <SectionRows>
        <li {...secure} className={cn("flex flex-wrap items-center justify-between gap-x-6 gap-y-2.5 py-3.5", inset)}>
          <div className="grid min-w-0 gap-0.5">
            <span className="text-base text-fg">{t("accountSecurity.password")}</span>
            <span className="text-sm text-fg-muted">{t("accountSecurity.passwordHint")}</span>
          </div>
          <Button variant="outline" size="sm" onPress={() => setPasswordOpen(true)}>
            <KeyRound aria-hidden />
            {t("accountSecurity.changePassword")}
          </Button>
        </li>

        <li {...secure} className={cn("grid gap-2.5 py-3.5", inset)}>
          <span className="text-base text-fg">{t("accountSecurity.sessions")}</span>
          {sessionsQuery.isPending ? (
            <LoadingRows rows={2} />
          ) : sessionsQuery.isError ? (
            <ErrorState compact error={sessionsQuery.error} onRetry={() => void sessionsQuery.refetch()} />
          ) : sessions.length === 0 ? (
            <span className="text-sm text-fg-muted">{t("accountSecurity.sessionsEmpty")}</span>
          ) : (
            <ul className="grid divide-y divide-line">
              {sessions.map((session) => (
                <li key={session.id} className="flex flex-wrap items-center justify-between gap-3 py-2.5">
                  <div className="flex min-w-0 items-start gap-2.5">
                    <Laptop className="mt-0.5 size-4 shrink-0 text-icon" aria-hidden />
                    <div className="grid min-w-0 gap-0.5">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-strong text-fg">{session.deviceName}</span>
                        {session.current ? (
                          <Chip size="sm" variant="soft" className="bg-ok-soft text-ok">
                            {t("accountSecurity.currentSession")}
                          </Chip>
                        ) : null}
                      </div>
                      <span className="text-xs text-fg-muted">
                        {t("accountSecurity.lastActive")}
                        <RelativeTime value={session.lastUsedAt} />
                      </span>
                    </div>
                  </div>
                  {!session.current ? (
                    <Button
                      size="sm"
                      variant="outline"
                      isDisabled={revoke.isPending}
                      onPress={() => revoke.mutate(session.id)}
                    >
                      {t("accountSecurity.revokeSession")}
                    </Button>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </li>
      </SectionRows>

      <ChangePasswordDialog open={passwordOpen} onOpenChange={setPasswordOpen} userId={user.id} />
    </Section>
  );
}
