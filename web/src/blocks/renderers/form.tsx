/**
 * A form a plugin or the assistant asks the user to fill in. Standard HeroUI fields only; the
 * host collects the values and decides what the submit does.
 */
import { Button, Description, Input, Label, ListBox, NumberField, Select, Switch, TextField } from "@heroui/react";
import { KeyRound } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import type { FormBlock, FormField } from "../types";
import { BlockTitle } from "./basic";

type Values = Record<string, string | number | boolean>;

function initialValues(fields: FormField[]): Values {
  const values: Values = {};
  for (const field of fields) {
    if (field.kind === "toggle") values[field.name] = field.value ?? false;
    else if (field.kind === "number") values[field.name] = field.value ?? field.min ?? 0;
    else if (field.kind === "secret") values[field.name] = "";
    else values[field.name] = field.value ?? "";
  }
  return values;
}

const LABEL = "text-sm font-mid text-fg";

function Field({ field, value, onChange, readOnly }: { field: FormField; value: string | number | boolean | undefined; onChange: (value: string | number | boolean) => void; readOnly: boolean }) {
  const { t } = useTranslation();
  switch (field.kind) {
    case "toggle":
      return (
        <Switch isSelected={value === true} onChange={onChange} isDisabled={readOnly} className="w-full">
          <Switch.Content className="flex w-full items-center justify-between gap-4">
            <Label className={LABEL}>{field.label}</Label>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch.Content>
        </Switch>
      );
    case "select":
      return (
        <Select value={typeof value === "string" && value ? value : null} onChange={(key) => onChange(key === null ? "" : String(key))} isDisabled={readOnly} className="grid gap-1.5">
          <Label className={LABEL}>{field.label}</Label>
          <Select.Trigger className="w-full">
            <Select.Value />
            <Select.Indicator />
          </Select.Trigger>
          <Select.Popover>
            <ListBox>
              {field.options.map((option) => (
                <ListBox.Item key={option.value} id={option.value} textValue={option.label}>
                  {option.label}
                  <ListBox.ItemIndicator />
                </ListBox.Item>
              ))}
            </ListBox>
          </Select.Popover>
        </Select>
      );
    case "number":
      return (
        <NumberField
          value={typeof value === "number" ? value : undefined}
          onChange={(next) => onChange(Number.isFinite(next) ? next : (field.min ?? 0))}
          minValue={field.min}
          maxValue={field.max}
          isDisabled={readOnly}
          className="grid gap-1.5"
        >
          <Label className={LABEL}>{field.label}</Label>
          <div className="flex items-center gap-2">
            <NumberField.Group className="w-40">
              <NumberField.DecrementButton />
              <NumberField.Input className="tabular-nums" />
              <NumberField.IncrementButton />
            </NumberField.Group>
            {field.unit ? <span className="text-sm text-fg-muted">{field.unit}</span> : null}
          </div>
        </NumberField>
      );
    case "secret":
      return (
        <TextField type="password" value={String(value ?? "")} onChange={onChange} isDisabled={readOnly} autoComplete="off" className="grid gap-1.5">
          <Label className={LABEL}>{field.label}</Label>
          <Input className="w-full" />
          {field.configured ? (
            <Description className="flex items-center gap-1.5 text-xs text-fg-muted">
              <KeyRound className="size-3.5 shrink-0" aria-hidden />
              {t("blocks.secretConfigured")}
            </Description>
          ) : null}
        </TextField>
      );
    case "text":
      return (
        <TextField value={String(value ?? "")} onChange={onChange} isDisabled={readOnly} className="grid gap-1.5">
          <Label className={LABEL}>{field.label}</Label>
          <Input className="w-full" placeholder={field.placeholder} />
        </TextField>
      );
  }
}

export function FormBlockView({ block, onSubmit, readOnly }: { block: FormBlock; onSubmit: (values: Values) => void; readOnly: boolean }) {
  const { t } = useTranslation();
  const [values, setValues] = useState<Values>(() => initialValues(block.fields));
  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit(values);
  };
  return (
    <form className="grid max-w-xl gap-4" onSubmit={submit}>
      {block.title ? <BlockTitle>{block.title}</BlockTitle> : null}
      {block.fields.map((field) => (
        <Field
          key={field.name}
          field={field}
          value={values[field.name]}
          readOnly={readOnly}
          onChange={(value) => setValues((current) => ({ ...current, [field.name]: value }))}
        />
      ))}
      {!readOnly ? (
        <div className="flex justify-end">
          <Button type="submit" variant="primary">
            {block.submitLabel ?? t("blocks.submit")}
          </Button>
        </div>
      ) : null}
    </form>
  );
}
