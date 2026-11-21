import { Description, FieldError, Input, Label, TextField as HeroTextField } from "@heroui/react";
import type { ReactNode } from "react";

interface FieldProps {
  label: ReactNode;
  error?: string | null;
  hint?: ReactNode;
  value: string;
  onChange: (value: string) => void;
  type?: "text" | "password";
  name?: string;
  autoComplete?: string;
  autoFocus?: boolean;
  placeholder?: string;
  className?: string;
  maxLength?: number;
  spellCheck?: "true" | "false";
  autoCorrect?: "on" | "off";
}

/** Label + input + inline hint or error, wired with aria attributes (HeroUI TextField). */
export function TextField({ label, error, hint, value, onChange, type = "text", className, ...props }: FieldProps) {
  return (
    <HeroTextField value={value} onChange={onChange} isInvalid={Boolean(error)} type={type} className={className} {...props}>
      <Label className="text-sm font-mid text-fg">{label}</Label>
      <Input className="h-10" />
      {error ? <FieldError className="text-xs text-failed">{error}</FieldError> : hint ? <Description className="text-xs text-fg-muted">{hint}</Description> : null}
    </HeroTextField>
  );
}
