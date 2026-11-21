/** Accepts only same-origin paths ("/x", not "//host" or "/\\host") as post-login targets. */
export function safeRedirect(value: unknown): string | undefined {
  if (typeof value !== "string" || !value.startsWith("/")) return undefined;
  if (value.startsWith("//") || value.startsWith("/\\")) return undefined;
  if (value.startsWith("/login") || value.startsWith("/setup")) return undefined;
  return value;
}
