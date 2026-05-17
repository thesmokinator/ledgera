import {
  Button,
  ConfigProvider,
  Form,
  Layout,
  Space,
  Typography,
  message,
  theme,
} from "antd";
import {
  BankOutlined,
  FileTextOutlined,
  HomeOutlined,
  PlusOutlined,
  ReloadOutlined,
  SettingOutlined,
} from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import dayjs from "dayjs";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CourtesyState,
  NavigationGroup,
  TransactionModal,
} from "./components";
import { useSystemTheme } from "./hooks/useSystemTheme";
import {
  AccountsRoute,
  LogsRoute,
  SettingsRoute,
  TransactionsRoute,
} from "./routes";
import type {
  AccountActivityRange,
  AppSettings,
  AutocompleteSuggestions,
  HledgerStatus,
  JournalSummary,
  JournalTransaction,
  NavigationItem,
  PostingInput,
  TransactionInput,
  TransactionType,
} from "./types";
import {
  isExecutedTransaction,
  isInAccountActivityRange,
  isSameJournalMonth,
  journalDateFormat,
  todayJournalDate,
} from "./utils/date";
import { toAutocompleteOptions } from "./utils/format";
import {
  collectAccounts,
  transactionTemplatePostings,
} from "./utils/account";
import { normalizeSettings } from "./utils/settings";
import {
  emptyTransaction,
  toTransactionInput,
} from "./utils/transaction";
import { parseError } from "./utils/error";
import "./App.css";

const accountActivityRangeOptions: AccountActivityRange[] = [
  "current-month",
  "30",
  "60",
  "90",
  "180",
  "365",
];

/** Invokes a typed Tauri command. */
function callCommand<TResponse, TPayload extends Record<string, unknown> = Record<string, never>>(
  command: string,
  payload?: TPayload,
): Promise<TResponse> {
  return invoke<TResponse>(command, payload);
}

/** Renders the Ledgera desktop application. */
function App() {
  const [activeView, setActiveView] = useState("transactions");
  const [activeMonth, setActiveMonth] = useState(() => dayjs().startOf("month"));
  const [accountActivityRange, setAccountActivityRange] = useState<AccountActivityRange>("current-month");
  const [transactionType, setTransactionType] = useState<TransactionType>("expense");
  const [editingTransaction, setEditingTransaction] = useState<JournalTransaction | null>(null);
  const [isTransactionModalOpen, setTransactionModalOpen] = useState(false);
  const [transactionForm] = Form.useForm<TransactionInput>();
  const [settingsForm] = Form.useForm<AppSettings>();
  const queryClient = useQueryClient();
  const [messageApi, contextHolder] = message.useMessage();
  const { t } = useTranslation();
  const systemPrefersDark = useSystemTheme();
  const isMacOs = navigator.userAgent.includes("Mac");

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: async () => normalizeSettings(await callCommand<AppSettings>("get_app_settings")),
  });

  const activeSettings = normalizeSettings(settingsQuery.data);
  const isDarkTheme = activeSettings.theme === "system" ? systemPrefersDark : activeSettings.theme === "dark";
  const activeTitle = activeView === "settings" ? t("common.settings") : t(`common.${activeView}`);

  const hledgerQuery = useQuery({
    queryKey: ["hledger-status"],
    queryFn: () => callCommand<HledgerStatus>("check_hledger"),
  });

  const transactionsQuery = useQuery({
    queryKey: ["transactions"],
    queryFn: () => callCommand<JournalSummary>("list_transactions"),
    enabled: Boolean(settingsQuery.data?.journalPath),
    retry: false,
  });

  const autocompleteQuery = useQuery({
    queryKey: ["autocomplete-suggestions"],
    queryFn: () => callCommand<AutocompleteSuggestions>("get_autocomplete_suggestions"),
    enabled: Boolean(settingsQuery.data?.journalPath),
    retry: false,
  });

  const updateSettingsMutation = useMutation({
    mutationFn: (settings: AppSettings) =>
      callCommand<AppSettings, { settings: AppSettings }>("update_app_settings", {
        settings: normalizeSettings(settings),
      }),
    onSuccess: async (_, variables) => {
      const next = normalizeSettings(variables);
      queryClient.setQueryData(["settings"], next);

      const prev = activeSettings;
      if (prev.journalPath !== next.journalPath) {
        queryClient.resetQueries({ queryKey: ["transactions"] });
        await queryClient.invalidateQueries({ queryKey: ["transactions"] });
        await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
      }
      if (prev.hledgerPath !== next.hledgerPath) {
        await queryClient.invalidateQueries({ queryKey: ["hledger-status"] });
      }
      if (prev.defaultCommodity !== next.defaultCommodity) {
        await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
      }
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  const createTransactionMutation = useMutation({
    mutationFn: (input: TransactionInput) =>
      callCommand<JournalSummary, { input: TransactionInput }>("create_transaction", { input }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transactionCreated"));
      setTransactionModalOpen(false);
      await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  const updateTransactionMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: TransactionInput }) =>
      callCommand<JournalSummary, { id: string; input: TransactionInput }>("update_transaction", {
        id,
        input,
      }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transactionUpdated"));
      setTransactionModalOpen(false);
      setEditingTransaction(null);
      await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  const deleteTransactionMutation = useMutation({
    mutationFn: (id: string) =>
      callCommand<JournalSummary, { id: string }>("delete_transaction", { id }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transactionDeleted"));
      await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  const autocompleteSuggestions = autocompleteQuery.data ?? {
    codes: [],
    descriptions: [],
    accounts: [],
    commodities: [],
    defaultCommodity: "",
    defaultCashAccount: "",
    defaultExpenseAccount: "",
    defaultIncomeAccount: "",
    defaultTransferAccount: "",
    defaultInvestmentAccount: "",
    defaultInvestmentCommodity: "",
  };
  const codeOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.codes),
    [autocompleteSuggestions.codes],
  );
  const descriptionOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.descriptions),
    [autocompleteSuggestions.descriptions],
  );
  const accountOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.accounts),
    [autocompleteSuggestions.accounts],
  );
  const commodityOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.commodities),
    [autocompleteSuggestions.commodities],
  );
  const defaultCommodity = autocompleteSuggestions.defaultCommodity || "";
  const hasConfiguredJournal = activeSettings.journalPath.trim().length > 0;
  const hledgerUnavailable = hledgerQuery.isFetched && hledgerQuery.data?.available === false;
  const journalLoadError = transactionsQuery.isError ? String(transactionsQuery.error) : "";
  const courtesyReasons = [
    !hasConfiguredJournal ? t("settings.noJournalConfigured") : null,
    journalLoadError ? t("settings.journalReadFailed") : null,
    hledgerUnavailable ? t("settings.hledgerNotFound") : null,
  ].filter((reason): reason is string => Boolean(reason));
  const shouldShowCourtesy = courtesyReasons.length > 0;


  const transactions = transactionsQuery.data?.transactions ?? [];
  const visibleMonthTransactions = transactions.filter((transaction) =>
    isSameJournalMonth(transaction.date, activeMonth),
  );
  const monthlyTransactions = visibleMonthTransactions.filter(isExecutedTransaction);
  const scheduledTransactions = visibleMonthTransactions.filter(
    (transaction) => !isExecutedTransaction(transaction),
  );
  const visibleAccountTransactions = transactions.filter((transaction) =>
    isInAccountActivityRange(transaction, accountActivityRange),
  );
  const accounts = collectAccounts(transactions, visibleAccountTransactions);
  const accountsCount = accounts.length;
  const activeMonthLabel = activeMonth.format("MMMM YYYY");

  useEffect(() => {
    if (settingsQuery.data) {
      settingsForm.setFieldsValue(normalizeSettings(settingsQuery.data));
    }
  }, [settingsForm, settingsQuery.data]);

  function applyTransactionType(type: TransactionType) {
    setTransactionType(type);
    transactionForm.setFieldValue(
      "postings",
      transactionTemplatePostings(type, autocompleteSuggestions, defaultCommodity),
    );
  }

  function openCreateTransaction() {
    setEditingTransaction(null);
    setTransactionType("expense");
    transactionForm.setFieldsValue({
      ...emptyTransaction,
      date: todayJournalDate(),
      postings: transactionTemplatePostings("expense", autocompleteSuggestions, defaultCommodity),
    });
    setTransactionModalOpen(true);
  }

  function openEditTransaction(transaction: JournalTransaction) {
    setEditingTransaction(transaction);
    setTransactionType("custom");
    transactionForm.setFieldsValue(toTransactionInput(transaction));
    setTransactionModalOpen(true);
  }

  function parseAmountValue(amount: string): number {
    const trimmed = amount.trim().replace(",", ".");
    const match = trimmed.match(/(-?[\d.]+)/);
    return match ? parseFloat(match[1]) : 0;
  }

  function parseUnitPrice(unitPrice: string): { value: number; commodity: string } {
    const trimmed = unitPrice.trim();
    const value = parseAmountValue(trimmed);
    const commodity = trimmed.replace(/[\d.,\s-]/g, "").trim();
    return { value, commodity };
  }

  function autoCalculateBalancingAmounts(postings: PostingInput[]): PostingInput[] {
    const result = postings.map((p) => ({ ...p }));
    const pricedIndex = result.findIndex(
      (p) => p.unitPrice.trim() && p.amount.trim()
    );
    if (pricedIndex === -1) return result;

    const priced = result[pricedIndex];
    const quantity = parseAmountValue(priced.amount);
    const { value: unitPriceValue, commodity: priceCommodity } = parseUnitPrice(
      priced.unitPrice
    );
    if (quantity === 0 || unitPriceValue === 0) return result;

    const total = quantity * unitPriceValue;

    // Find a balancing posting: one with no unitPrice, preferring the same commodity as the price
    const balanceIndex = result.findIndex(
      (p, i) =>
        i !== pricedIndex &&
        !p.unitPrice.trim() &&
        (!p.amount.trim() || p.commodity === priceCommodity || p.commodity === defaultCommodity)
    );
    if (balanceIndex === -1) return result;

    result[balanceIndex] = {
      ...result[balanceIndex],
      amount: (-total).toFixed(2),
      commodity: priceCommodity || result[balanceIndex].commodity || defaultCommodity,
    };
    return result;
  }

  function submitTransaction(values: TransactionInput) {
    const rawPostings = (values.postings ?? [])
      .filter((posting) => posting.account.trim().length > 0)
      .map((posting) => ({
        account: posting.account,
        amount: posting.amount ?? "",
        commodity: posting.amount?.trim() ? (posting.commodity ?? defaultCommodity) : (posting.commodity ?? ""),
        unitPrice: posting.unitPrice ?? "",
        comment: posting.comment ?? "",
      }));

    const balancedPostings = autoCalculateBalancingAmounts(rawPostings);

    const normalizedValues: TransactionInput = {
      date: dayjs.isDayjs(values.date) ? values.date.format(journalDateFormat) : values.date,
      status: values.status ?? "",
      code: values.code ?? "",
      description: values.description,
      postings: balancedPostings,
    };

    if (editingTransaction) {
      updateTransactionMutation.mutate({ id: editingTransaction.id, input: normalizedValues });
      return;
    }

    createTransactionMutation.mutate(normalizedValues);
  }

  function updateSettingsOnChange(_: Partial<AppSettings>, values: AppSettings) {
    updateSettingsMutation.mutate(normalizeSettings(values));
  }

  const isSavingTransaction =
    createTransactionMutation.isPending || updateTransactionMutation.isPending;

  const navigationItems = useMemo<NavigationItem[]>(
    () => {
      const hasJournal = Boolean(activeSettings.journalPath.trim());
      return [
        { key: "transactions", label: "common.transactions", icon: <HomeOutlined />, disabled: !hasJournal },
        { key: "accounts", label: "common.accounts", icon: <BankOutlined />, disabled: !hasJournal },
        { key: "settings", label: "common.settings", icon: <SettingOutlined /> },
        ...(activeSettings.powerUser
          ? [{ key: "logs", label: "logs.title", icon: <FileTextOutlined /> }]
          : []),
      ];
    },
    [activeSettings.powerUser, activeSettings.journalPath],
  );

  return (
    <ConfigProvider
      theme={{
        algorithm: isDarkTheme ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: {
          borderRadius: 10,
          colorPrimary: "#10b981",
        },
      }}
    >
      <Layout className={`app-shell ${isDarkTheme ? "theme-dark" : "theme-light"} ${isMacOs ? "platform-macos" : ""}`}>
        {contextHolder}
        <Layout.Sider className="app-sidebar" width={288}>
          <NavigationGroup
            items={navigationItems}
            activeKey={activeView}
            onSelect={(key) => setActiveView(key)}
          />
        </Layout.Sider>

        <Layout className="app-main">
          <Layout.Header className="app-header">
            <div className="titlebar-drag" data-tauri-drag-region>
              <Typography.Title level={3}>{activeTitle}</Typography.Title>
            </div>
            <Space>
              <Button icon={<ReloadOutlined />} onClick={() => queryClient.invalidateQueries()}>
                {t("common.refresh")}
              </Button>
              <Button type="primary" icon={<PlusOutlined />} disabled={shouldShowCourtesy} onClick={openCreateTransaction}>
                {t("transactions.newTransaction")}
              </Button>
            </Space>
          </Layout.Header>

          <Layout.Content className="app-content">
            {activeView === "settings" ? (
              <SettingsRoute
                form={settingsForm}
                initialValues={activeSettings}
                commodityOptions={commodityOptions}
                hledgerStatus={hledgerQuery.data}
                journalSummary={transactionsQuery.data}
                journalError={transactionsQuery.isError ? String(transactionsQuery.error) : null}
                onValuesChange={updateSettingsOnChange}
              />
            ) : activeView === "logs" ? (
              <LogsRoute />
            ) : shouldShowCourtesy ? (
              <CourtesyState
                reasons={courtesyReasons}
                details={journalLoadError || hledgerQuery.data?.message}
              />
            ) : activeView === "accounts" ? (
              <AccountsRoute
                accounts={accounts}
                accountActivityRange={accountActivityRange}
                accountActivityRangeOptions={accountActivityRangeOptions}
                loading={transactionsQuery.isFetching}
                powerUser={activeSettings.powerUser}
                onActivityRangeChange={setAccountActivityRange}
                onEditTransaction={openEditTransaction}
                onDeleteTransaction={(id) => deleteTransactionMutation.mutate(id)}
              />
            ) : (
              <TransactionsRoute
                monthlyTransactions={monthlyTransactions}
                scheduledTransactions={scheduledTransactions}
                accountsCount={accountsCount}
                activeMonth={activeMonth}
                activeMonthLabel={activeMonthLabel}
                loading={transactionsQuery.isFetching}
                powerUser={activeSettings.powerUser}
                onMonthChange={setActiveMonth}
                onEditTransaction={openEditTransaction}
                onDeleteTransaction={(id) => deleteTransactionMutation.mutate(id)}
              />
            )}
          </Layout.Content>
        </Layout>

        <TransactionModal
          open={isTransactionModalOpen}
          editingTransaction={editingTransaction}
          transactionForm={transactionForm}
          isSaving={isSavingTransaction}
          transactionType={transactionType}
          codeOptions={codeOptions}
          descriptionOptions={descriptionOptions}
          accountOptions={accountOptions}
          commodityOptions={commodityOptions}
          defaultCommodity={defaultCommodity}
          onClose={() => setTransactionModalOpen(false)}
          onSubmit={submitTransaction}
          onTransactionTypeChange={applyTransactionType}
        />
      </Layout>
    </ConfigProvider>
  );
}

export default App;
