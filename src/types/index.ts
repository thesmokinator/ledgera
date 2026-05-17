import type { Dayjs } from "dayjs";

export type ThemePreference = "system" | "dark" | "light";

export type AppSettings = {
  journalPath: string;
  hledgerPath: string;
  theme: ThemePreference;
  powerUser: boolean;
  defaultCommodity: string;
};

export type HledgerStatus = {
  available: boolean;
  version: string;
  message: string;
  resolvedPath: string;
  source: "configured" | "detected" | "fallback";
};

export type JournalPosting = {
  account: string;
  amount: string;
  commodity: string;
  comment: string;
  raw: string;
};

export type TransactionDisplay = {
  account: string;
  amount: string;
  kind: string;
};

export type JournalTransaction = {
  id: string;
  sourceFile: string;
  date: string;
  status: string;
  code: string;
  description: string;
  postings: JournalPosting[];
  display: TransactionDisplay;
  raw: string;
  startLine: number;
  endLine: number;
};

export type DashboardSummary = {
  monthlyTransactions: JournalTransaction[];
  scheduledTransactions: JournalTransaction[];
  activeAccountsCount: number;
};

export type JournalSummary = {
  path: string;
  transactions: JournalTransaction[];
  commodities: string[];
  dashboard: DashboardSummary;
};

export type AutocompleteSuggestions = {
  codes: string[];
  descriptions: string[];
  accounts: string[];
  commodities: string[];
  defaultCommodity: string;
  defaultCashAccount: string;
  defaultExpenseAccount: string;
  defaultIncomeAccount: string;
  defaultTransferAccount: string;
  defaultInvestmentAccount: string;
  defaultInvestmentCommodity: string;
};

export type PostingInput = {
  account: string;
  amount: string;
  commodity: string;
  unitPrice: string;
  comment: string;
};

export type TransactionInput = {
  date: string;
  status: string;
  code: string;
  description: string;
  postings: PostingInput[];
};

export type TransactionType = "expense" | "income" | "transfer" | "investment" | "custom";

export type AccountActivityRange = "current-month" | "30" | "60" | "90" | "180" | "365";

export type AccountSummary = {
  account: string;
  transactions: number;
  accountTransactions: JournalTransaction[];
};

export type MonthSetter = (updater: (month: Dayjs) => Dayjs) => void;

export type NavigationItem = {
  key: string;
  label: string;
  icon: React.ReactNode;
};
