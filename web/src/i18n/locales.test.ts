import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";
import { normalizeLanguage } from "./index";

const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

function keys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([key, child]) => keys(child, prefix ? `${prefix}.${key}` : key));
}

const baseKeys = (resource: unknown) => [...new Set(keys(resource).map((key) => key.replace(PLURAL_SUFFIX, "")))].sort();

describe("locales", () => {
  it("zh-CN and en define the same keys (plural forms aside)", () => {
    expect(baseKeys(en)).toEqual(baseKeys(zhCN));
  });

  it("maps browser language tags onto supported locales", () => {
    expect(normalizeLanguage("zh-TW")).toBe("zh-CN");
    expect(normalizeLanguage("zh")).toBe("zh-CN");
    expect(normalizeLanguage("en-GB")).toBe("en");
    expect(normalizeLanguage("fr-FR")).toBe("fr-FR");
  });
});
