/** Splits a formatted size such as "3.35 MB" into its number and unit, for <Measure>. */
export function splitMeasure(text: string): { value: string; unit?: string } {
  const match = /^(.*\d)\s*([^\d\s][^\d]*)$/.exec(text);
  return match ? { value: match[1] ?? text, unit: match[2] } : { value: text };
}
