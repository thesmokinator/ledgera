import { Layout } from "antd";
import type { FormInstance } from "antd";
import type { ReactNode } from "react";
import { CourtesyState } from "./CourtesyState";
import {
  AccountsRoute,
  BalancesRoute,
  LogsRoute,
  SettingsRoute,
  TransactionsRoute,
} from "../routes";
import type {
  AppSettings,
  AppView,
  HledgerStatus,
  JournalSummary,
  JournalTransaction,
  UpdateStatus,
} from "../types";

type AppContentProps = {
  activeView: AppView;
  settingsForm: FormInstance<AppSettings>;
  activeSettings: AppSettings;
  commodityOptions: { value: string }[];
  hledgerStatus: HledgerStatus | undefined;
  journalSummary: JournalSummary | undefined;
  journalError: string | null;
  updateStatus: UpdateStatus | undefined;
  isCheckingForUpdates: boolean;
  shouldShowCourtesy: boolean;
  courtesyReasons: string[];
  courtesyDetails: string | undefined;
  onCheckForUpdates: () => void;
  onSettingsValuesChange: (changed: Partial<AppSettings>, values: AppSettings) => void;
  onEditTransaction: (transaction: JournalTransaction) => void;
  onDeleteTransaction: (id: string) => void;
};

export function AppContent({
  activeView,
  settingsForm,
  activeSettings,
  commodityOptions,
  hledgerStatus,
  journalSummary,
  journalError,
  updateStatus,
  isCheckingForUpdates,
  shouldShowCourtesy,
  courtesyReasons,
  courtesyDetails,
  onCheckForUpdates,
  onSettingsValuesChange,
  onEditTransaction,
  onDeleteTransaction,
}: AppContentProps) {
  function renderJournalRoute(): ReactNode {
    const routes: Record<"transactions" | "accounts" | "balances", ReactNode> = {
      accounts: (
        <AccountsRoute
          powerUser={activeSettings.powerUser}
          onEditTransaction={onEditTransaction}
          onDeleteTransaction={onDeleteTransaction}
        />
      ),
      balances: <BalancesRoute fetchPrices={activeSettings.fetchPrices} />,
      transactions: (
        <TransactionsRoute
          powerUser={activeSettings.powerUser}
          onEditTransaction={onEditTransaction}
          onDeleteTransaction={onDeleteTransaction}
        />
      ),
    };

    if (activeView === "accounts" || activeView === "balances" || activeView === "transactions") {
      return routes[activeView];
    }

    return routes.transactions;
  }

  function renderActiveRoute(): ReactNode {
    if (activeView === "settings") {
      return (
        <SettingsRoute
          form={settingsForm}
          initialValues={activeSettings}
          commodityOptions={commodityOptions}
          hledgerStatus={hledgerStatus}
          journalSummary={journalSummary}
          journalError={journalError}
          updateStatus={updateStatus}
          isCheckingForUpdates={isCheckingForUpdates}
          onCheckForUpdates={onCheckForUpdates}
          onValuesChange={onSettingsValuesChange}
        />
      );
    }

    if (activeView === "logs") {
      return <LogsRoute />;
    }

    if (shouldShowCourtesy) {
      return <CourtesyState reasons={courtesyReasons} details={courtesyDetails} />;
    }

    return renderJournalRoute();
  }

  return <Layout.Content className="app-content">{renderActiveRoute()}</Layout.Content>;
}
