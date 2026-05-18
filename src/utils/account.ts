import type { AccountSummary, JournalTransaction, PostingInput, TransactionType, AutocompleteSuggestions } from "../types";

export function findAccountByRoot(accounts: string[], roots: string[], offset = 0): string {
  const matches = accounts.filter((account) =>
    roots.some((root) => account.toLowerCase().startsWith(root.toLowerCase())),
  );
  return matches[offset] ?? matches[0] ?? "";
}

export function transactionTemplatePostings(
  type: TransactionType,
  suggestions: AutocompleteSuggestions,
  defaultCommodity: string,
): PostingInput[] {
  const accounts = suggestions.accounts;
  const fallbackAssetAccount = findAccountByRoot(accounts, ["assets", "asset"]);
  const fallbackLiabilityAccount = findAccountByRoot(accounts, ["liabilities", "liability"]);
  const fallbackCashAccount = fallbackAssetAccount || fallbackLiabilityAccount;
  const cashAccount = suggestions.defaultCashAccount || fallbackCashAccount;
  const transferAccount = suggestions.defaultTransferAccount || findAccountByRoot(accounts, ["assets", "asset", "liabilities", "liability"], 1) || cashAccount;
  const investmentCommodity = suggestions.defaultInvestmentCommodity || "";

  switch (type) {
    case "expense":
      return [
        { account: suggestions.defaultExpenseAccount || findAccountByRoot(accounts, ["expenses", "expense"]), amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
        { account: cashAccount, amount: "", commodity: "", unitPrice: "", comment: "" },
      ];
    case "income":
      return [
        { account: cashAccount, amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
        { account: suggestions.defaultIncomeAccount || findAccountByRoot(accounts, ["income", "revenue"]), amount: "", commodity: "", unitPrice: "", comment: "" },
      ];
    case "transfer":
      return [
        { account: cashAccount, amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
        { account: transferAccount, amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
      ];
    case "investment":
      return [
        { account: suggestions.defaultInvestmentAccount, amount: "", commodity: investmentCommodity, unitPrice: "", comment: "" },
        { account: cashAccount, amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
      ];
    case "custom":
      return [
        { account: "", amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
        { account: "", amount: "", commodity: "", unitPrice: "", comment: "" },
      ];
  }
}

export function transactionIncludesAccount(transaction: JournalTransaction, account: string): boolean {
  return transaction.postings.some(
    (posting) => posting.account.trim().toLowerCase() === account.toLowerCase(),
  );
}

export function collectAccounts(
  transactions: JournalTransaction[],
  visibleTransactions: JournalTransaction[],
): AccountSummary[] {
  const accountNames = new Map<string, string>();

  transactions.forEach((transaction) => {
    transaction.postings.forEach((posting) => {
      const account = posting.account.trim();
      if (account) {
        accountNames.set(account.toLowerCase(), account);
      }
    });
  });

  return Array.from(accountNames.values())
    .map((account) => {
      const accountTransactions = visibleTransactions.filter((transaction) =>
        transactionIncludesAccount(transaction, account),
      );

      return {
        account,
        transactions: accountTransactions.length,
        accountTransactions,
      };
    })
    .sort((left, right) => left.account.localeCompare(right.account));
}

const groupOrder = ["assets", "liabilities", "equity", "income", "expenses"];

export function groupAccounts(
  accounts: AccountSummary[],
): { group: string; items: AccountSummary[] }[] {
  const map = new Map<string, AccountSummary[]>();
  for (const a of accounts) {
    const root = a.account.split(":")[0].toLowerCase();
    const key = groupOrder.includes(root) ? root : "other";
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(a);
  }
  const result: { group: string; items: AccountSummary[] }[] = [];
  for (const g of groupOrder) {
    const items = map.get(g);
    if (items) result.push({ group: g, items });
  }
  const other = map.get("other");
  if (other) result.push({ group: "other", items: other });
  return result;
}
