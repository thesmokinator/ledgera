import type { AppError } from "../types";

/**
 * Attempts to parse an error body (string) as a structured AppError JSON.
 * Returns the AppError if successful, otherwise null.
 */
export function parseAppError(body: unknown): AppError | null {
  if (typeof body !== "string") return null;
  try {
    const parsed = JSON.parse(body);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      typeof parsed.code === "string" &&
      typeof parsed.message === "string"
    ) {
      return {
        code: parsed.code,
        message: parsed.message,
        details: typeof parsed.details === "string" ? parsed.details : undefined,
        fieldErrors: Array.isArray(parsed.fieldErrors)
          ? parsed.fieldErrors
            .filter(
              (fieldError: unknown): fieldError is { path: string[]; message: string } =>
                typeof fieldError === "object" &&
                fieldError !== null &&
                Array.isArray((fieldError as { path?: unknown }).path) &&
                (fieldError as { path: unknown[] }).path.every((part) => typeof part === "string") &&
                typeof (fieldError as { message?: unknown }).message === "string",
            )
            .map((fieldError: { path: string[]; message: string }) => ({
              path: fieldError.path,
              message: fieldError.message,
            }))
          : undefined,
      };
    }
  } catch {
    // not JSON
  }
  return null;
}

/**
 * Maps an AppError code to a user-facing message.
 * Falls back to the code itself if no i18n translation exists (so it can be translated later).
 */
export function formatAppError(error: AppError, t: (key: string) => string): string {
  const i18nKey = `errors.${error.code}`;
  const translated = t(i18nKey);

  // If i18next returns the key itself, no translation exists → show the code
  if (translated === i18nKey) {
    const base = `[${error.code}] ${error.message}`;
    return error.details ? `${base}\n${error.details}` : base;
  }

  // Translation exists → use it, optionally append details for power users
  return error.details ? `${translated}\n${error.details}` : translated;
}

/**
 * Parses any error value and returns a user-facing string.
 * - Structured AppError → formatted via i18n
 * - Raw string → shown as-is (legacy / unexpected)
 */
export function parseError(error: unknown, t: (key: string) => string): string {
  const appError = parseAppError(error);
  if (appError) {
    return formatAppError(appError, t);
  }
  return String(error ?? "Unknown error");
}
