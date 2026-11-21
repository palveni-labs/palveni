import { Button, Description, FieldError, Input, Label, Modal, Radio, RadioGroup, Spinner, Switch, TextField, toast } from "@heroui/react";
import { Link } from "@tanstack/react-router";
import { AlertCircle, Plus, X } from "lucide-react";
import { useId, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { ApiError } from "@/api/errors";
import { useCreateLibrary, useScanLibrary } from "@/api/queries/libraries";
import type { LibraryKind } from "@/api/types";
import { describeError, errorText } from "@/lib/errors";
import { isLibraryKind, LIBRARY_KIND_META, LIBRARY_KINDS } from "@/lib/kinds";
import { isAbsolutePath } from "@/lib/paths";
import { useFeature } from "@/modules/access";
import { MountPicker } from "./mount-picker";

interface FormErrors {
  name?: string;
  paths?: string;
  /** Indexes of the paths the message is about; the message sits under the first of them. */
  badPaths?: number[];
  form?: string;
}

/** 新建媒体库: a name, a media type, one or more folders on the server, and whether to scan now. */
export function CreateLibraryDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation();
  const formId = useId();
  const create = useCreateLibrary();
  const scan = useScanLibrary();
  const storageEnabled = useFeature("connections");
  const [name, setName] = useState("");
  const [kind, setKind] = useState<LibraryKind>("video");
  const [paths, setPaths] = useState<string[]>([""]);
  const [scanAfter, setScanAfter] = useState(true);
  const [errors, setErrors] = useState<FormErrors>({});

  const reset = () => {
    setName("");
    setKind("video");
    setPaths([""]);
    setScanAfter(true);
    setErrors({});
    create.reset();
  };

  const close = () => {
    onOpenChange(false);
    reset();
  };

  const validate = (): { ok: boolean; cleanPaths: string[] } => {
    const next: FormErrors = {};
    const cleanPaths = [...new Set(paths.map((path) => path.trim()).filter(Boolean))];
    if (!name.trim()) next.name = t("adminLibraries.form.nameRequired");
    if (cleanPaths.length === 0) {
      next.paths = t("adminLibraries.form.pathRequired");
      next.badPaths = [0];
    } else {
      const relative = paths.flatMap((path, index) => (path.trim() && !isAbsolutePath(path.trim()) ? [index] : []));
      if (relative.length > 0) {
        next.paths = t("adminLibraries.form.pathAbsolute");
        next.badPaths = relative;
      }
    }
    setErrors(next);
    return { ok: Object.keys(next).length === 0, cleanPaths };
  };

  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    const { ok, cleanPaths } = validate();
    if (!ok) return;
    create.mutate(
      { name: name.trim(), kind, paths: cleanPaths },
      {
        onSuccess: (library) => {
          toast.success(t("adminLibraries.created", { name: library.name }));
          if (scanAfter) scan.mutate(library.id, { onError: (error) => toast.danger(describeError(t, error).title) });
          close();
        },
        onError: (error) => {
          const text = errorText(t, error);
          if (!(error instanceof ApiError)) {
            setErrors({ form: text });
            return;
          }
          const details = error.details;
          const pathError = error.code === "path_not_found" || (error.code === "validation_failed" && details?.field === "paths");
          const nameError = (error.code === "conflict" || error.code === "validation_failed") && details?.field === "name";
          if (pathError) {
            // `details.index` counts the paths as sent (trimmed, without blanks or repeats). Without
            // details, fall back to a path the server message names.
            const sent = details?.index !== undefined ? cleanPaths[details.index] : (details?.path ?? cleanPaths.find((path) => error.message.includes(path)));
            const index = sent === undefined ? -1 : paths.findIndex((path) => path.trim() === sent);
            if (index >= 0) {
              setErrors({ paths: error.code === "path_not_found" ? t("adminLibraries.form.pathMissing", { path: sent }) : text, badPaths: [index] });
              return;
            }
          }
          setErrors(nameError ? { name: text } : { form: text });
        },
      },
    );
  };

  const badPaths = errors.badPaths ?? [];
  const firstBad = badPaths[0];

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
        <Modal.Dialog className="gap-0 p-0">
          <Modal.Header className="grid gap-1 px-6 pt-6 pb-4">
            <Modal.Heading className="font-heading text-lg font-title text-heading">{t("adminLibraries.create")}</Modal.Heading>
            <p className="text-sm text-fg-muted">{t("adminLibraries.form.description")}</p>
          </Modal.Header>
          <Modal.Body className="px-6 pb-2">
            <form id={formId} className="grid gap-6" onSubmit={onSubmit} noValidate>
              <TextField value={name} onChange={setName} isInvalid={Boolean(errors.name)} autoFocus className="grid gap-1.5">
                <Label className="text-sm font-mid text-fg">{t("adminLibraries.form.name")}</Label>
                <Input placeholder={t("adminLibraries.form.namePlaceholder")} />
                {errors.name ? <FieldError className="text-sm text-failed">{errors.name}</FieldError> : null}
              </TextField>

              <RadioGroup
                value={kind}
                onChange={(value) => {
                  if (isLibraryKind(value)) setKind(value);
                }}
                orientation="horizontal"
                className="grid gap-2"
              >
                <Label className="text-sm font-mid text-fg">{t("adminLibraries.form.kind")}</Label>
                <div className="flex flex-wrap gap-x-5 gap-y-2.5">
                  {LIBRARY_KINDS.map((value) => {
                    const Icon = LIBRARY_KIND_META[value].icon;
                    return (
                      <Radio key={value} value={value}>
                        <Radio.Content className="gap-2 text-base font-body text-fg">
                          <Radio.Control>
                            <Radio.Indicator />
                          </Radio.Control>
                          <Icon className="size-4 text-icon" aria-hidden />
                          {t(`kinds.${value}`)}
                        </Radio.Content>
                      </Radio>
                    );
                  })}
                </div>
              </RadioGroup>

              <div className="grid gap-2">
                {paths.map((path, index) => {
                  const invalid = badPaths.includes(index);
                  return (
                    <div key={index} className="flex items-start gap-2">
                      <TextField
                        value={path}
                        onChange={(value) => setPaths((current) => current.map((entry, i) => (i === index ? value : entry)))}
                        isInvalid={invalid}
                        aria-label={index === 0 ? undefined : t("adminLibraries.form.pathN", { n: index + 1 })}
                        className="grid min-w-0 flex-1 gap-1.5"
                      >
                        {index === 0 ? <Label className="text-sm font-mid text-fg">{t("adminLibraries.form.paths")}</Label> : null}
                        <Input
                          className="font-mono"
                          placeholder={t("adminLibraries.form.pathPlaceholder")}
                          spellCheck={false}
                          autoCapitalize="none"
                          autoCorrect="off"
                        />
                        {index === 0 && !errors.paths ? <Description className="text-sm text-fg-muted">{t("adminLibraries.form.pathHint")}</Description> : null}
                        {invalid && index === firstBad && errors.paths ? <FieldError className="text-sm text-failed">{errors.paths}</FieldError> : null}
                      </TextField>
                      {paths.length > 1 ? (
                        <Button
                          variant="ghost"
                          isIconOnly
                          aria-label={t("adminLibraries.form.removePathN", { n: index + 1 })}
                          className={cn(index === 0 && "mt-[calc(var(--pv-text-sm)*1.5+0.375rem)]")}
                          onPress={() => {
                            setPaths((current) => current.filter((_, i) => i !== index));
                            setErrors((current) => ({ ...current, paths: undefined, badPaths: undefined }));
                          }}
                        >
                          <X aria-hidden />
                        </Button>
                      ) : null}
                    </div>
                  );
                })}
                {errors.paths && badPaths.length === 0 ? (
                  <p role="alert" className="text-sm text-failed">
                    {errors.paths}
                  </p>
                ) : null}
                <Button variant="ghost" size="sm" className="-ms-2 w-fit" onPress={() => setPaths((current) => [...current, ""])}>
                  <Plus aria-hidden />
                  {t("adminLibraries.form.addPath")}
                </Button>
              </div>

              {open && storageEnabled && <MountPicker onPick={(path) => {
                setPaths((current) => [...new Set([...current.filter((value) => value.trim()), path])]);
                setErrors((current) => ({ ...current, paths: undefined, badPaths: undefined }));
              }} />}
              {storageEnabled && <Link to="/library/connections" className="w-fit text-sm text-link" onClick={close}>{t("connections.title")}</Link>}
              <Switch isSelected={scanAfter} onChange={setScanAfter}>
                <Switch.Content className="gap-3 text-base font-body text-fg">
                  <Switch.Control>
                    <Switch.Thumb />
                  </Switch.Control>
                  {t("adminLibraries.form.scanAfter")}
                </Switch.Content>
                <Description className="text-sm text-fg-muted">{t("adminLibraries.form.scanAfterHint")}</Description>
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
