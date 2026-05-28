import type { JournalTransaction } from "../types";

export type CommandPaletteCommand = {
  id: string;
  label: string;
  shortcut?: string;
  keywords?: string[];
};

export type CommandPaletteResult =
  | {
    type: "command";
    id: string;
    label: string;
    shortcut?: string;
  }
  | {
    type: "account";
    id: string;
    account: string;
  }
  | {
    type: "transaction";
    id: string;
    transaction: JournalTransaction;
  };

type ScoredResult = {
  score: number;
  result: CommandPaletteResult;
};

function normalize(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, " ");
}

type ScoredAccount = { score: number; account: string };
type ScoredTransaction = { score: number; transaction: JournalTransaction };

function scoreMatch(haystack: string, needle: string): number {
  const normalizedHaystack = normalize(haystack);
  const normalizedNeedle = normalize(needle);
  if (!normalizedNeedle) return 0;

  // Exact match
  if (normalizedHaystack === normalizedNeedle) return 100;

  // Starts with
  if (normalizedHaystack.startsWith(normalizedNeedle)) return 80;

  // Word starts with
  const words = normalizedHaystack.split(/[\s\-_:/]+/);
  for (const word of words) {
    if (word.startsWith(normalizedNeedle)) return 60;
  }

  // Contains
  if (normalizedHaystack.includes(normalizedNeedle)) return 40;

  // Fuzzy: all characters appear in order
  let haystackIndex = 0;
  for (let i = 0; i < normalizedNeedle.length; i++) {
    haystackIndex = normalizedHaystack.indexOf(normalizedNeedle[i], haystackIndex);
    if (haystackIndex === -1) return 0;
    haystackIndex++;
  }
  return 20;
}

function scoreAccounts(accounts: string[], normalizedQuery: string): ScoredAccount[] {
  const scored: ScoredAccount[] = [];
  for (const account of accounts) {
    const accountScore = scoreMatch(account, normalizedQuery);
    if (accountScore > 0) {
      scored.push({ score: accountScore, account });
    }
  }
  return scored;
}

function scoreTransactions(transactions: JournalTransaction[], normalizedQuery: string): ScoredTransaction[] {
  const scored: ScoredTransaction[] = [];
  for (const tx of transactions) {
    const descScore = scoreMatch(tx.description, normalizedQuery);
    let bestScore = descScore;
    for (const posting of tx.postings) {
      const postingScore = scoreMatch(posting.account, normalizedQuery);
      if (postingScore > bestScore) bestScore = postingScore;
    }
    if (bestScore > 0) {
      scored.push({ score: bestScore, transaction: tx });
    }
  }
  return scored;
}

export function searchCommandPalette({
  query,
  commands,
  accounts,
  transactions,
  limit = 12,
}: {
  query: string;
  commands: CommandPaletteCommand[];
  accounts: string[];
  transactions: JournalTransaction[];
  limit?: number;
}): CommandPaletteResult[] {
  const normalizedQuery = normalize(query);

  // When query is empty, return all commands
  if (!normalizedQuery) {
    return commands.slice(0, limit).map((cmd) => ({
      type: "command" as const,
      id: cmd.id,
      label: cmd.label,
      shortcut: cmd.shortcut,
    }));
  }

  const scored: ScoredResult[] = [];

  // Score commands
  for (const cmd of commands) {
    let bestScore = scoreMatch(cmd.label, normalizedQuery);
    if (cmd.keywords) {
      for (const keyword of cmd.keywords) {
        const keywordScore = scoreMatch(keyword, normalizedQuery);
        if (keywordScore > bestScore) bestScore = keywordScore;
      }
    }
    if (bestScore > 0) {
      scored.push({
        score: bestScore + 10, // Slight boost for commands
        result: {
          type: "command",
          id: cmd.id,
          label: cmd.label,
          shortcut: cmd.shortcut,
        },
      });
    }
  }

  // Score accounts
  for (const { score, account } of scoreAccounts(accounts, normalizedQuery)) {
    scored.push({ score, result: { type: "account" as const, id: account, account } });
  }

  // Score transactions
  for (const { score, transaction } of scoreTransactions(transactions, normalizedQuery)) {
    scored.push({
      score: score + 5, // Slight boost for transactions over accounts
      result: { type: "transaction" as const, id: transaction.id, transaction },
    });
  }

  // Sort by score descending, then by type priority
  scored.sort((a, b) => {
    if (b.score !== a.score) return b.score - a.score;
    const typeOrder: Record<string, number> = { command: 0, account: 1, transaction: 2 };
    return (typeOrder[a.result.type] ?? 3) - (typeOrder[b.result.type] ?? 3);
  });

  return scored.slice(0, limit).map((s) => s.result);
}

export type SearchResult =
  | {
    type: "account";
    id: string;
    account: string;
  }
  | {
    type: "transaction";
    id: string;
    transaction: JournalTransaction;
  };

export function searchJournalData({
  query,
  accounts,
  transactions,
  limit = 10,
}: {
  query: string;
  accounts: string[];
  transactions: JournalTransaction[];
  limit?: number;
}): SearchResult[] {
  const normalizedQuery = normalize(query);
  if (!normalizedQuery) return [];

  const scored: { score: number; result: SearchResult }[] = [];

  for (const { score, account } of scoreAccounts(accounts, normalizedQuery)) {
    scored.push({ score, result: { type: "account", id: account, account } });
  }

  for (const { score, transaction } of scoreTransactions(transactions, normalizedQuery)) {
    scored.push({ score, result: { type: "transaction", id: transaction.id, transaction } });
  }

  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, limit).map((s) => s.result);
}
