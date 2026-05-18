import { LeftOutlined, RightOutlined } from "@ant-design/icons";
import { Button, Card, Space, Tabs, Typography } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import dayjs from "dayjs";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { TransactionsTable } from "./TransactionsTable";
import type { JournalSummary, JournalTransaction } from "./types";
import { formatCount } from "../utils/format";
import { collectAccounts } from "../utils/account";
import {
  isExecutedTransaction,
  isSameJournalMonth,
} from "../utils/date";
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
    <Space direction="vertical" size={24} className="content-stack">
      <div className="metric-grid">
        <Card className="metric-card">
          <span>{t("dashboard.monthlyTransactions")}</span>
          <strong>{formatCount(monthlyTransactions.length)}</strong>
          <p>{t("dashboard.monthlyTransactionsDescription", { month: activeMonthLabel })}</p>
        </Card>
        <Card className="metric-card">
          <span>{t("dashboard.scheduledTransactions")}</span>
          <strong>{formatCount(scheduledTransactions.length)}</strong>
          <p>{t("dashboard.scheduledTransactionsDescription", { month: activeMonthLabel })}</p>
        </Card>
        <Card className="metric-card">
          <span>{t("dashboard.activeAccounts")}</span>
          <strong>{formatCount(accountsCount)}</strong>
          <p>{t("dashboard.activeAccountsDescription")}</p>
        </Card>
      </div>

      <Card className={`${styles.card} ${styles.monthToolbarCard}`}>
        <Space className={styles.monthToolbar} wrap>
          <Button icon={<LeftOutlined />} onClick={() => setActiveMonth(activeMonth.subtract(1, "month"))}>
            {t("common.previous")}
          </Button>
          <Typography.Title level={4}>{activeMonthLabel}</Typography.Title>
          <Button icon={<RightOutlined />} onClick={() => setActiveMonth(activeMonth.add(1, "month"))}>
            {t("common.next")}
          </Button>
          <Button onClick={() => setActiveMonth(dayjs().startOf("month"))}>
            {t("common.currentMonth")}
          </Button>
        </Space>
      </Card>

      <Tabs
        className={styles.documentTabs}
        items={[
          {
            key: "executed",
            label: t("dashboard.monthlyTransactionsTab"),
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
            label: t("dashboard.scheduledTransactionsTab"),
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
    </Space>
  );
}
