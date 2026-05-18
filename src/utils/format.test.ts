import { describe, it, expect } from "vitest";
import {
  formatCount,
  toAutocompleteOptions,
  formatJournalName,
  formatFileSize,
} from "./format";

describe("formatCount", () => {
  it("formats 0", () => {
    expect(formatCount(0)).toBe("0");
  });

  it("formats integers without fractions", () => {
    expect(formatCount(42)).toBe("42");
    expect(formatCount(1000)).toBe("1,000");
    expect(formatCount(1000000)).toBe("1,000,000");
  });

  it("drops fractions from non-integer values", () => {
    expect(formatCount(42.7)).toBe("43");
    expect(formatCount(3.14159)).toBe("3");
  });

  it("formats large numbers with grouping", () => {
    expect(formatCount(123456789)).toBe("123,456,789");
  });
});

describe("toAutocompleteOptions", () => {
  it("maps strings to { value } objects", () => {
    expect(toAutocompleteOptions(["EUR", "USD", "GBP"])).toEqual([
      { value: "EUR" },
      { value: "USD" },
      { value: "GBP" },
    ]);
  });

  it("returns empty array for empty input", () => {
    expect(toAutocompleteOptions([])).toEqual([]);
  });

  it("handles strings with special characters", () => {
    expect(toAutocompleteOptions(["€", "$"])).toEqual([
      { value: "€" },
      { value: "$" },
    ]);
  });
});

describe("formatJournalName", () => {
  it("extracts filename from Unix path", () => {
    expect(formatJournalName("/Users/name/accounting/main.journal")).toBe("main.journal");
  });

  it("extracts filename from Windows path", () => {
    expect(formatJournalName("C:\\Users\\name\\accounting\\main.journal")).toBe("main.journal");
  });

  it("returns the name unchanged if no path separator", () => {
    expect(formatJournalName("journal.ledger")).toBe("journal.ledger");
  });

  it("handles trailing slash", () => {
    expect(formatJournalName("/path/to/journal/")).toBe("journal");
  });

  it("trims whitespace", () => {
    expect(formatJournalName("  /path/to/file.journal  ")).toBe("file.journal");
  });

  it("returns empty string for empty input", () => {
    expect(formatJournalName("")).toBe("");
  });

  it("returns whitespace-only input as empty", () => {
    expect(formatJournalName("   ")).toBe("");
  });
});

describe("formatFileSize", () => {
  it("shows bytes", () => {
    expect(formatFileSize(0)).toBe("0 B");
    expect(formatFileSize(512)).toBe("512 B");
  });
  it("shows KB", () => {
    expect(formatFileSize(1024)).toBe("1.0 KB");
    expect(formatFileSize(1536)).toBe("1.5 KB");
  });
  it("shows MB", () => {
    expect(formatFileSize(1048576)).toBe("1.0 MB");
  });
});
