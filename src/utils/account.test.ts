import { describe, it, expect } from "vitest";
import {
  findAccountByRoot,
  transactionTemplatePostings,
  transactionIncludesAccount,
  collectAccounts,
} from "./account";
import type {
  AutocompleteSuggestions,
  JournalTransaction,
} from "../types";

function makeTx(id: string, postings: { account: string; amount: string; commodity: string; comment?: string }[]): JournalTransaction {
  return {
    id,
    sourceFile: "main.journal",
    date: "2024-01-15",
    status: "*",
    code: "",
    description: "test tx",
    postings: postings.map((p) => ({
      account: p.account,
      amount: p.amount,
      commodity: p.commodity,
      comment: p.comment ?? "",
      raw: "",
    })),
    display: { account: "", amount: "", kind: "" },
    raw: "",
    startLine: 1,
    endLine: 1,
  };
}

const defaultSuggestions: AutocompleteSuggestions = {
  codes: [],
  descriptions: [],
  accounts: [
    "assets:bank:checking",
    "assets:cash",
    "expenses:food",
    "expenses:rent",
    "income:salary",
    "liabilities:credit-card",
    "investments:stocks",
    "revenues:consulting",
  ],
  commodities: ["EUR", "USD"],
  defaultCommodity: "EUR",
  defaultCashAccount: "assets:bank:checking",
  defaultExpenseAccount: "expenses:food",
  defaultIncomeAccount: "income:salary",
  defaultTransferAccount: "assets:cash",
  defaultInvestmentAccount: "investments:stocks",
  defaultInvestmentCommodity: "AAPL",
};

describe("findAccountByRoot", () => {
  const accounts = ["assets:bank", "expenses:food", "liabilities:credit"];

  it("finds account matching a root prefix", () => {
    expect(findAccountByRoot(accounts, ["assets"])).toBe("assets:bank");
  });

  it("is case-insensitive", () => {
    expect(findAccountByRoot(accounts, ["EXPENSES"])).toBe("expenses:food");
  });

  it("returns empty string when no match", () => {
    expect(findAccountByRoot(accounts, ["equity"])).toBe("");
  });

  it("returns the first match when multiple roots match different accounts", () => {
    expect(findAccountByRoot(accounts, ["liabilities", "assets"])).toBe("assets:bank");
  });

  it("returns match at given offset", () => {
    const manyAccounts = ["assets:checking", "assets:savings", "assets:cash"];
    expect(findAccountByRoot(manyAccounts, ["assets"], 0)).toBe("assets:checking");
    expect(findAccountByRoot(manyAccounts, ["assets"], 1)).toBe("assets:savings");
    expect(findAccountByRoot(manyAccounts, ["assets"], 2)).toBe("assets:cash");
  });

  it("falls back to first match when offset exceeds bounds", () => {
    expect(findAccountByRoot(accounts, ["assets"], 10)).toBe("assets:bank");
  });
});

describe("transactionTemplatePostings", () => {
  it("returns expense postings with expense account and cash account", () => {
    const result = transactionTemplatePostings("expense", defaultSuggestions, "EUR");
    expect(result[0]).toEqual({ account: "expenses:food", amount: "", commodity: "EUR", unitPrice: "", comment: "" });
    expect(result[1]).toEqual({ account: "assets:bank:checking", amount: "", commodity: "", unitPrice: "", comment: "" });
  });

  it("returns income postings with cash and income account", () => {
    const result = transactionTemplatePostings("income", defaultSuggestions, "EUR");
    expect(result[0]).toEqual({ account: "assets:bank:checking", amount: "", commodity: "EUR", unitPrice: "", comment: "" });
    expect(result[1]).toEqual({ account: "income:salary", amount: "", commodity: "", unitPrice: "", comment: "" });
  });

  it("returns transfer postings with two accounts", () => {
    const result = transactionTemplatePostings("transfer", defaultSuggestions, "EUR");
    expect(result[0]).toEqual({ account: "assets:bank:checking", amount: "", commodity: "EUR", unitPrice: "", comment: "" });
    expect(result[1]).toEqual({ account: "assets:cash", amount: "", commodity: "EUR", unitPrice: "", comment: "" });
  });

  it("returns investment postings", () => {
    const result = transactionTemplatePostings("investment", defaultSuggestions, "EUR");
    expect(result[0]).toEqual({ account: "investments:stocks", amount: "", commodity: "AAPL", unitPrice: "", comment: "" });
    expect(result[1]).toEqual({ account: "assets:bank:checking", amount: "", commodity: "EUR", unitPrice: "", comment: "" });
  });

  it("returns empty custom postings", () => {
    const result = transactionTemplatePostings("custom", defaultSuggestions, "EUR");
    expect(result[0]).toEqual({ account: "", amount: "", commodity: "EUR", unitPrice: "", comment: "" });
    expect(result[1]).toEqual({ account: "", amount: "", commodity: "", unitPrice: "", comment: "" });
  });

  it("falls back to account root matching when defaults are empty", () => {
    const emptySuggestions: AutocompleteSuggestions = {
      ...defaultSuggestions,
      defaultCashAccount: "",
      defaultExpenseAccount: "",
      defaultIncomeAccount: "",
      defaultTransferAccount: "",
      defaultInvestmentAccount: "",
      defaultInvestmentCommodity: "",
    };
    const result = transactionTemplatePostings("expense", emptySuggestions, "EUR");
    expect(result[0].account).toBe("expenses:food");
    expect(result[1].account).toBe("assets:bank:checking");
  });
});

describe("transactionIncludesAccount", () => {
  const tx = makeTx("1", [
    { account: "assets:bank", amount: "-10", commodity: "EUR" },
    { account: "expenses:food", amount: "10", commodity: "EUR" },
  ]);

  it("returns true for an account present in postings", () => {
    expect(transactionIncludesAccount(tx, "assets:bank")).toBe(true);
  });

  it("is case-insensitive", () => {
    expect(transactionIncludesAccount(tx, "ASSETS:BANK")).toBe(true);
  });

  it("returns false for an account not in postings", () => {
    expect(transactionIncludesAccount(tx, "income:salary")).toBe(false);
  });

  it("handles whitespace in account names", () => {
    const txWithSpace = makeTx("2", [
      { account: "  assets:bank  ", amount: "-10", commodity: "EUR" },
    ]);
    expect(transactionIncludesAccount(txWithSpace, "assets:bank")).toBe(true);
  });
});

describe("collectAccounts", () => {
  const allTx = [
    makeTx("1", [
      { account: "assets:bank", amount: "-100", commodity: "EUR" },
      { account: "expenses:food", amount: "100", commodity: "EUR" },
    ]),
    makeTx("2", [
      { account: "assets:bank", amount: "-50", commodity: "EUR" },
      { account: "expenses:rent", amount: "50", commodity: "EUR" },
    ]),
    makeTx("3", [
      { account: "income:salary", amount: "1000", commodity: "EUR" },
      { account: "assets:bank", amount: "1000", commodity: "EUR" },
    ]),
  ];

  it("collects all unique accounts sorted alphabetically", () => {
    const result = collectAccounts(allTx, allTx);
    const names = result.map((a) => a.account);
    expect(names).toEqual(["assets:bank", "expenses:food", "expenses:rent", "income:salary"]);
  });

  it("counts transactions per account correctly", () => {
    const result = collectAccounts(allTx, allTx);
    const bank = result.find((a) => a.account === "assets:bank");
    expect(bank?.transactions).toBe(3);
    const food = result.find((a) => a.account === "expenses:food");
    expect(food?.transactions).toBe(1);
  });

  it("filters by visibleTransactions", () => {
    const visible = allTx.slice(0, 2);
    const result = collectAccounts(allTx, visible);
    const bank = result.find((a) => a.account === "assets:bank");
    expect(bank?.transactions).toBe(2);
    const salary = result.find((a) => a.account === "income:salary");
    expect(salary?.transactions).toBe(0);
  });

  it("handles case-insensitive duplicate account names", () => {
    const txs = [
      makeTx("1", [{ account: "Assets:Bank", amount: "100", commodity: "EUR" }]),
      makeTx("2", [{ account: "assets:bank", amount: "200", commodity: "EUR" }]),
    ];
    const result = collectAccounts(txs, txs);
    expect(result.length).toBe(1);
    expect(result[0].transactions).toBe(2);
  });

  it("returns empty array for no transactions", () => {
    expect(collectAccounts([], [])).toEqual([]);
  });
});
