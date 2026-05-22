import type { Dayjs } from "dayjs";

export type ThemePreference = "system" | "dark" | "light";

export type AppView = "transactions" | "accounts" | "balances" | "settings" | "logs";

export type AppSettings = {
  journalPath: string;
  hledgerPath: string;
  theme: ThemePreference;
  powerUser: boolean;
  defaultCommodity: string;
  fetchPrices: boolean;
  commoditySymbols: string;
  excludeBalances: string;
  includeInvestments: string;
  prefillPostings: boolean;
};

export type HledgerStatus = {
  available: boolean;
  version: string;
  message: string;
  resolvedPath: string;
  source: "configured" | "detected" | "fallback";
};

export type UpdateStatus = {
  currentVersion: string;
  latestVersion: string | null;
  available: boolean;
  releaseUrl: string | null;
  releaseName: string | null;
  publishedAt: string | null;
  checkedAt: string | null;
  source: "cache" | "network";
  error: string | null;
};

export type JournalPosting = {
  account: string;
  amount: string;
  commodity: string;
  comment: string;
  raw: string;
};

export type TransactionFlow = {
  from: string[];
  to: string[];
};

export type AmountTint = "negative" | "positive" | "neutral";

export type AmountStyle = {
  decimalMark: string;
  digitSeparator: string;
  digitGroups: number[];
  precision: number;
  commodityPosition: string;
  commoditySpaced: boolean;
};

export type TransactionDisplay = {
  account: string;
  amount: string;
  formatted: string;
  kind: string;
  tint: AmountTint;
  flow: TransactionFlow;
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

export type SearchMatchRange = {
  start: number;
  end: number;
};

export type JournalSearchMatch = {
  field: "description" | "account" | "comment";
  value: string;
  ranges: SearchMatchRange[];
  postingIndex: number | null;
};

export type JournalSearchResult = {
  transaction: JournalTransaction;
  matches: JournalSearchMatch[];
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
  fileCount: number;
  totalSizeBytes: number;
  amountStyle: AmountStyle;
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

export type TransactionType = "movement" | "investment" | "advanced";

export type TransactionInput = {
  mode: TransactionType;
  date: string;
  status: string;
  code: string;
  description: string;
  postings: PostingInput[];
};

export type AppFieldError = {
  path: string[];
  message: string;
};

export type AppError = {
  code: string;
  message: string;
  details?: string;
  fieldErrors?: AppFieldError[];
};

export type LogEntry = {
  ts: string;
  level: "info" | "warn" | "error";
  code: string;
  message: string;
  details?: string;
};

export type Balance = {
  account: string;
  amount: number;
  commodity: string;
  formatted: string;
  tint: AmountTint;
};

export type AccountActivityRange = "current-month" | "30" | "60" | "90" | "180" | "365";

export type AccountOverviewRow = {
  account: string;
  balance: Balance | null;
  activityCount: number;
  transactions: JournalTransaction[];
};

export type AccountOverviewGroup = {
  group: string;
  accounts: AccountOverviewRow[];
};

export type AccountsOverview = {
  groups: AccountOverviewGroup[];
};

export type AccountSummary = {
  account: string;
  transactions: number;
  accountTransactions: JournalTransaction[];
};

export type InvestmentOverview = {
  commodity: string;
  account: string;
  quantity: number;
  quantityFormatted: string;
  price: number | null;
  priceFormatted: string | null;
  currency: string | null;
  marketValueFormatted: string | null;
  tint: AmountTint;
};

export type MonthSetter = (updater: (month: Dayjs) => Dayjs) => void;

export type NavigationItem = {
  key: AppView;
  label: string;
  icon: React.ReactNode;
  disabled?: boolean;
  shortcut?: string;
  badge?: string;
};
