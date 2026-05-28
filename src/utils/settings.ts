import type { AppModulesSettings, AppSettings } from "../types";

export const defaultGitCommitMessage = "Update journal from Ledgera";

export const defaultModules: AppModulesSettings = {
  marketPrices: { enabled: false },
  developerTools: { enabled: false },
  updateChecker: { enabled: true },
  gitSync: { enabled: false, commitMessage: defaultGitCommitMessage },
};

export const defaultSettings: AppSettings = {
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
};

export function normalizeModules(settings?: Partial<AppSettings>): AppModulesSettings {
  const modules = settings?.modules;
  const marketPricesEnabled = modules?.marketPrices?.enabled ?? settings?.fetchPrices ?? defaultModules.marketPrices.enabled;
  const developerToolsEnabled = modules?.developerTools?.enabled ?? settings?.powerUser ?? defaultModules.developerTools.enabled;

  return {
    marketPrices: { enabled: marketPricesEnabled },
    developerTools: { enabled: developerToolsEnabled },
    updateChecker: { enabled: modules?.updateChecker?.enabled ?? defaultModules.updateChecker.enabled },
    gitSync: {
      enabled: modules?.gitSync?.enabled ?? defaultModules.gitSync.enabled,
      commitMessage: modules?.gitSync?.commitMessage?.trim() || defaultGitCommitMessage,
    },
  };
}

export function normalizeSettings(settings?: Partial<AppSettings>): AppSettings {
  const modules = normalizeModules(settings);

  return {
    ...defaultSettings,
    ...settings,
    theme: settings?.theme ?? "system",
    language: settings?.language ?? "system",
    powerUser: modules.developerTools.enabled,
    fetchPrices: modules.marketPrices.enabled,
    modules,
  };
}
