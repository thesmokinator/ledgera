import { describe, it, expect, vi, afterEach } from "vitest";
import dayjs from "dayjs";
import {
  journalDateFormat,
  todayJournalDate,
  isValidJournalDate,
  isSameJournalMonth,
  isExecutedTransaction,
  normalizeDate,
} from "./date";
import type { JournalTransaction } from "../types";

function makeTx(date: string): JournalTransaction {
  return {
    id: "1",
    sourceFile: "main.journal",
    date,
    status: "*",
    code: "",
    description: "test",
    postings: [],
    display: { account: "", amount: "", kind: "" },
    raw: "",
    startLine: 1,
    endLine: 1,
  };
}

describe("journalDateFormat", () => {
  it("is the expected format string", () => {
    expect(journalDateFormat).toBe("YYYY-MM-DD");
  });
});

describe("todayJournalDate", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns today's date in journal format", () => {
    const fakeToday = "2025-03-15";
    vi.useFakeTimers();
    vi.setSystemTime(new Date(fakeToday));
    expect(todayJournalDate()).toBe(fakeToday);
  });

  it("returns a valid date that passes isValidJournalDate", () => {
    expect(isValidJournalDate(todayJournalDate())).toBe(true);
  });
});

describe("isValidJournalDate", () => {
  it("accepts a valid date", () => {
    expect(isValidJournalDate("2024-01-15")).toBe(true);
  });

  it("rejects an invalid month", () => {
    expect(isValidJournalDate("2024-13-01")).toBe(false);
  });

  it("rejects an invalid day", () => {
    expect(isValidJournalDate("2024-02-30")).toBe(false);
  });

  it("rejects a non-date string", () => {
    expect(isValidJournalDate("not-a-date")).toBe(false);
    expect(isValidJournalDate("")).toBe(false);
  });

  it("rejects wrong format", () => {
    expect(isValidJournalDate("15/01/2024")).toBe(false);
    expect(isValidJournalDate("2024/01/15")).toBe(false);
    expect(isValidJournalDate("20240115")).toBe(false);
  });

  it("accepts a valid leap year date", () => {
    expect(isValidJournalDate("2024-02-29")).toBe(true);
  });

  it("rejects Feb 29 on a non-leap year", () => {
    expect(isValidJournalDate("2023-02-29")).toBe(false);
  });

  it("accepts Dec 31", () => {
    expect(isValidJournalDate("2024-12-31")).toBe(true);
  });

  it("rejects garbage input", () => {
    expect(isValidJournalDate("abcd-ef-gh")).toBe(false);
    expect(isValidJournalDate("2024-1-5")).toBe(false);
    expect(isValidJournalDate("2024-01-5")).toBe(false);
  });
});

describe("isSameJournalMonth", () => {
  it("returns true for same month", () => {
    expect(isSameJournalMonth("2024-03-15", dayjs("2024-03-01"))).toBe(true);
  });

  it("returns false for different month", () => {
    expect(isSameJournalMonth("2024-03-15", dayjs("2024-04-01"))).toBe(false);
  });

  it("returns false for an invalid date string", () => {
    expect(isSameJournalMonth("invalid", dayjs("2024-03-01"))).toBe(false);
  });

  it("returns true for same month different year", () => {
    expect(isSameJournalMonth("2023-03-15", dayjs("2024-03-01"))).toBe(false);
  });
});

describe("isExecutedTransaction", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns true for a past date", () => {
    expect(isExecutedTransaction(makeTx("2020-01-01"))).toBe(true);
  });

  it("returns true for today", () => {
    const today = dayjs().format(journalDateFormat);
    expect(isExecutedTransaction(makeTx(today))).toBe(true);
  });

  it("returns false for a future date", () => {
    const future = dayjs().add(10, "day").format(journalDateFormat);
    expect(isExecutedTransaction(makeTx(future))).toBe(false);
  });

  it("returns false for an invalid date", () => {
    expect(isExecutedTransaction(makeTx("not-a-date"))).toBe(false);
  });
});

describe("normalizeDate", () => {
  it("converts a Dayjs object to YYYY-MM-DD", () => {
    expect(normalizeDate(dayjs("2026-05-31"))).toBe("2026-05-31");
  });

  it("passes through a valid date string", () => {
    expect(normalizeDate("2024-01-15")).toBe("2024-01-15");
  });

  it("passes through a non-date string unchanged", () => {
    expect(normalizeDate("hello")).toBe("hello");
  });

  it("returns undefined for null", () => {
    expect(normalizeDate(null)).toBeUndefined();
  });

  it("returns undefined for undefined", () => {
    expect(normalizeDate(undefined)).toBeUndefined();
  });

  it("returns undefined for an empty string", () => {
    expect(normalizeDate("")).toBeUndefined();
  });

  it("returns undefined for whitespace-only string", () => {
    expect(normalizeDate("   ")).toBeUndefined();
  });

  it("trims whitespace from a string value", () => {
    expect(normalizeDate("  2026-05-31  ")).toBe("2026-05-31");
  });
});


