import { describe, expect, it } from "vitest";
import { searchCommandPalette, type CommandPaletteCommand } from "./search";
import type { JournalTransaction } from "../types";

const commands: CommandPaletteCommand[] = [
  { id: "new", label: "New transaction", shortcut: "⌘N", keywords: ["create"] },
  { id: "settings", label: "Settings", shortcut: "⌘4" },
];

function tx(id: string, description: string, account = "expenses:groceries"): JournalTransaction {
  return {
    id,
    sourceFile: "main.journal",
    date: "2026-05-19",
    status: "",
    code: "",
    description,
    postings: [
      { account, amount: "10", commodity: "€", comment: "weekly", raw: "" },
    ],
    display: {
      account,
      amount: "€10",
      formatted: "€10,00",
      kind: "expense",
      tint: "negative",
      flow: { from: ["assets:bank"], to: [account] },
    },
    raw: "",
    startLine: 1,
    endLine: 2,
  };
}

describe("searchCommandPalette", () => {
  it("returns commands when query is empty", () => {
    const results = searchCommandPalette({
      query: "",
      commands,
      accounts: ["assets:bank"],
      transactions: [tx("1", "Groceries")],
    });

    expect(results.map((result) => result.id)).toEqual(["new", "settings"]);
  });

  it("ranks commands before accounts and transactions", () => {
    const results = searchCommandPalette({
      query: "new",
      commands,
      accounts: ["assets:new-bank"],
      transactions: [tx("1", "New laptop")],
    });

    expect(results[0]).toMatchObject({ type: "command", id: "new" });
  });

  it("finds accounts", () => {
    const results = searchCommandPalette({
      query: "groceries",
      commands,
      accounts: ["expenses:groceries"],
      transactions: [],
    });

    expect(results).toContainEqual({
      type: "account",
      id: "expenses:groceries",
      account: "expenses:groceries",
    });
  });

  it("finds transactions by description and posting account", () => {
    const results = searchCommandPalette({
      query: "ofantina",
      commands,
      accounts: [],
      transactions: [tx("1", "Agricola Ofantina")],
    });

    expect(results[0]).toMatchObject({ type: "transaction", id: "1" });
  });

  it("honors the result limit", () => {
    const results = searchCommandPalette({
      query: "assets",
      commands,
      accounts: ["assets:a", "assets:b", "assets:c"],
      transactions: [],
      limit: 2,
    });

    expect(results).toHaveLength(2);
  });
});
