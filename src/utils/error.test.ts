import { describe, it, expect } from "vitest";
import { parseAppError, parseError, formatAppError } from "./error";
import type { AppError } from "../types";

function mockT(key: string): string {
  const translations: Record<string, string> = {
    "errors.JOURNAL_NOT_CONFIGURED": "Nessun journal configurato.",
    "errors.JOURNAL_NOT_FOUND": "File journal non trovato.",
  };
  return translations[key] ?? key;
}

describe("formatAppError", () => {
  it("returns translated message when i18n key exists", () => {
    const error: AppError = {
      code: "JOURNAL_NOT_CONFIGURED",
      message: "Configure a journal path.",
    };
    expect(formatAppError(error, mockT)).toBe("Nessun journal configurato.");
  });

  it("falls back to code + message when no translation", () => {
    const error: AppError = {
      code: "UNKNOWN_ERROR",
      message: "Something went wrong.",
    };
    const result = formatAppError(error, mockT);
    expect(result).toContain("[UNKNOWN_ERROR]");
    expect(result).toContain("Something went wrong.");
  });

  it("appends details when present", () => {
    const error: AppError = {
      code: "JOURNAL_NOT_FOUND",
      message: "Journal file does not exist.",
      details: "Expected at: /tmp/test.journal",
    };
    const result = formatAppError(error, mockT);
    expect(result).toContain("File journal non trovato.");
    expect(result).toContain("/tmp/test.journal");
  });
});

describe("parseError", () => {
  it("parses a JSON AppError string", () => {
    const raw = JSON.stringify({
      code: "JOURNAL_NOT_CONFIGURED",
      message: "Configure a journal path.",
    });
    const result = parseError(raw, mockT);
    expect(result).toBe("Nessun journal configurato.");
  });

  it("parses field errors from a JSON AppError string", () => {
    const raw = JSON.stringify({
      code: "transaction_validation_failed",
      message: "Transaction validation failed.",
      fieldErrors: [
        { path: ["postings", "0", "account"], message: "Account is required." },
        { path: ["description"], message: "Description is required." },
      ],
    });

    const result = parseAppError(raw);

    expect(result?.fieldErrors).toEqual([
      { path: ["postings", "0", "account"], message: "Account is required." },
      { path: ["description"], message: "Description is required." },
    ]);
  });

  it("falls back to raw string for non-JSON errors", () => {
    const result = parseError("Something broke", mockT);
    expect(result).toBe("Something broke");
  });

  it("falls back to raw string for malformed JSON", () => {
    const result = parseError('{"foo":"bar"}', mockT);
    expect(result).toBe('{"foo":"bar"}');
  });

  it("handles null/undefined gracefully", () => {
    expect(parseError(null, mockT)).toBe("Unknown error");
    expect(parseError(undefined, mockT)).toBe("Unknown error");
  });
});
