import type { Dayjs } from "dayjs";

export type ThemePreference = "system" | "dark" | "light";
export type LanguagePreference = "system" | "en" | "it";

export type AppView = "transactions" | "accounts" | "balances" | "sync" | "settings" | "logs" | "reports" | "recurring" | "budget";

export type AppModuleSettings = {
  enabled: boolean;
};

export type GitSyncModuleSettings = AppModuleSettings & {
  commitMessage: string;
};

export type AppModulesSettings = {
  marketPrices: AppModuleSettings;
  developerTools: AppModuleSettings;
  updateChecker: AppModuleSettings;
  gitSync: GitSyncModuleSettings;
  autoGenerateRecurring: boolean;
};

export type CommoditySymbolMapping = {
  commodity: string;
  yahooSymbol: string;
};

export type AppSettings = {
  journalPath: string;
  hledgerPath: string;
  theme: ThemePreference;
  language: LanguagePreference;
  powerUser: boolean;
  defaultCommodity: string;
  fetchPrices: boolean;
  commoditySymbols: CommoditySymbolMapping[];
  excludeBalances: string[];
  includeInvestments: string[];
  prefillPostings: boolean;
  modules: AppModulesSettings;
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
  unitPrice: string;
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
  suggestions: AutocompleteSuggestions;
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
  comments: string[];
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
};

export type Balance = {
  account: string;
  amount: number;
  commodity: string;
  formatted: string;
  tint: AmountTint;
};

export type ReportResult = {
  reportType: string;
  interval: string;
  periodColumns: string[];
  rows: ReportRow[];
  visualization: ReportVisualization;
};

export type ReportVisualizationKind = "allocation" | "breakdown" | "cashflow" | string;

export type ReportVisualization = {
  kind: ReportVisualizationKind;
  entries: ReportChartEntry[];
  periods: ReportPeriodSummary[];
  accountLevel: number;
};

export type ReportChartEntry = {
  account: string;
  label: string;
  amount: number;
  chartAmount: number;
  chartAmountFormatted: string;
  commodity: string;
  formatted: string;
  tint: AmountTint;
};

export type ReportPeriodSummary = {
  period: string;
  amount: number;
  chartAmount: number;
  chartAmountFormatted: string;
  commodity: string;
  formatted: string;
  tint: AmountTint;
};

export type ReportPeriodAmount = {
  period: string;
  amount: number;
  commodity: string;
  formatted: string;
  tint: AmountTint;
};

export type ReportRow = {
  account: string;
  indent: number;
  isTotal: boolean;
  amounts: ReportPeriodAmount[];
  total: ReportPeriodAmount;
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
  error: string | null;
};

export type MonthSetter = (updater: (month: Dayjs) => Dayjs) => void;

export type GitSyncFileStatus = {
  path: string;
  status: string;
};

export type GitCommitInfo = {
  hash: string;
  fullHash: string;
  subject: string;
};

export type GitSyncStatus = {
  available: boolean;
  repoFound: boolean;
  repoRoot: string | null;
  branch: string | null;
  upstream: string | null;
  remote: string | null;
  ahead: number;
  behind: number;
  dirty: boolean;
  files: GitSyncFileStatus[];
  lastCommit: GitCommitInfo | null;
  error: string | null;
};

export type Notifier = {
  success: (content: string) => void;
  error: (content: string) => void;
};

export type NavigationItem = {
  key: AppView;
  label: string;
  icon: React.ReactNode;
  disabled?: boolean;
  shortcut?: string;
  badge?: string;
  badgeTone?: "warning" | "danger";
};

export type PeriodicRule = {
  id: string;
  ruleId: string;
  sourceFile: string;
  periodExpr: string;
  description: string;
  postings: JournalPosting[];
  status: string;
  code: string;
  startDate: string | null;
  endDate: string | null;
  comment: string;
  raw: string;
  startLine: number;
  endLine: number;
};

export type PeriodicRulesSummary = {
  rules: PeriodicRule[];
  ruleCount: number;
};

export type PeriodicRuleInput = {
  ruleId: string;
  periodExpr: string;
  description: string;
  postings: PostingInput[];
  status: string;
  code: string;
  startDate?: string;
  endDate?: string;
  comment?: string;
};

export type PendingRecurringDates = {
  ruleId: string;
  description: string;
  dates: string[];
};

export type GenerateResult = {
  generated: number;
  rules: string[];
};

export type BudgetPeriodAmount = {
  period: string;
  actual: number;
  budget: number;
  remaining: number;
  pctUsed: number;
  commodity: string;
  actualFormatted: string;
  budgetFormatted: string;
  remainingFormatted: string;
  tint: AmountTint;
};

export type BudgetRow = {
  account: string;
  periods: BudgetPeriodAmount[];
  totalActual: number;
  totalBudget: number;
  totalRemaining: number;
  totalPctUsed: number;
  commodity: string;
  totalActualFormatted: string;
  totalBudgetFormatted: string;
  totalRemainingFormatted: string;
  tint: AmountTint;
};

export type BudgetReport = {
  periodColumns: string[];
  rows: BudgetRow[];
};
