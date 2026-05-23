import { describe, it, expect } from "vitest";
import { normalizeSettings, defaultSettings } from "./settings";
import type { AppSettings } from "../types";

describe("defaultSettings", () => {
  it("has expected default values", () => {
    expect(defaultSettings).toEqual({
      journalPath: "",
      hledgerPath: "",
      theme: "system",
      language: "system",
      powerUser: false,
      defaultCommodity: "",
      fetchPrices: false,
      commoditySymbols: "",
      excludeBalances: "",
      includeInvestments: "",
      prefillPostings: false,
    });
  });
});

describe("normalizeSettings", () => {
  it("returns defaults when called with undefined", () => {
    expect(normalizeSettings(undefined)).toEqual(defaultSettings);
  });

  it("returns defaults when called with empty object", () => {
    expect(normalizeSettings({})).toEqual(defaultSettings);
  });

  it("merges provided values over defaults", () => {
    const partial: Partial<AppSettings> = {
      journalPath: "/path/to/journal",
      defaultCommodity: "EUR",
    };
    const result = normalizeSettings(partial);
    expect(result.journalPath).toBe("/path/to/journal");
    expect(result.defaultCommodity).toBe("EUR");
    expect(result.hledgerPath).toBe("");
    expect(result.theme).toBe("system");
    expect(result.language).toBe("system");
    expect(result.powerUser).toBe(false);
  });

  it("forces theme to 'system' when not provided", () => {
    const result = normalizeSettings({ theme: undefined } as Partial<AppSettings>);
    expect(result.theme).toBe("system");
  });

  it("forces language to 'system' when not provided", () => {
    const result = normalizeSettings({ language: undefined } as Partial<AppSettings>);
    expect(result.language).toBe("system");
  });

  it("forces powerUser to false when not provided", () => {
    const result = normalizeSettings({ powerUser: undefined } as Partial<AppSettings>);
    expect(result.powerUser).toBe(false);
  });

  it("preserves explicit theme value", () => {
    expect(normalizeSettings({ theme: "dark" }).theme).toBe("dark");
    expect(normalizeSettings({ theme: "light" }).theme).toBe("light");
    expect(normalizeSettings({ theme: "system" }).theme).toBe("system");
  });

  it("preserves explicit language value", () => {
    expect(normalizeSettings({ language: "en" }).language).toBe("en");
    expect(normalizeSettings({ language: "it" }).language).toBe("it");
    expect(normalizeSettings({ language: "system" }).language).toBe("system");
  });

  it("preserves explicit powerUser value", () => {
    expect(normalizeSettings({ powerUser: true }).powerUser).toBe(true);
    expect(normalizeSettings({ powerUser: false }).powerUser).toBe(false);
  });

  it("merges all fields", () => {
    const full: AppSettings = {
      journalPath: "/journal.journal",
      hledgerPath: "/usr/local/bin/hledger",
      theme: "dark",
      language: "it",
      powerUser: true,
      defaultCommodity: "USD",
      fetchPrices: true,
      commoditySymbols: "TEST=TEST.DE",
      excludeBalances: "",
      includeInvestments: "",
      prefillPostings: false,
    };
    expect(normalizeSettings(full)).toEqual(full);
  });
});
