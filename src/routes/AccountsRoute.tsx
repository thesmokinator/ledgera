import { Card, Select, Space, Table, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { TransactionsTable } from "./TransactionsTable";
import type { AccountActivityRange, AccountSummary, JournalTransaction } from "./types";
import { formatCount } from "../utils/format";
import styles from "./AccountsRoute.module.css";

const groupOrder = ["assets", "liabilities", "equity", "income", "expenses"];

function groupAccounts(accounts: AccountSummary[]): { group: string; items: AccountSummary[] }[] {
  const map = new Map<string, AccountSummary[]>();
  for (const a of accounts) {
    const root = a.account.split(":")[0].toLowerCase();
    const key = groupOrder.includes(root) ? root : "other";
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(a);
  }
  const result: { group: string; items: AccountSummary[] }[] = [];
  for (const g of groupOrder) {
    const items = map.get(g);
    if (items) result.push({ group: g, items });
  }
  const other = map.get("other");
  if (other) result.push({ group: "other", items: other });
  return result;
}

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
  const grouped = groupAccounts(accounts);

  return (
    <Space direction="vertical" size={24} className="content-stack">
      <Card
        className={styles.card}
        title={t("accounts.title")}
        extra={(
          <Space className={styles.rangeControl}>
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
              loading={loading}
              dataSource={items}
              pagination={false}
              showHeader={false}
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
