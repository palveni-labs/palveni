export const USERNAME_MAX = 64;
export const PASSWORD_MIN = 8;

export type UsernameProblem = "required" | "tooLong" | "whitespace";

export function checkUsername(value: string): UsernameProblem | null {
  if (value.length === 0) return "required";
  if ([...value].length > USERNAME_MAX) return "tooLong";
  if (/\s/.test(value)) return "whitespace";
  return null;
}

export function checkPassword(value: string): "tooShort" | null {
  return [...value].length < PASSWORD_MIN ? "tooShort" : null;
}
