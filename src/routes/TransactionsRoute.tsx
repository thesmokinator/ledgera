import { LeftOutlined, RightOutlined } from "@ant-design/icons";
import { Button, Card, Tabs, Typography } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import dayjs from "dayjs";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { TransactionsTable } from "./TransactionsTable";
import type { JournalSummary, JournalTransaction } from "./types";
import { formatCount } from "../utils/format";
import { collectAccounts } from "../utils/account";
import {
  isExecutedTransaction,
  isSameJournalMonth,
} from "../utils/date";
import { currentMonthShortcut } from "../utils/shortcut";
import styles from "./TransactionsRoute.module.css";

export function TransactionsRoute({
  powerUser,
  onEditTransaction,
  onDeleteTransaction,
}: {
  powerUser: boolean;
  onEditTransaction: (transaction: JournalTransaction) => void;
  onDeleteTransaction: (id: string) => void;
}) {
  const { t } = useTranslation();
  const [activeMonth, setActiveMonth] = useState(() => dayjs().startOf("month"));

  const goToCurrentMonth = useCallback(() => {
    setActiveMonth(dayjs().startOf("month"));
  }, []);

  const goToPreviousMonth = useCallback(() => {
    setActiveMonth((prev) => prev.subtract(1, "month"));
  }, []);

  const goToNextMonth = useCallback(() => {
    setActiveMonth((prev) => prev.add(1, "month"));
  }, []);

  // Keyboard navigation: arrows and Cmd/Ctrl+T
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Ignore when focus is inside an input, textarea, select, or contenteditable
      const target = e.target as HTMLElement;
      const tagName = target.tagName;
      if (
        tagName === "INPUT" ||
        tagName === "TEXTAREA" ||
        tagName === "SELECT" ||
        target.isContentEditable
      ) {
        return;
      }

      // Cmd+T / Ctrl+T -> current month
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "t") {
        e.preventDefault();
        goToCurrentMonth();
        return;
      }

      // ArrowLeft -> previous month
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        goToPreviousMonth();
        return;
      }

      // ArrowRight -> next month
      if (e.key === "ArrowRight") {
        e.preventDefault();
        goToNextMonth();
        return;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [goToCurrentMonth, goToPreviousMonth, goToNextMonth]);

  const transactionsQuery = useQuery({
    queryKey: ["transactions"],
    queryFn: () => invoke<JournalSummary>("list_transactions"),
    retry: false,
    refetchOnMount: true,
  });

  const transactions = transactionsQuery.data?.transactions ?? [];
  const visible = transactions.filter((tx) => isSameJournalMonth(tx.date, activeMonth));
  const monthlyTransactions = visible.filter(isExecutedTransaction);
  const scheduledTransactions = visible.filter((tx) => !isExecutedTransaction(tx));
  const accountsCount = collectAccounts(transactions, visible).length;
  const activeMonthLabel = activeMonth.format("MMMM YYYY");

  return (
    <div className={`${styles.content_stack} content-stack`}>
      <div className="metric-grid">
        <Card className="metric-card">
          <span>{t("dashboard.monthly_transactions")}</span>
          <strong>{formatCount(monthlyTransactions.length)}</strong>
          <p>{t("dashboard.monthly_transactions_description", { month: activeMonthLabel })}</p>
        </Card>
        <Card className="metric-card">
          <span>{t("dashboard.scheduled_transactions")}</span>
          <strong>{formatCount(scheduledTransactions.length)}</strong>
          <p>{t("dashboard.scheduled_transactions_description", { month: activeMonthLabel })}</p>
        </Card>
        <Card className="metric-card">
          <span>{t("dashboard.active_accounts")}</span>
          <strong>{formatCount(accountsCount)}</strong>
          <p>{t("dashboard.active_accounts_description")}</p>
        </Card>
      </div>

      <div className={styles.month_toolbar}>
        <div className={styles.month_nav_group}>
          <Button
            className={styles.month_arrow}
            icon={<LeftOutlined />}
            aria-label={t("common.previous")}
            onClick={goToPreviousMonth}
          />
          <Typography.Title level={4} className={styles.month_title}>
            {activeMonthLabel}
          </Typography.Title>
          <Button
            className={styles.month_arrow}
            icon={<RightOutlined />}
            aria-label={t("common.next")}
            onClick={goToNextMonth}
          />
        </div>
        <Button
          className={styles.current_month_button}
          onClick={goToCurrentMonth}
        >
          {t("common.current_month")}&nbsp;<span className={styles.shortcut_badge}>{currentMonthShortcut()}</span>
        </Button>
      </div>

      <Tabs
        className={styles.document_tabs}
        items={[
          {
            key: "executed",
            label: t("dashboard.monthly_transactions_tab"),
            children: (
              <TransactionsTable
                transactions={monthlyTransactions}
                loading={transactionsQuery.isFetching}
                powerUser={powerUser}
                onEdit={onEditTransaction}
                onDelete={onDeleteTransaction}
              />
            ),
          },
          {
            key: "scheduled",
            label: t("dashboard.scheduled_transactions_tab"),
            children: (
              <TransactionsTable
                transactions={scheduledTransactions}
                loading={transactionsQuery.isFetching}
                powerUser={powerUser}
                onEdit={onEditTransaction}
                onDelete={onDeleteTransaction}
              />
            ),
          },
        ]}
      />
    </div>
  );
}
