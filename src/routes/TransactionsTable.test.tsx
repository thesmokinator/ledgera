import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { accountFlow } from "./TransactionsTable";
import type { JournalTransaction } from "./types";

function transactionWithFlow(
  kind: string,
  from: string[],
  to: string[],
): JournalTransaction {
  return {
    id: "test:1",
    sourceFile: "test.journal",
    date: "2026-05-18",
    status: "",
    code: "",
    description: "Test transaction",
    postings: [],
    display: {
      account: to[0] ?? from[0] ?? "-",
      amount: "5.20",
      formatted: "5,20",
      kind,
      flow: { from, to },
    },
    raw: "",
    startLine: 1,
    endLine: 3,
  };
}

describe("accountFlow", () => {
  it("renders backend-provided income flow from source to destination", () => {
    const { container } = render(<>{accountFlow(transactionWithFlow("income", ["income:other"], ["assets:bank:postepay"]))}</>);

    expect(container.textContent).toContain("income:other");
    expect(container.textContent).toContain("assets:bank:postepay");
  });

  it("renders split flows without reparsing postings in the frontend", () => {
    const { container } = render(<>{accountFlow(transactionWithFlow("expense", ["assets:bank"], ["expenses:food", "expenses:home"]))}</>);

    expect(container.textContent).toContain("assets:bank");
    expect(screen.getByText("expenses:food")).not.toBeNull();
    expect(screen.getByText("expenses:home")).not.toBeNull();
    expect(container.textContent).not.toContain("expenses:food, expenses:home");
  });

  it("falls back to display account when backend flow is empty", () => {
    render(<>{accountFlow(transactionWithFlow("unknown", [], []))}</>);

    expect(screen.getByText("-")).not.toBeNull();
  });
});
