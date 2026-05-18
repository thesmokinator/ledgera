import { Card, Select, Space, Table, Typography } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { TransactionsTable } from "./TransactionsTable";
import type { AccountActivityRange, AccountSummary, JournalSummary, JournalTransaction } from "./types";
import { formatCount } from "../utils/format";
import { groupAccounts, collectAccounts } from "../utils/account";
import { isInAccountActivityRange } from "../utils/date";
import styles from "./AccountsRoute.module.css";

const accountActivityRangeOptions: AccountActivityRange[] = [
  "current-month", "30", "60", "90", "180", "365",
];

export function AccountsRoute({
  powerUser,
  onEditTransaction,
  onDeleteTransaction,
}: {
  powerUser: boolean;
  onEditTransaction: (transaction: JournalTransaction) => void;
  onDeleteTransaction: (id: string) => void;
}) {
  const { t } = useTranslation();
  const [activityRange, setActivityRange] = useState<AccountActivityRange>("current-month");

  const transactionsQuery = useQuery({
    queryKey: ["transactions"],
    queryFn: () => invoke<JournalSummary>("list_transactions"),
    retry: false,
    refetchOnMount: true,
  });

  const transactions = transactionsQuery.data?.transactions ?? [];
  const visible = transactions.filter((tx) => isInAccountActivityRange(tx, activityRange));
  const accounts = collectAccounts(transactions, visible);
  const grouped = groupAccounts(accounts);

  return (
    <Space direction="vertical" size={24} className="content-stack">
      <Card
        className={styles.card}
        title={t("accounts.title")}
        extra={(
          <Space className={styles.rangeControl}>
            <Select<AccountActivityRange>
              value={activityRange}
              onChange={setActivityRange}
              options={accountActivityRangeOptions.map((range) => ({
                value: range,
                label: range === "current-month"
                  ? t("accounts.currentMonth")
                  : t("accounts.lastDays", { count: Number(range) }),
              }))}
            />
          </Space>
        )}
      >
        {grouped.map(({ group, items }) => (
          <div key={group} className={styles.group}>
            <Typography.Title level={5} className={styles.groupTitle}>
              {t(`accounts.groups.${group}`)}
              <Typography.Text type="secondary" className={styles.groupCount}>
                {formatCount(items.length)}
              </Typography.Text>
            </Typography.Title>
            <Table<AccountSummary>
              rowKey="account"
              loading={transactionsQuery.isFetching}
              dataSource={items}
              pagination={false}
              showHeader={false}
              scroll={{ x: 400 }}
              expandable={{
                expandedRowRender: (account) => (
                  <div className={styles.transactionsPanel}>
                    <Typography.Text className={styles.transactionsTitle}>
                      {t("accounts.accountActivity", {
                        account: account.account,
                        count: account.transactions,
                      })}
                    </Typography.Text>
                    <TransactionsTable
                      transactions={account.accountTransactions}
                      loading={transactionsQuery.isFetching}
                      powerUser={powerUser}
                      onEdit={onEditTransaction}
                      onDelete={onDeleteTransaction}
                    />
                  </div>
                ),
                rowExpandable: (account) => account.transactions > 0,
              }}
              columns={[
                { title: t("transactions.account"), dataIndex: "account" },
                {
                  title: t("accounts.transactionsCount"),
                  dataIndex: "transactions",
                  width: 120,
                  align: "right",
                  render: (count: number) => formatCount(count),
                },
              ]}
            />
          </div>
        ))}
      </Card>
    </Space>
  );
}
