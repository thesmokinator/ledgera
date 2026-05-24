import type { LanguagePreference } from "../types";

export const supportedLanguages: Array<{ value: LanguagePreference; labelKey: string }> = [
  { value: "system", labelKey: "settings.language_system" },
  { value: "en", labelKey: "settings.language_english" },
  { value: "it", labelKey: "settings.language_italian" },
];

const supportedLanguageCodes = ["en", "it"] as const;

type SupportedLanguageCode = typeof supportedLanguageCodes[number];

function normalizeDetectedLanguage(language: string): SupportedLanguageCode | null {
  const normalized = language.toLowerCase().split("-")[0];
  return supportedLanguageCodes.includes(normalized as SupportedLanguageCode)
    ? (normalized as SupportedLanguageCode)
    : null;
}

export function detectSystemLanguage(): SupportedLanguageCode {
  if (typeof navigator === "undefined") return "en";

  const candidates = [
    ...(Array.isArray(navigator.languages) ? navigator.languages : []),
    navigator.language,
  ].filter(Boolean);

  for (const candidate of candidates) {
    const language = normalizeDetectedLanguage(candidate);
    if (language) return language;
  }

  return "en";
}

export function resolveLanguagePreference(language: LanguagePreference): SupportedLanguageCode {
  return language === "system" ? detectSystemLanguage() : language;
}
