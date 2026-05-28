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
    case "movement":
      return [
        { account: cashAccount, amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
        { account: suggestions.defaultExpenseAccount || findAccountByRoot(accounts, ["expenses", "expense"]) || transferAccount, amount: "", commodity: "", unitPrice: "", comment: "" },
      ];
    case "investment":
      return [
        { account: suggestions.defaultInvestmentAccount, amount: "", commodity: investmentCommodity, unitPrice: "", comment: "" },
        { account: cashAccount, amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
      ];
    case "advanced":
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


