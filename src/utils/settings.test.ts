import { describe, it, expect } from "vitest";
import { normalizeModules, normalizeSettings, defaultGitCommitMessage, defaultModules, defaultSettings } from "./settings";
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
      commoditySymbols: [],
      excludeBalances: [],
      includeInvestments: [],
      prefillPostings: false,
      modules: defaultModules,
    });
  });
});

describe("normalizeModules", () => {
  it("returns module defaults when called with undefined", () => {
    expect(normalizeModules(undefined)).toEqual(defaultModules);
  });

  it("migrates legacy fetchPrices and powerUser into modules", () => {
    expect(normalizeModules({ fetchPrices: true, powerUser: true })).toEqual({
      marketPrices: { enabled: true },
      developerTools: { enabled: true },
      updateChecker: { enabled: true },
      gitSync: { enabled: false, commitMessage: defaultGitCommitMessage },
      autoGenerateRecurring: false,
    });
  });

  it("prefers explicit modules over legacy fields", () => {
    expect(normalizeModules({
      fetchPrices: true,
      powerUser: true,
      modules: {
        marketPrices: { enabled: false },
        developerTools: { enabled: false },
        gitSync: { enabled: true, commitMessage: "My journal update" },
      },
    })).toEqual({
      marketPrices: { enabled: false },
      developerTools: { enabled: false },
      updateChecker: { enabled: true },
      gitSync: { enabled: true, commitMessage: "My journal update" },
      autoGenerateRecurring: false,
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
    expect(result.modules).toEqual(defaultModules);
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

  it("normalizes legacy powerUser and fetchPrices values", () => {
    const result = normalizeSettings({ powerUser: true, fetchPrices: true });
    expect(result.powerUser).toBe(true);
    expect(result.fetchPrices).toBe(true);
    expect(result.modules.developerTools.enabled).toBe(true);
    expect(result.modules.marketPrices.enabled).toBe(true);
  });

  it("normalizes modules back to legacy compatibility fields", () => {
    const result = normalizeSettings({
      modules: {
        marketPrices: { enabled: true },
        developerTools: { enabled: true },
        updateChecker: { enabled: true },
        gitSync: { enabled: true, commitMessage: "Sync journal" },
      },
    });

    expect(result.fetchPrices).toBe(true);
    expect(result.powerUser).toBe(true);
    expect(result.modules.gitSync.enabled).toBe(true);
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
      commoditySymbols: [{ commodity: "TEST", yahooSymbol: "TEST.DE" }],
      excludeBalances: [],
      includeInvestments: [],
      prefillPostings: false,
      modules: {
        marketPrices: { enabled: true },
        developerTools: { enabled: true },
        updateChecker: { enabled: true },
        gitSync: { enabled: true, commitMessage: "Sync journal" },
        autoGenerateRecurring: false,
      },
    };
    expect(normalizeSettings(full)).toEqual(full);
  });
});
