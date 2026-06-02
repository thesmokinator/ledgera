import {
  ConfigProvider,
  Form,
  Layout,
  message,
  theme,
} from "antd";
import {
  BankOutlined,
  BarChartOutlined,
  FileTextOutlined,
  HomeOutlined,
  PieChartOutlined,
  ScheduleOutlined,
  SettingOutlined,
  SyncOutlined,
} from "@ant-design/icons";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import dayjs from "dayjs";
import "dayjs/locale/it";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AppContent,
  AppHeader,
  AppLoader,
  CommandPalette,
  NavigationGroup,
  TransactionModal,
} from "./components";
import { useSystemTheme } from "./hooks/useSystemTheme";
import { useHotkeys } from "./hooks/useHotkeys";

import type {
  AppSettings,
  AppView,
  HledgerStatus,
  NavigationItem,
} from "./types";

import { navShortcut } from "./utils/shortcut";
import { callCommand } from "./utils/command";
import { resolveLanguagePreference } from "./utils/language";
import { useUpdateStatus } from "./hooks/useUpdateStatus";
import { useJournalData } from "./hooks/useJournalData";
import { useAppSettings } from "./hooks/useAppSettings";
import { useTransactionModal } from "./hooks/useTransactionModal";
import { useTransactionActions } from "./hooks/useTransactionActions";
import { gitSyncSummary, useGitSync } from "./hooks/useGitSync";
import "./App.css";

/** Renders the Ledgera desktop application. */
function App() {
  const [activeView, setActiveView] = useState<AppView>("transactions");
  const [spotlightOpen, setSpotlightOpen] = useState(false);
  const [settingsForm] = Form.useForm<AppSettings>();
  const [messageApi, contextHolder] = message.useMessage();
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const systemPrefersDark = useSystemTheme();
  const isMacOs = navigator.userAgent.includes("Mac");

  const {
    settingsQuery,
    activeSettings,
    updateSettingsOnChange,
  } = useAppSettings({ messageApi, t });
  const isDarkTheme = activeSettings.theme === "dark" || (activeSettings.theme === "system" && systemPrefersDark);
  const activeTitle = activeView === "settings" ? t("common.settings") : t(`common.${activeView}`);

  const hledgerQuery = useQuery({
    queryKey: ["hledger-status"],
    queryFn: () => callCommand<HledgerStatus>("check_hledger"),
  });

  const {
    updateStatus,
    isCheckingForUpdates,
    checkForUpdates,
  } = useUpdateStatus({
    enabled: activeSettings.modules.updateChecker.enabled,
    messageApi,
    t,
  });

  const {
    transactionsQuery,
    autocompleteSuggestions,
    codeOptions,
    descriptionOptions,
    accountOptions,
    commodityOptions,
    commentOptions,
    defaultCommodity,
    hasConfiguredJournal,
    journalLoadError,
  } = useJournalData(activeSettings);

  const {
    enabled: gitSyncEnabled,
    gitSyncStatus,
    isCheckingGitSync,
    isPulling: isPullingGitSync,
    isCommittingAndPushing: isCommittingAndPushingGitSync,
    refreshGitSyncStatus,
    pullJournal,
    commitAndPushJournal,
  } = useGitSync({ activeSettings, messageApi, t });

  const {
    transactionForm,
    transactionType,
    editingTransaction,
    isTransactionModalOpen,
    applyTransactionType,
    openCreateTransaction,
    openEditTransaction,
    closeTransactionModal,
    clearEditingTransaction,
  } = useTransactionModal({
    activeSettings,
    autocompleteSuggestions,
    defaultCommodity,
  });

  const {
    clearTransactionError,
    deleteTransaction,
    isSavingTransaction,
    submitTransaction,
    transactionError,
  } = useTransactionActions({
    defaultCommodity,
    editingTransaction,
    transactionForm,
    transactionType,
    messageApi,
    t,
    onSaved: () => {
      closeTransactionModal();
      clearEditingTransaction();
      if (gitSyncEnabled) refreshGitSyncStatus();
    },
  });


  const closeTransactionModalWithCleanup = useCallback(() => {
    clearTransactionError();
    closeTransactionModal();
  }, [clearTransactionError, closeTransactionModal]);

  const hledgerUnavailable = hledgerQuery.isFetched && hledgerQuery.data?.available === false;
  const courtesyReasons: string[] = [];
  if (!hasConfiguredJournal) courtesyReasons.push(t("settings.no_journal_configured"));
  if (journalLoadError) courtesyReasons.push(t("settings.journal_read_failed"));
  if (hledgerUnavailable) courtesyReasons.push(t("settings.hledger_not_found"));
  const shouldShowCourtesy = courtesyReasons.length > 0;

  useEffect(() => {
    if (settingsQuery.data) {
      settingsForm.setFieldsValue(activeSettings);
    }
  }, [settingsForm, settingsQuery.data]);

  useEffect(() => {
    const language = resolveLanguagePreference(activeSettings.language);
    if (i18n.resolvedLanguage !== language) {
      i18n.changeLanguage(language);
    }
    dayjs.locale(language);
  }, [activeSettings.language, i18n]);

  const shortcuts = useMemo(() => {
    const hasJournal = Boolean(activeSettings.journalPath.trim());
    const recurringIndex = 5;
    const syncIndex = 6;
    const logsIndex = activeSettings.modules.gitSync.enabled ? 7 : 6;
    const settingsIndex = 6 + Number(activeSettings.modules.gitSync.enabled) + Number(activeSettings.powerUser);

    return [
      { keys: "command+n, ctrl+n", action: () => { if (!shouldShowCourtesy) openCreateTransaction(); } },
      { keys: "command+1, ctrl+1", action: () => setActiveView("transactions"), disabled: !hasJournal },
      { keys: "command+2, ctrl+2", action: () => setActiveView("accounts"), disabled: !hasJournal },
      { keys: "command+3, ctrl+3", action: () => setActiveView("balances"), disabled: !hasJournal },
      { keys: "command+4, ctrl+4", action: () => setActiveView("reports"), disabled: !hasJournal },
      { keys: `command+${recurringIndex}, ctrl+${recurringIndex}`, action: () => setActiveView("recurring"), disabled: !hasJournal },
      { keys: `command+${syncIndex}, ctrl+${syncIndex}`, action: () => setActiveView("sync"), disabled: !hasJournal || !activeSettings.modules.gitSync.enabled },
      { keys: `command+${logsIndex}, ctrl+${logsIndex}`, action: () => setActiveView("logs"), disabled: !activeSettings.powerUser },
      { keys: "command+shift+g, ctrl+shift+g", action: () => { if (activeView === "sync") refreshGitSyncStatus(); else setActiveView("sync"); }, disabled: !gitSyncEnabled },
      { keys: `command+${settingsIndex}, ctrl+${settingsIndex}`, action: () => setActiveView("settings") },
      { keys: "command+k, ctrl+k", action: () => setSpotlightOpen(true), disabled: shouldShowCourtesy },
      { keys: "escape", action: () => { if (spotlightOpen) setSpotlightOpen(false); else if (isTransactionModalOpen) closeTransactionModalWithCleanup(); } },
    ];
  }, [activeSettings.journalPath, activeSettings.modules.gitSync.enabled, activeSettings.powerUser, shouldShowCourtesy, isTransactionModalOpen, spotlightOpen, closeTransactionModalWithCleanup, activeView, gitSyncEnabled, refreshGitSyncStatus]);

  useHotkeys(shortcuts);

  const autoGeneratedRef = useRef(false);
  useEffect(() => {
    if (
      autoGeneratedRef.current ||
      !activeSettings.journalPath.trim() ||
      !activeSettings.modules.autoGenerateRecurring
    ) {
      return;
    }
    autoGeneratedRef.current = true;
    callCommand("generate_recurring_transactions", { ruleIdFilter: null })
      .then(() => {
        queryClient.invalidateQueries({ queryKey: ["transactions"] });
        queryClient.invalidateQueries({ queryKey: ["periodic-rules"] });
      })
      .catch(() => {});
  }, [activeSettings.journalPath, activeSettings.modules.autoGenerateRecurring, queryClient]);

  const navigationItems = useMemo<NavigationItem[]>(
    () => {
      const hasJournal = Boolean(activeSettings.journalPath.trim());
      const items: NavigationItem[] = [
        { key: "transactions", label: "common.transactions", icon: <HomeOutlined />, disabled: !hasJournal, shortcut: navShortcut(1) },
        { key: "accounts", label: "common.accounts", icon: <BankOutlined />, disabled: !hasJournal, shortcut: navShortcut(2) },
        { key: "balances", label: "common.balances", icon: <PieChartOutlined />, disabled: !hasJournal, shortcut: navShortcut(3) },
        { key: "reports", label: "common.reports", icon: <BarChartOutlined />, disabled: !hasJournal, shortcut: navShortcut(4) },
        { key: "recurring", label: "common.recurring", icon: <ScheduleOutlined />, disabled: !hasJournal, shortcut: navShortcut(5) },
      ];

      if (activeSettings.modules.gitSync.enabled) {
        const summary = gitSyncSummary(gitSyncStatus);
        const syncBadge = gitSyncStatus && summary.tone !== "success" && summary.tone !== "neutral"
          ? summary.tone === "danger"
            ? t("sync.nav_badge_issue")
            : t(summary.labelKey, summary.labelOptions)
          : undefined;

        items.push({
          key: "sync",
          label: "common.sync",
          icon: <SyncOutlined />,
          disabled: !hasJournal,
          shortcut: navShortcut(6),
          badge: syncBadge,
          badgeTone: summary.tone === "danger" ? "danger" : "warning",
        });
      }

      if (activeSettings.powerUser) {
        const logsIndex = activeSettings.modules.gitSync.enabled ? 7 : 6;
        items.push({ key: "logs", label: "logs.title", icon: <FileTextOutlined />, shortcut: navShortcut(logsIndex) });
      }

      const settingsIndex = items.length + 1;
      items.push({
        key: "settings",
        label: "common.settings",
        icon: <SettingOutlined />,
        shortcut: navShortcut(settingsIndex),
        badge: updateStatus?.available ? t("settings.update_badge") : undefined,
      });

      return items;
    },
    [activeSettings.powerUser, activeSettings.journalPath, activeSettings.modules.gitSync.enabled, updateStatus?.available, gitSyncStatus, t],
  );

  // Show a loading spinner while the initial settings are being fetched
  if (settingsQuery.isPending) {
    return <AppLoader systemPrefersDark={systemPrefersDark} />;
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
      <Layout className={`app-shell ${isDarkTheme ? "theme-dark" : "theme-light"}${isMacOs ? " platform-macos" : ""}`}>
        {contextHolder}
        <Layout.Sider className="app-sidebar" width={288}>
          <NavigationGroup
            items={navigationItems}
            activeKey={activeView}
            onSelect={(key) => setActiveView(key)}
          />
        </Layout.Sider>

        <Layout className="app-main">
          <AppHeader
            title={activeTitle}
            disableActions={shouldShowCourtesy}
            onCreateTransaction={openCreateTransaction}
            onOpenSearch={() => setSpotlightOpen(true)}
          />

          <AppContent
            activeView={activeView}
            settingsForm={settingsForm}
            activeSettings={activeSettings}
            codeOptions={codeOptions}
            descriptionOptions={descriptionOptions}
            commodityOptions={commodityOptions}
            accountOptions={accountOptions}
            commentOptions={commentOptions}
            defaultCommodity={defaultCommodity}
            hledgerStatus={hledgerQuery.data}
            journalSummary={transactionsQuery.data}
            journalError={transactionsQuery.isError ? String(transactionsQuery.error) : null}
            updateStatus={updateStatus}
            isCheckingForUpdates={isCheckingForUpdates}
            updateCheckerEnabled={activeSettings.modules.updateChecker.enabled}
            gitSyncStatus={gitSyncStatus}
            isCheckingGitSync={isCheckingGitSync}
            isPullingGitSync={isPullingGitSync}
            isCommittingAndPushingGitSync={isCommittingAndPushingGitSync}
            shouldShowCourtesy={shouldShowCourtesy}
            courtesyReasons={courtesyReasons}
            courtesyDetails={journalLoadError || hledgerQuery.data?.message}
            onCheckForUpdates={checkForUpdates}
            onRefreshGitSyncStatus={() => { refreshGitSyncStatus(); }}
            onPullGitSync={pullJournal}
            onCommitAndPushGitSync={commitAndPushJournal}
            onSettingsValuesChange={updateSettingsOnChange}
            onEditTransaction={openEditTransaction}
            onDeleteTransaction={deleteTransaction}
          />
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
          commentOptions={commentOptions}
          defaultCommodity={defaultCommodity}
          saveError={transactionError}
          onClose={closeTransactionModalWithCleanup}
          onFormChange={clearTransactionError}
          onSubmit={submitTransaction}
          onTransactionTypeChange={applyTransactionType}
        />
      </Layout>
    </ConfigProvider>
  );
}

export default App;
