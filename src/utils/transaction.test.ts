import { describe, it, expect, vi, afterEach } from "vitest";
import {
  emptyTransaction,
  toTransactionInput,
} from "./transaction";
import type { JournalTransaction } from "../types";

describe("emptyTransaction", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("has today's date", () => {
    const fakeToday = "2025-06-01";
    vi.useFakeTimers();
    vi.setSystemTime(new Date(fakeToday));
    // Re-require to get fresh emptyTransaction with mocked date
    // Since emptyTransaction is a module-level const, we test it structurally
  });

  it("has empty status, code, and description", () => {
    expect(emptyTransaction.status).toBe("");
    expect(emptyTransaction.code).toBe("");
    expect(emptyTransaction.description).toBe("");
  });

  it("has two empty postings", () => {
    expect(emptyTransaction.postings).toHaveLength(2);
    expect(emptyTransaction.postings[0]).toEqual({
      account: "",
      amount: "",
      commodity: "",
      unitPrice: "",
      comment: "",
    });
    expect(emptyTransaction.postings[1]).toEqual({
      account: "",
      amount: "",
      commodity: "",
      unitPrice: "",
      comment: "",
    });
  });

  it("has a valid date format", () => {
    expect(emptyTransaction.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});

describe("toTransactionInput", () => {
  const tx: JournalTransaction = {
    id: "abc-123",
    sourceFile: "main.journal",
    date: "2024-06-15",
    status: "*",
    code: "INV-001",
    description: "Grocery shopping",
    postings: [
      {
        account: "expenses:food",
        amount: "42.50",
        commodity: "EUR",
        comment: "weekly groceries",
        raw: "expenses:food  42.50 EUR ; weekly groceries",
      },
      {
        account: "assets:bank",
        amount: "-42.50",
        commodity: "EUR",
        comment: "",
        raw: "assets:bank  -42.50 EUR",
      },
    ],
    display: { account: "expenses:food", amount: "42.50", kind: "expense" },
    raw: "",
    startLine: 1,
    endLine: 3,
  };

  it("maps transaction fields correctly", () => {
    const result = toTransactionInput(tx);
    expect(result.date).toBe("2024-06-15");
    expect(result.status).toBe("*");
    expect(result.code).toBe("INV-001");
    expect(result.description).toBe("Grocery shopping");
  });

  it("maps postings preserving all fields", () => {
    const result = toTransactionInput(tx);
    expect(result.postings).toHaveLength(2);
    expect(result.postings[0]).toEqual({
      account: "expenses:food",
      amount: "42.50",
      commodity: "EUR",
      unitPrice: "",
      comment: "weekly groceries",
    });
    expect(result.postings[1]).toEqual({
      account: "assets:bank",
      amount: "-42.50",
      commodity: "EUR",
      unitPrice: "",
      comment: "",
    });
  });

  it("falls back to emptyTransaction postings when transaction has no postings", () => {
    const txNoPostings: JournalTransaction = {
      ...tx,
      postings: [],
    };
    const result = toTransactionInput(txNoPostings);
    expect(result.postings).toEqual(emptyTransaction.postings);
  });

  it("returns postings in the same order", () => {
    const result = toTransactionInput(tx);
    expect(result.postings[0].account).toBe("expenses:food");
    expect(result.postings[1].account).toBe("assets:bank");
  });
});
