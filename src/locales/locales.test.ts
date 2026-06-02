import { describe, expect, it } from "vitest";
import en from "./en.json";
import itLocale from "./it.json";

function flattenKeys(value: unknown, prefix = ""): string[] {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return prefix ? [prefix] : [];
  }

  return Object.entries(value).flatMap(([key, nestedValue]) =>
    flattenKeys(nestedValue, prefix ? `${prefix}.${key}` : key),
  );
}

describe("locales", () => {
  it("keeps English and Italian keys in sync", () => {
    expect(flattenKeys(itLocale).sort()).toEqual(flattenKeys(en).sort());
  });
});
