import {
  AlertDialog,
  Button,
  Chip,
  FieldError,
  Input,
  Label,
  Modal,
  Radio,
  RadioGroup,
  Spinner,
  Switch,
  TextField,
  toast,
} from "@heroui/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertCircle, KeyRound, Pencil, Trash2, UserPlus, Users as UsersIcon } from "lucide-react";
import { useId, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api/endpoints";
import { ApiError } from "@/api/errors";
import { useCurrentUser } from "@/api/queries/auth";
import { useLibraries } from "@/api/queries/libraries";
import { queryKeys } from "@/api/query-keys";
import type { Library, ManagedUser, Role } from "@/api/types";
import { RelativeTime } from "@/components/relative-time";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { UserAvatar } from "@/shell/user-menu";
import { useSecureSurface } from "@/skins/secure";
import { PageFrame } from "@/ui/frame";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";

const MIN_PASSWORD_LENGTH = 8;
const MAX_USERNAME_LENGTH = 64;

function LibraryAclChecklist({
  libraries,
  selectedIds,
  onToggle,
}: {
  libraries: Library[];
  selectedIds: string[];
  onToggle: (id: string) => void;
}) {
  const { t } = useTranslation();
  if (libraries.length === 0) {
    return <p className="text-sm text-fg-muted">{t("libraries.emptyAdmin")}</p>;
  }
  return (
    <div className="grid gap-1.5 rounded-tile border border-line bg-sunken p-3">
      <span className="text-xs font-mid text-fg-muted">{t("adminUsers.allowedLibrariesLabel")}</span>
      <div className="grid gap-2 sm:grid-cols-2">
        {libraries.map((lib) => {
          const checked = selectedIds.includes(lib.id);
          return (
            <label key={lib.id} className="flex items-center gap-2 text-sm text-fg">
              <input
                type="checkbox"
                checked={checked}
                onChange={() => onToggle(lib.id)}
              />
              <span className="truncate">{lib.name}</span>
              <span className="text-xs text-fg-muted">({t(`kinds.${lib.kind}`)})</span>
            </label>
          );
        })}
      </div>
    </div>
  );
}

function CreateUserDialog({
  open,
  onOpenChange,
  libraries,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  libraries: Library[];
}) {
  const { t } = useTranslation();
  const formId = useId();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("user");
  const [allLibraries, setAllLibraries] = useState(true);
  const [allowedIds, setAllowedIds] = useState<string[]>([]);
  const [allowDownload, setAllowDownload] = useState(true);
  const [allowTranscode, setAllowTranscode] = useState(true);
  const [dailyAiLimit, setDailyAiLimit] = useState("100");
  const [errors, setErrors] = useState<{ username?: string; password?: string; form?: string }>({});

  const create = useMutation({
    mutationFn: () =>
      api.users.create({
        username: username.trim(),
        password,
        role,
        policy: {
          allLibraries: role === "admin" ? true : allLibraries,
          allowDownload,
          allowTranscode,
          dailyAiLimit: Math.max(0, Number.parseInt(dailyAiLimit, 10) || 0),
        },
        allowedLibraryIds: role === "admin" || allLibraries ? [] : allowedIds,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.users.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
    },
  });

  const reset = () => {
    setUsername("");
    setPassword("");
    setRole("user");
    setAllLibraries(true);
    setAllowedIds([]);
    setAllowDownload(true);
    setAllowTranscode(true);
    setDailyAiLimit("100");
    setErrors({});
    create.reset();
  };

  const close = () => {
    onOpenChange(false);
    reset();
  };

  const toggleLibrary = (id: string) => {
    setAllowedIds((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    const trimmed = username.trim();
    const nextErrors: { username?: string; password?: string; form?: string } = {};
    if (!trimmed) {
      nextErrors.username = t("validation.username.required");
    } else if ([...trimmed].length > MAX_USERNAME_LENGTH) {
      nextErrors.username = t("validation.username.tooLong", { max: MAX_USERNAME_LENGTH });
    } else if (/\s/.test(trimmed)) {
      nextErrors.username = t("validation.username.whitespace");
    }
    if ([...password].length < MIN_PASSWORD_LENGTH) {
      nextErrors.password = t("validation.password.tooShort", { min: MIN_PASSWORD_LENGTH });
    }
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;

    create.mutate(undefined, {
      onSuccess: (created) => {
        toast.success(t("adminUsers.created", { name: created.username }));
        close();
      },
      onError: (error) => {
        const text = errorText(t, error);
        if (error instanceof ApiError && error.details?.field === "username") {
          setErrors({ username: text });
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
      isDismissable={!create.isPending}
    >
      <Modal.Container size="md" scroll="inside">
        <Modal.Dialog {...secure} className="gap-0 p-0">
          <Modal.Header className="grid gap-1 px-6 pt-6 pb-4">
            <Modal.Heading className="font-heading text-lg font-title text-heading">
              {t("adminUsers.create")}
            </Modal.Heading>
          </Modal.Header>
          <Modal.Body className="px-6 pb-2">
            <form id={formId} className="grid gap-4" onSubmit={onSubmit} noValidate>
              <TextField value={username} onChange={setUsername} isInvalid={Boolean(errors.username)} autoFocus className="grid gap-1.5">
                <Label className="text-sm font-mid text-fg">{t("auth.username")}</Label>
                <Input autoComplete="off" />
                {errors.username ? <FieldError className="text-sm text-failed">{errors.username}</FieldError> : null}
              </TextField>

              <TextField value={password} onChange={setPassword} isInvalid={Boolean(errors.password)} className="grid gap-1.5">
                <Label className="text-sm font-mid text-fg">{t("auth.password")}</Label>
                <Input type="password" autoComplete="new-password" />
                {errors.password ? <FieldError className="text-sm text-failed">{errors.password}</FieldError> : null}
              </TextField>

              <RadioGroup
                value={role}
                onChange={(val) => {
                  if (val === "admin" || val === "user") setRole(val);
                }}
                orientation="horizontal"
                className="grid gap-1.5"
              >
                <Label className="text-sm font-mid text-fg">{t("settings.role")}</Label>
                <div className="flex flex-wrap gap-5">
                  {(["user", "admin"] as const).map((r) => (
                    <Radio key={r} value={r}>
                      <Radio.Content className="gap-2 text-base font-body text-fg">
                        <Radio.Control>
                          <Radio.Indicator />
                        </Radio.Control>
                        {t(`user.roles.${r}`)}
                      </Radio.Content>
                    </Radio>
                  ))}
                </div>
              </RadioGroup>

              {role === "user" ? (
                <>
                  <Switch isSelected={allLibraries} onChange={setAllLibraries}>
                    <Switch.Content className="gap-3 text-sm font-body text-fg">
                      <Switch.Control>
                        <Switch.Thumb />
                      </Switch.Control>
                      {t("adminUsers.allLibraries")}
                    </Switch.Content>
                  </Switch>
                  {!allLibraries ? (
                    <LibraryAclChecklist libraries={libraries} selectedIds={allowedIds} onToggle={toggleLibrary} />
                  ) : null}
                </>
              ) : null}

              <Switch isSelected={allowDownload} onChange={setAllowDownload}>
                <Switch.Content className="gap-3 text-sm font-body text-fg">
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  {t("adminUsers.allowDownload")}
                </Switch.Content>
              </Switch>

              <Switch isSelected={allowTranscode} onChange={setAllowTranscode}>
                <Switch.Content className="gap-3 text-sm font-body text-fg">
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  {t("adminUsers.allowTranscode")}
                </Switch.Content>
              </Switch>

              <TextField value={dailyAiLimit} onChange={setDailyAiLimit} className="grid gap-1.5">
                <Label className="text-sm font-mid text-fg">{t("adminUsers.dailyAiLimit")}</Label>
                <Input type="number" min={0} max={100000} />
              </TextField>

              {errors.form ? (
                <p role="alert" className="flex items-start gap-2 rounded-tile bg-failed-soft px-3 py-2.5 text-sm text-fg">
                  <AlertCircle className="mt-0.5 size-4 shrink-0 text-failed" aria-hidden />
                  {errors.form}
                </p>
              ) : null}
            </form>
          </Modal.Body>
          <Modal.Footer className="flex justify-end gap-2 px-6 pt-4 pb-6">
            <Button variant="secondary" onPress={close} isDisabled={create.isPending}>
              {t("common.cancel")}
            </Button>
            <Button type="submit" form={formId} variant="primary" isPending={create.isPending}>
              {create.isPending ? <Spinner size="sm" color="current" /> : null}
              {t("common.create")}
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

function EditUserDialog({
  account,
  libraries,
  isSoleEnabledAdmin,
  onClose,
}: {
  account: ManagedUser;
  libraries: Library[];
  isSoleEnabledAdmin: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const formId = useId();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();

  const [role, setRole] = useState<Role>(account.role);
  const [disabled, setDisabled] = useState(account.disabled);
  const [allLibraries, setAllLibraries] = useState(account.policy.allLibraries);
  const [allowedIds, setAllowedIds] = useState<string[]>(account.allowedLibraryIds);
  const [allowDownload, setAllowDownload] = useState(account.policy.allowDownload);
  const [allowTranscode, setAllowTranscode] = useState(account.policy.allowTranscode);
  const [dailyAiLimit, setDailyAiLimit] = useState(String(account.policy.dailyAiLimit));
  const [error, setError] = useState<string | null>(null);

  const update = useMutation({
    mutationFn: () =>
      api.users.update(account.id, {
        role,
        disabled,
        policy: {
          allLibraries: role === "admin" ? true : allLibraries,
          allowDownload,
          allowTranscode,
          dailyAiLimit: Math.max(0, Number.parseInt(dailyAiLimit, 10) || 0),
        },
        allowedLibraryIds: role === "admin" || allLibraries ? [] : allowedIds,
      }),
    onSuccess: (updated) => {
      toast.success(t("adminUsers.saved", { name: updated.username }));
      void queryClient.invalidateQueries({ queryKey: queryKeys.users.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
      onClose();
    },
    onError: (err) => {
      setError(errorText(t, err));
    },
  });

  const toggleLibrary = (id: string) => {
    setAllowedIds((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  };

  return (
    <Modal.Backdrop isOpen onOpenChange={(next) => { if (!next) onClose(); }} isDismissable={!update.isPending}>
      <Modal.Container size="md" scroll="inside">
        <Modal.Dialog {...secure} className="gap-0 p-0">
          <Modal.Header className="grid gap-1 px-6 pt-6 pb-4">
            <Modal.Heading className="font-heading text-lg font-title text-heading">
              {t("adminUsers.editTitle", { name: account.username })}
            </Modal.Heading>
          </Modal.Header>
          <Modal.Body className="px-6 pb-2">
            <form
              id={formId}
              className="grid gap-4"
              onSubmit={(event) => {
                event.preventDefault();
                setError(null);
                update.mutate();
              }}
              noValidate
            >
              <RadioGroup
                value={role}
                isDisabled={isSoleEnabledAdmin}
                onChange={(val) => {
                  if (val === "admin" || val === "user") setRole(val);
                }}
                orientation="horizontal"
                className="grid gap-1.5"
              >
                <Label className="text-sm font-mid text-fg">{t("settings.role")}</Label>
                <div className="flex flex-wrap gap-5">
                  {(["user", "admin"] as const).map((r) => (
                    <Radio key={r} value={r}>
                      <Radio.Content className="gap-2 text-base font-body text-fg">
                        <Radio.Control>
                          <Radio.Indicator />
                        </Radio.Control>
                        {t(`user.roles.${r}`)}
                      </Radio.Content>
                    </Radio>
                  ))}
                </div>
              </RadioGroup>

              <Switch isSelected={disabled} isDisabled={isSoleEnabledAdmin} onChange={setDisabled}>
                <Switch.Content className="gap-3 text-sm font-body text-fg">
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  {t("adminUsers.disableAccount")}
                </Switch.Content>
              </Switch>

              {role === "user" ? (
                <>
                  <Switch isSelected={allLibraries} onChange={setAllLibraries}>
                    <Switch.Content className="gap-3 text-sm font-body text-fg">
                      <Switch.Control>
                        <Switch.Thumb />
                      </Switch.Control>
                      {t("adminUsers.allLibraries")}
                    </Switch.Content>
                  </Switch>
                  {!allLibraries ? (
                    <LibraryAclChecklist libraries={libraries} selectedIds={allowedIds} onToggle={toggleLibrary} />
                  ) : null}
                </>
              ) : null}

              <Switch isSelected={allowDownload} onChange={setAllowDownload}>
                <Switch.Content className="gap-3 text-sm font-body text-fg">
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  {t("adminUsers.allowDownload")}
                </Switch.Content>
              </Switch>

              <Switch isSelected={allowTranscode} onChange={setAllowTranscode}>
                <Switch.Content className="gap-3 text-sm font-body text-fg">
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  {t("adminUsers.allowTranscode")}
                </Switch.Content>
              </Switch>

              <TextField value={dailyAiLimit} onChange={setDailyAiLimit} className="grid gap-1.5">
                <Label className="text-sm font-mid text-fg">{t("adminUsers.dailyAiLimit")}</Label>
                <Input type="number" min={0} max={100000} />
              </TextField>

              {error ? (
                <p role="alert" className="flex items-start gap-2 rounded-tile bg-failed-soft px-3 py-2.5 text-sm text-fg">
                  <AlertCircle className="mt-0.5 size-4 shrink-0 text-failed" aria-hidden />
                  {error}
                </p>
              ) : null}
            </form>
          </Modal.Body>
          <Modal.Footer className="flex justify-end gap-2 px-6 pt-4 pb-6">
            <Button variant="secondary" onPress={onClose} isDisabled={update.isPending}>
              {t("common.cancel")}
            </Button>
            <Button type="submit" form={formId} variant="primary" isPending={update.isPending}>
              {update.isPending ? <Spinner size="sm" color="current" /> : null}
              {t("settings.save")}
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

function ResetPasswordDialog({
  account,
  onClose,
}: {
  account: ManagedUser;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const formId = useId();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  const resetPw = useMutation({
    mutationFn: () => api.users.update(account.id, { password }),
    onSuccess: () => {
      toast.success(t("adminUsers.resetDone", { name: account.username }));
      void queryClient.invalidateQueries({ queryKey: queryKeys.users.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
      onClose();
    },
    onError: (err) => setError(errorText(t, err)),
  });

  return (
    <Modal.Backdrop isOpen onOpenChange={(next) => { if (!next) onClose(); }} isDismissable={!resetPw.isPending}>
      <Modal.Container size="sm">
        <Modal.Dialog {...secure} className="gap-0 p-0">
          <Modal.Header className="grid gap-1 px-6 pt-6 pb-4">
            <Modal.Heading className="font-heading text-lg font-title text-heading">
              {t("adminUsers.resetPasswordTitle", { name: account.username })}
            </Modal.Heading>
          </Modal.Header>
          <Modal.Body className="px-6 pb-2">
            <form
              id={formId}
              className="grid gap-4"
              onSubmit={(event) => {
                event.preventDefault();
                if ([...password].length < MIN_PASSWORD_LENGTH) {
                  setError(t("validation.password.tooShort", { min: MIN_PASSWORD_LENGTH }));
                  return;
                }
                setError(null);
                resetPw.mutate();
              }}
              noValidate
            >
              <TextField value={password} onChange={setPassword} isInvalid={Boolean(error)} autoFocus className="grid gap-1.5">
                <Label className="text-sm font-mid text-fg">{t("accountSecurity.newPassword")}</Label>
                <Input type="password" autoComplete="new-password" />
                {error ? <FieldError className="text-sm text-failed">{error}</FieldError> : null}
              </TextField>
            </form>
          </Modal.Body>
          <Modal.Footer className="flex justify-end gap-2 px-6 pt-4 pb-6">
            <Button variant="secondary" onPress={onClose} isDisabled={resetPw.isPending}>
              {t("common.cancel")}
            </Button>
            <Button type="submit" form={formId} variant="primary" isPending={resetPw.isPending}>
              {resetPw.isPending ? <Spinner size="sm" color="current" /> : null}
              {t("settings.save")}
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

function DeleteUserDialog({
  account,
  open,
  onClose,
}: {
  account: ManagedUser | null;
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const secure = useSecureSurface();
  const queryClient = useQueryClient();

  const remove = useMutation({
    mutationFn: (id: string) => api.users.remove(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.users.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.maintenance.auditAll });
    },
  });

  return (
    <AlertDialog.Backdrop
      isOpen={open}
      onOpenChange={(next) => {
        if (!next) {
          remove.reset();
          onClose();
        }
      }}
      isDismissable={!remove.isPending}
    >
      <AlertDialog.Container size="sm">
        <AlertDialog.Dialog {...secure}>
          <AlertDialog.Header>
            <AlertDialog.Icon status="danger">
              <Trash2 className="size-5" aria-hidden />
            </AlertDialog.Icon>
            <AlertDialog.Heading className="font-heading text-lg font-title text-heading">
              {t("adminUsers.deleteTitle", { name: account?.username ?? "" })}
            </AlertDialog.Heading>
          </AlertDialog.Header>
          <AlertDialog.Body>
            <div className="grid gap-2 text-base text-fg">
              <p>{t("adminUsers.deleteDescription")}</p>
              {remove.isError ? (
                <p role="alert" className="text-sm text-failed">
                  {errorText(t, remove.error)}
                </p>
              ) : null}
            </div>
          </AlertDialog.Body>
          <AlertDialog.Footer>
            <Button variant="secondary" onPress={onClose} isDisabled={remove.isPending}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              isPending={remove.isPending}
              onPress={() => {
                if (!account) return;
                remove.mutate(account.id, {
                  onSuccess: () => {
                    toast.success(t("adminUsers.deleted", { name: account.username }));
                    onClose();
                  },
                });
              }}
            >
              {remove.isPending ? <Spinner size="sm" color="current" /> : <Trash2 aria-hidden />}
              {t("common.delete")}
            </Button>
          </AlertDialog.Footer>
        </AlertDialog.Dialog>
      </AlertDialog.Container>
    </AlertDialog.Backdrop>
  );
}

function UserRow({
  account,
  isSelf,
  isSoleEnabledAdmin,
  onEdit,
  onResetPassword,
  onDelete,
}: {
  account: ManagedUser;
  isSelf: boolean;
  isSoleEnabledAdmin: boolean;
  onEdit: (account: ManagedUser) => void;
  onResetPassword: (account: ManagedUser) => void;
  onDelete: (account: ManagedUser) => void;
}) {
  const { t } = useTranslation();
  const inset = useRowInset();
  const accessSummary =
    account.role === "admin" || account.policy.allLibraries
      ? t("adminUsers.allLibraries")
      : account.allowedLibraryIds.length > 0
        ? t("adminUsers.restrictedLibraries", { count: account.allowedLibraryIds.length })
        : t("adminUsers.noLibrariesSelected");

  return (
    <li
      data-user-row={account.username}
      className={cn("flex flex-wrap items-center justify-between gap-4 py-4", inset)}
    >
      <div className="flex min-w-0 items-start gap-3">
        <UserAvatar name={account.username} className="mt-0.5 bg-sunken text-fg" />
        <div className="grid min-w-0 gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-md font-strong text-fg">{account.username}</span>
            <Chip size="sm" variant="soft" className="bg-sunken text-fg">
              {t(`user.roles.${account.role}`)}
            </Chip>
            {isSelf ? (
              <Chip size="sm" variant="soft" className="bg-ok-soft text-ok">
                {t("adminUsers.you")}
              </Chip>
            ) : null}
            {account.disabled ? (
              <Chip size="sm" variant="soft" className="bg-failed-soft text-failed">
                {t("adminUsers.disabled")}
              </Chip>
            ) : null}
          </div>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-fg-muted">
            <span>{accessSummary}</span>
            <span aria-hidden>·</span>
            <span>{t("adminUsers.aiQuota", { limit: account.policy.dailyAiLimit })}</span>
            <span aria-hidden>·</span>
            <span data-num className="tabular-nums">
              {t("adminUsers.activeSessions", { count: account.activeSessions })}
            </span>
            <span aria-hidden>·</span>
            <RelativeTime value={account.createdAt} />
          </div>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="outline" onPress={() => onEdit(account)} aria-label={`${t("adminUsers.edit")} ${account.username}`}>
          <Pencil aria-hidden />
          {t("adminUsers.edit")}
        </Button>
        <Button size="sm" variant="outline" onPress={() => onResetPassword(account)} aria-label={`${t("adminUsers.resetPassword")} ${account.username}`}>
          <KeyRound aria-hidden />
          {t("adminUsers.resetPassword")}
        </Button>
        {!isSelf && !isSoleEnabledAdmin ? (
          <Button size="sm" variant="outline" onPress={() => onDelete(account)} aria-label={`${t("common.delete")} ${account.username}`}>
            <Trash2 aria-hidden />
            {t("common.delete")}
          </Button>
        ) : null}
      </div>
    </li>
  );
}

/** 系统 › 家庭成员: multi-user accounts, per-library access control, password resets and lifecycle. */
export function UsersPage() {
  const { t } = useTranslation();
  const currentUser = useCurrentUser();
  const secure = useSecureSurface();
  const librariesQuery = useLibraries();
  const usersQuery = useQuery({
    queryKey: queryKeys.users.list,
    queryFn: ({ signal }) => api.users.list(signal),
  });

  const [createOpen, setCreateOpen] = useState(false);
  const [editing, setEditing] = useState<ManagedUser | null>(null);
  const [resetting, setResetting] = useState<ManagedUser | null>(null);
  const [deleting, setDeleting] = useState<ManagedUser | null>(null);
  const [deleteOpen, setDeleteOpen] = useState(false);

  const users = usersQuery.data?.users ?? [];
  const libraries = librariesQuery.data?.libraries ?? [];
  const enabledAdminCount = users.filter((u) => u.role === "admin" && !u.disabled).length;
  const activeSessionsTotal = users.reduce((sum, u) => sum + u.activeSessions, 0);

  return (
    <PageFrame>
      <PageHeader
        title={t("nav.users")}
        summary={users.length > 0 ? t("adminUsers.summary", { total: users.length, sessions: activeSessionsTotal }) : undefined}
        actions={
          <Button variant="primary" size="sm" onPress={() => setCreateOpen(true)}>
            <UserPlus aria-hidden />
            {t("adminUsers.create")}
          </Button>
        }
      />

      {usersQuery.isPending ? (
        <LoadingRows rows={3} />
      ) : usersQuery.isError ? (
        <ErrorState error={usersQuery.error} onRetry={() => void usersQuery.refetch()} />
      ) : users.length === 0 ? (
        <EmptyState
          icon={UsersIcon}
          title={t("nav.users")}
          action={
            <Button variant="primary" onPress={() => setCreateOpen(true)}>
              <UserPlus aria-hidden />
              {t("adminUsers.create")}
            </Button>
          }
        />
      ) : (
        <div {...secure}>
          <Section title={t("nav.users")} count={users.length}>
            <SectionRows>
              {users.map((account) => (
                <UserRow
                  key={account.id}
                  account={account}
                  isSelf={account.id === currentUser.id}
                  isSoleEnabledAdmin={account.role === "admin" && !account.disabled && enabledAdminCount <= 1}
                  onEdit={setEditing}
                  onResetPassword={setResetting}
                  onDelete={(target) => {
                    setDeleting(target);
                    setDeleteOpen(true);
                  }}
                />
              ))}
            </SectionRows>
          </Section>
        </div>
      )}

      <CreateUserDialog open={createOpen} onOpenChange={setCreateOpen} libraries={libraries} />
      {editing ? (
        <EditUserDialog
          key={editing.id}
          account={editing}
          libraries={libraries}
          isSoleEnabledAdmin={editing.role === "admin" && !editing.disabled && enabledAdminCount <= 1}
          onClose={() => setEditing(null)}
        />
      ) : null}
      {resetting ? (
        <ResetPasswordDialog key={resetting.id} account={resetting} onClose={() => setResetting(null)} />
      ) : null}
      <DeleteUserDialog account={deleting} open={deleteOpen} onClose={() => setDeleteOpen(false)} />
    </PageFrame>
  );
}
