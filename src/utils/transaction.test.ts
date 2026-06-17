import { describe, it, expect, vi, afterEach } from "vitest";
import {
  createEmptyTransaction,
  emptyTransaction,
  toTransactionInput,
  parseAmountValue,
  parseUnitPrice,
  autoCalculateBalancingAmounts,
  makeOutgoingAmount,
  makeMovementInputAmount,
} from "./transaction";
import { classSuffix } from "../components/Amount";
import type { JournalTransaction } from "../types";

describe("emptyTransaction", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("creates a transaction dated today", () => {
    const fakeToday = "2025-06-01";
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2025-06-01T12:00:00"));

    expect(createEmptyTransaction().date).toBe(fakeToday);
  });

  it("has movement mode with empty status, code, and description", () => {
    expect(emptyTransaction.mode).toBe("movement");
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
        unitPrice: "",
        comment: "weekly groceries",
        raw: "expenses:food  42.50 EUR ; weekly groceries",
      },
      {
        account: "assets:bank",
        amount: "-42.50",
        commodity: "EUR",
        unitPrice: "",
        comment: "",
        raw: "assets:bank  -42.50 EUR",
      },
    ],
    display: {
      account: "expenses:food",
      amount: "42.50",
      formatted: "42.50",
      kind: "expense",
      tint: "negative",
      flow: { from: ["assets:bank"], to: ["expenses:food"] },
    },
    raw: "",
    startLine: 1,
    endLine: 3,
  };

  it("maps transaction fields correctly", () => {
    const result = toTransactionInput(tx);
    expect(result.mode).toBe("advanced");
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

describe("parseAmountValue", () => {
  it("parses a plain number", () => {
    expect(parseAmountValue("42.50")).toBe(42.50);
  });

  it("parses number with comma as decimal", () => {
    expect(parseAmountValue("42,50")).toBe(42.50);
  });

  it("extracts number from string with commodity", () => {
    expect(parseAmountValue("42.50 EUR")).toBe(42.50);
  });

  it("handles negative numbers", () => {
    expect(parseAmountValue("-1500")).toBe(-1500);
  });

  it("returns 0 for empty string", () => {
    expect(parseAmountValue("")).toBe(0);
  });

  it("returns 0 for non-numeric string", () => {
    expect(parseAmountValue("abc")).toBe(0);
  });
});

describe("parseUnitPrice", () => {
  it("parses price value and commodity", () => {
    const result = parseUnitPrice("150 EUR");
    expect(result.value).toBe(150);
    expect(result.commodity).toBe("EUR");
  });

  it("parses price with decimal", () => {
    const result = parseUnitPrice("148.70 EUR");
    expect(result.value).toBe(148.70);
    expect(result.commodity).toBe("EUR");
  });

  it("returns empty commodity if only number", () => {
    const result = parseUnitPrice("150");
    expect(result.value).toBe(150);
    expect(result.commodity).toBe("");
  });
});

describe("makeOutgoingAmount", () => {
  it("prepends minus sign to a plain number", () => {
    expect(makeOutgoingAmount("100")).toBe("-100");
  });

  it("trims whitespace", () => {
    expect(makeOutgoingAmount("  100  ")).toBe("-100");
  });

  it("keeps already negative value unchanged", () => {
    expect(makeOutgoingAmount("-100")).toBe("-100");
    expect(makeOutgoingAmount("-0")).toBe("-0");
  });

  it("returns empty string as-is", () => {
    expect(makeOutgoingAmount("")).toBe("");
    expect(makeOutgoingAmount("   ")).toBe("");
  });
});

describe("classSuffix", () => {
  it("capitalizes first letter", () => {
    expect(classSuffix("expense")).toBe("Expense");
    expect(classSuffix("income")).toBe("Income");
    expect(classSuffix("transfer")).toBe("Transfer");
  });

  it("handles single character", () => {
    expect(classSuffix("a")).toBe("A");
  });

  it("handles empty string", () => {
    expect(classSuffix("")).toBe("");
  });
});

describe("makeMovementInputAmount", () => {
  it("removes leading minus sign", () => {
    expect(makeMovementInputAmount("-100")).toBe("100");
  });

  it("keeps positive value unchanged", () => {
    expect(makeMovementInputAmount("100")).toBe("100");
  });

  it("trims whitespace", () => {
    expect(makeMovementInputAmount("  -50  ")).toBe("50");
  });

  it("handles zero", () => {
    expect(makeMovementInputAmount("0")).toBe("0");
    expect(makeMovementInputAmount("-0")).toBe("0");
  });
});

describe("autoCalculateBalancingAmounts", () => {
  it("calculates balancing amount from unit price", () => {
    const postings = [
      { account: "a", amount: "10", commodity: "VWCE", unitPrice: "150 EUR", comment: "" },
      { account: "b", amount: "", commodity: "EUR", unitPrice: "", comment: "" },
    ];
    const result = autoCalculateBalancingAmounts(postings, "EUR");
    expect(result[1].amount).toBe("-1500.00");
    expect(result[1].commodity).toBe("EUR");
    expect(result[0].unitPrice).toBe("150 EUR");
  });

  it("enriches unitPrice with balancing commodity when missing", () => {
    const postings = [
      { account: "a", amount: "205", commodity: "XEON", unitPrice: "149,38", comment: "" },
      { account: "b", amount: "", commodity: "EUR", unitPrice: "", comment: "" },
    ];
    const result = autoCalculateBalancingAmounts(postings, "EUR");
    expect(result[1].amount).toBe("-30622.90");
    expect(result[1].commodity).toBe("EUR");
    expect(result[0].unitPrice).toBe("149,38 EUR");
  });

  it("does not modify unitPrice when it already contains commodity", () => {
    const postings = [
      { account: "a", amount: "10", commodity: "VWCE", unitPrice: "150 USD", comment: "" },
      { account: "b", amount: "", commodity: "USD", unitPrice: "", comment: "" },
    ];
    const result = autoCalculateBalancingAmounts(postings, "USD");
    expect(result[0].unitPrice).toBe("150 USD");
    expect(result[1].commodity).toBe("USD");
  });

  it("does nothing if no posting has unitPrice", () => {
    const postings = [
      { account: "a", amount: "10", commodity: "VWCE", unitPrice: "", comment: "" },
      { account: "b", amount: "", commodity: "EUR", unitPrice: "", comment: "" },
    ];
    const result = autoCalculateBalancingAmounts(postings, "EUR");
    expect(result).toEqual(postings);
  });
});
