import { Card, Select, Space, Table, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { TransactionsTable } from "./TransactionsTable";
import type { AccountActivityRange, AccountSummary, JournalTransaction } from "./types";

export function AccountsRoute({
  accounts,
  accountActivityRange,
  accountActivityRangeOptions,
  loading,
  powerUser,
  onActivityRangeChange,
  onEditTransaction,
  onDeleteTransaction,
}: {
  accounts: AccountSummary[];
  accountActivityRange: AccountActivityRange;
  accountActivityRangeOptions: AccountActivityRange[];
  loading: boolean;
  powerUser: boolean;
  onActivityRangeChange: (range: AccountActivityRange) => void;
  onEditTransaction: (transaction: JournalTransaction) => void;
  onDeleteTransaction: (id: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <Space direction="vertical" size={24} className="content-stack">
      <Card
        className="settings-card accounts-card"
        title={t("accounts.allAccounts")}
        extra={(
          <Space className="accounts-range-control">
            <Typography.Text>{t("accounts.activityRange")}</Typography.Text>
            <Select<AccountActivityRange>
              value={accountActivityRange}
              onChange={onActivityRangeChange}
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
        <Table<AccountSummary>
          rowKey="account"
          loading={loading}
          dataSource={accounts}
          pagination={{ pageSize: 12 }}
          expandable={{
            expandedRowRender: (account) => (
              <div className="account-transactions-panel">
                <Typography.Text className="account-transactions-title">
                  {t("accounts.accountActivity", {
                    account: account.account,
                    count: account.transactions,
                  })}
                </Typography.Text>
                <TransactionsTable
                  transactions={account.accountTransactions}
                  loading={loading}
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
              width: 180,
              align: "right",
              render: (count: number) => new Intl.NumberFormat("en", { maximumFractionDigits: 0 }).format(count),
            },
          ]}
        />
      </Card>
    </Space>
  );
}
