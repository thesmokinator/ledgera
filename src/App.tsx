import {
  Button,
  ConfigProvider,
  Form,
  Layout,
  Typography,
  message,
  theme,
} from "antd";
import {
  BankOutlined,
  FileTextOutlined,
  HomeOutlined,
  PieChartOutlined,
  SearchOutlined,
  SettingOutlined,
} from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import dayjs from "dayjs";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CommandPalette,
  CourtesyState,
  NavigationGroup,
  TransactionModal,
} from "./components";
import { useSystemTheme } from "./hooks/useSystemTheme";
import { useHotkeys } from "./hooks/useHotkeys";
import {
  AccountsRoute,
  BalancesRoute,
  LogsRoute,
  SettingsRoute,
  TransactionsRoute,
} from "./routes";
import type {
  AppSettings,
  AutocompleteSuggestions,
  HledgerStatus,
  JournalSummary,
  JournalTransaction,
  NavigationItem,
  TransactionInput,
  TransactionType,
  UpdateStatus,
} from "./types";
import {
  journalDateFormat,
  todayJournalDate,
} from "./utils/date";
import { toAutocompleteOptions } from "./utils/format";
import { transactionTemplatePostings } from "./utils/account";
import { normalizeSettings } from "./utils/settings";
import {
  emptyTransaction,
  toTransactionInput,
  autoCalculateBalancingAmounts,
  withDefaultCommodity,
} from "./utils/transaction";
import { parseError } from "./utils/error";
import { navShortcut, newTransactionShortcut, spotlightShortcut } from "./utils/shortcut";
import "./App.css";

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
  const [transactionType, setTransactionType] = useState<TransactionType>("expense");
  const [editingTransaction, setEditingTransaction] = useState<JournalTransaction | null>(null);
  const [isTransactionModalOpen, setTransactionModalOpen] = useState(false);
  const [spotlightOpen, setSpotlightOpen] = useState(false);
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

  const updateStatusQuery = useQuery({
    queryKey: ["update-status"],
    queryFn: () => callCommand<UpdateStatus, { force: boolean }>("check_for_updates", { force: false }),
    retry: false,
  });

  const checkForUpdatesMutation = useMutation({
    mutationFn: () => callCommand<UpdateStatus, { force: boolean }>("check_for_updates", { force: true }),
    onSuccess: (status) => {
      queryClient.setQueryData(["update-status"], status);
      if (status.available) {
        messageApi.success(t("settings.update_available", { version: status.latestVersion }));
      } else if (status.error) {
        messageApi.error(t("settings.update_check_failed"));
      } else {
        messageApi.success(t("settings.up_to_date"));
      }
    },
    onError: () => messageApi.error(t("settings.update_check_failed")),
  });

  const updateStatus = checkForUpdatesMutation.data ?? updateStatusQuery.data;

  const transactionsQuery = useQuery({
    queryKey: ["transactions"],
    queryFn: () => callCommand<JournalSummary>("list_transactions"),
    enabled: Boolean(settingsQuery.data?.journalPath),
    retry: false,
    refetchOnMount: true,
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
      if (prev.excludeBalances !== next.excludeBalances) {
        await queryClient.invalidateQueries({ queryKey: ["balances"] });
      }
      if (prev.includeInvestments !== next.includeInvestments) {
        await queryClient.invalidateQueries({ queryKey: ["investments"] });
      }
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  const createTransactionMutation = useMutation({
    mutationFn: (input: TransactionInput) =>
      callCommand<JournalSummary, { input: TransactionInput }>("create_transaction", { input }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transaction_created"));
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
      messageApi.success(t("transactions.transaction_updated"));
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
      messageApi.success(t("transactions.transaction_deleted"));
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
    !hasConfiguredJournal ? t("settings.no_journal_configured") : null,
    journalLoadError ? t("settings.journal_read_failed") : null,
    hledgerUnavailable ? t("settings.hledger_not_found") : null,
  ].filter((reason): reason is string => Boolean(reason));
  const shouldShowCourtesy = courtesyReasons.length > 0;

  useEffect(() => {
    if (settingsQuery.data) {
      settingsForm.setFieldsValue(normalizeSettings(settingsQuery.data));
    }
  }, [settingsForm, settingsQuery.data]);

  function applyTransactionType(type: TransactionType) {
    setTransactionType(type);
    transactionForm.setFieldValue(
      "postings",
      activeSettings.prefillPostings
        ? transactionTemplatePostings(type, autocompleteSuggestions, defaultCommodity)
        : withDefaultCommodity(emptyTransaction.postings, defaultCommodity),
    );
  }

  function openCreateTransaction() {
    setEditingTransaction(null);
    setTransactionType("expense");
    transactionForm.setFieldsValue({
      ...emptyTransaction,
      date: todayJournalDate(),
      postings: activeSettings.prefillPostings
        ? transactionTemplatePostings("expense", autocompleteSuggestions, defaultCommodity)
        : withDefaultCommodity(emptyTransaction.postings, defaultCommodity),
    });
    setTransactionModalOpen(true);
  }

  function openEditTransaction(transaction: JournalTransaction) {
    setEditingTransaction(transaction);
    setTransactionType("custom");
    transactionForm.setFieldsValue(toTransactionInput(transaction, defaultCommodity));
    setTransactionModalOpen(true);
  }



  const shortcuts = useMemo(() => {
    const hasJournal = Boolean(activeSettings.journalPath.trim());
    return [
      { keys: "command+n, ctrl+n", action: () => { if (!shouldShowCourtesy) openCreateTransaction(); } },
      { keys: "command+1, ctrl+1", action: () => setActiveView("transactions"), disabled: !hasJournal },
      { keys: "command+2, ctrl+2", action: () => setActiveView("accounts"), disabled: !hasJournal },
      { keys: "command+3, ctrl+3", action: () => setActiveView("balances"), disabled: !hasJournal },
      { keys: "command+4, ctrl+4", action: () => setActiveView("settings") },
      { keys: "command+5, ctrl+5", action: () => setActiveView("logs"), disabled: !activeSettings.powerUser },
      { keys: "command+,, ctrl+,", action: () => setActiveView("settings") },
      { keys: "command+k, ctrl+k", action: () => setSpotlightOpen(true) },
      { keys: "escape", action: () => { if (spotlightOpen) setSpotlightOpen(false); else if (isTransactionModalOpen) setTransactionModalOpen(false); } },
    ];
  }, [activeSettings.journalPath, activeSettings.powerUser, shouldShowCourtesy, isTransactionModalOpen, spotlightOpen]);

  useHotkeys(shortcuts);

  function submitTransaction(values: TransactionInput) {
    const rawPostings = (values.postings ?? [])
      .filter((posting) => posting.account.trim().length > 0)
      .map((posting) => ({
        account: posting.account,
        amount: posting.amount ?? "",
        commodity: posting.amount?.trim()
          ? (posting.commodity?.trim() || defaultCommodity)
          : (posting.commodity?.trim() || ""),
        unitPrice: posting.unitPrice ?? "",
        comment: posting.comment ?? "",
      }));

    const balancedPostings = autoCalculateBalancingAmounts(rawPostings, defaultCommodity);

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
        { key: "transactions", label: "common.transactions", icon: <HomeOutlined />, disabled: !hasJournal, shortcut: navShortcut(1) },
        { key: "accounts", label: "common.accounts", icon: <BankOutlined />, disabled: !hasJournal, shortcut: navShortcut(2) },
        { key: "balances", label: "common.balances", icon: <PieChartOutlined />, disabled: !hasJournal, shortcut: navShortcut(3) },
        { key: "settings", label: "common.settings", icon: <SettingOutlined />, shortcut: navShortcut(4), badge: updateStatus?.available ? t("settings.update_badge") : undefined },
        ...(activeSettings.powerUser
          ? [{ key: "logs", label: "logs.title", icon: <FileTextOutlined />, shortcut: navShortcut(5) }]
          : []),
      ];
    },
    [activeSettings.powerUser, activeSettings.journalPath, updateStatus?.available, t],
  );

  // Show a loading spinner while the initial settings are being fetched
  if (settingsQuery.isPending) {
    return (
      <div className={`app-loader-react ${systemPrefersDark ? "theme-dark" : "theme-light"}`}>
        <div className="app-loader-spinner" />
        <span className="app-loader-label">ledgera</span>
      </div>
    );
  }

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
            <div className="header-actions">
              <button
                type="button"
                className="search_trigger"
                onClick={() => setSpotlightOpen(true)}
                title={t("common.search")}
                aria-label={t("common.search")}
              >
                <SearchOutlined className="search_trigger_icon" />
                <span className="search_trigger_label">Search journal…</span>
                <span className="search_trigger_shortcut">{spotlightShortcut()}</span>
              </button>
              <Button type="primary" disabled={shouldShowCourtesy} onClick={openCreateTransaction}>
                {t("transactions.new_transaction")} ({newTransactionShortcut()})
              </Button>
            </div>
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
                updateStatus={updateStatus}
                isCheckingForUpdates={updateStatusQuery.isFetching || checkForUpdatesMutation.isPending}
                onCheckForUpdates={() => checkForUpdatesMutation.mutate()}
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
                powerUser={activeSettings.powerUser}
                onEditTransaction={openEditTransaction}
                onDeleteTransaction={(id) => deleteTransactionMutation.mutate(id)}
              />
            ) : activeView === "balances" ? (
              <BalancesRoute fetchPrices={activeSettings.fetchPrices} />
            ) : activeView === "logs" ? (
              <LogsRoute />
            ) : shouldShowCourtesy ? (
              <CourtesyState
                reasons={courtesyReasons}
                details={journalLoadError || hledgerQuery.data?.message}
              />
            ) : (
              <TransactionsRoute
                powerUser={activeSettings.powerUser}
                onEditTransaction={openEditTransaction}
                onDeleteTransaction={(id) => deleteTransactionMutation.mutate(id)}
              />
            )}
          </Layout.Content>
        </Layout>

        <CommandPalette
          open={spotlightOpen}
          onClose={() => setSpotlightOpen(false)}
          onTransaction={(transaction) => {
            openEditTransaction(transaction);
          }}
        />

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
