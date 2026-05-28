import { Card, Empty, Input, Select, Space, Table, Typography } from "antd";
import { useQuery } from "@tanstack/react-query";
import { callCommand } from "../utils/command";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { TransactionsTable } from "./TransactionsTable";
import { Amount } from "../components/Amount";
import type { AccountActivityRange, AccountOverviewGroup, AccountOverviewRow, AccountsOverview, Balance, JournalTransaction } from "./types";
import { formatCount } from "../utils/format";
import styles from "./AccountsRoute.module.css";

const accountActivityRangeOptions: AccountActivityRange[] = [
  "current-month", "30", "60", "90", "180", "365",
];

function renderBalance(balance: Balance | null) {
  if (!balance) return "-";
  return (
    <Amount
      formatted={balance.formatted || "-"}
      tint={balance.tint}
    />
  );
}

function filterGroups(groups: AccountOverviewGroup[], search: string): AccountOverviewGroup[] {
  const query = search.trim().toLowerCase();
  if (!query) return groups;

  return groups
    .map((group) => ({
      ...group,
      accounts: group.accounts.filter((account) =>
        account.account.toLowerCase().includes(query) ||
        account.balance?.commodity.toLowerCase().includes(query),
      ),
    }))
    .filter((group) => group.accounts.length > 0);
}

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
  const [search, setSearch] = useState("");

  const accountsQuery = useQuery({
    queryKey: ["accounts-overview", activityRange],
    queryFn: () => callCommand<AccountsOverview>("get_accounts_overview", { activityRange }),
    retry: false,
    refetchOnMount: true,
  });

  const grouped = useMemo(
    () => filterGroups(accountsQuery.data?.groups ?? [], search),
    [accountsQuery.data?.groups, search],
  );
  const isInitialLoading = accountsQuery.isLoading && !accountsQuery.data;
  const columns = [
    { title: t("transactions.account"), dataIndex: "account" },
    {
      title: t("accounts.transactions_count"),
      dataIndex: "activityCount",
      width: 160,
      align: "right" as const,
      render: (count: number) => formatCount(count),
    },
    {
      title: t("accounts.current_balance"),
      width: 220,
      align: "right" as const,
      render: (_: unknown, account: AccountOverviewRow) => renderBalance(account.balance),
    },
  ];

  return (
    <Space orientation="vertical" size={24} className="content-stack">
      <Card
        className={styles.card}
        title={t("accounts.title")}
        extra={(
          <Space className={styles.range_control} wrap>
            <Input.Search
              allowClear
              value={search}
              placeholder={t("accounts.search_placeholder")}
              onChange={(event) => setSearch(event.target.value)}
              className={styles.search_input}
            />
            <Select<AccountActivityRange>
              value={activityRange}
              onChange={setActivityRange}
              options={accountActivityRangeOptions.map((range) => ({
                value: range,
                label: range === "current-month"
                  ? t("accounts.current_month")
                  : t("accounts.last_days", { count: Number(range) }),
              }))}
            />
          </Space>
        )}
      >
        {isInitialLoading ? (
          <Table<AccountOverviewRow>
            rowKey="account"
            loading
            dataSource={[]}
            pagination={false}
            scroll={{ x: 680 }}
            columns={columns}
          />
        ) : grouped.length === 0 ? (
          <Empty description={t(search ? "accounts.noSearchResults" : "accounts.empty")} />
        ) : grouped.map(({ group, accounts }) => (
          <div key={group} className={styles.group}>
            <Typography.Title level={5} className={styles.group_title}>
              {t(`accounts.groups.${group}`)}
              <Typography.Text type="secondary" className={styles.group_count}>
                {formatCount(accounts.length)}
              </Typography.Text>
            </Typography.Title>
            <Table<AccountOverviewRow>
              rowKey="account"
              loading={accountsQuery.isFetching}
              dataSource={accounts}
              pagination={false}
              scroll={{ x: 680 }}
              expandable={{
                expandedRowRender: (account) => (
                  <div className={styles.transactions_panel}>
                    <Typography.Text className={styles.transactions_title}>
                      {t("accounts.account_activity", {
                        account: account.account,
                        count: account.activityCount,
                      })}
                    </Typography.Text>
                    <TransactionsTable
                      transactions={account.transactions}
                      loading={accountsQuery.isFetching}
                      powerUser={powerUser}
                      onEdit={onEditTransaction}
                      onDelete={onDeleteTransaction}
                    />
                  </div>
                ),
                rowExpandable: (account) => account.activityCount > 0,
              }}
              columns={columns}
            />
          </div>
        ))}
      </Card>
    </Space>
  );
}
