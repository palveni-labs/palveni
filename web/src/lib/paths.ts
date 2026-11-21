/** POSIX absolute path, Windows drive path, or UNC share. */
export function isAbsolutePath(value: string): boolean {
  return /^\//.test(value) || /^[A-Za-z]:[\\/]/.test(value) || /^\\\\[^\\]+\\/.test(value);
}
