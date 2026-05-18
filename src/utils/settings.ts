import type { AppSettings } from "../types";

export const defaultSettings: AppSettings = {
  journalPath: "",
  hledgerPath: "",
  theme: "system",
  powerUser: false,
  defaultCommodity: "",
  fetchPrices: false,
  commoditySymbols: "",
};

export function normalizeSettings(settings?: Partial<AppSettings>): AppSettings {
  return {
    ...defaultSettings,
    ...settings,
    theme: settings?.theme ?? "system",
    powerUser: settings?.powerUser ?? false,
    fetchPrices: settings?.fetchPrices ?? false,
  };
}
