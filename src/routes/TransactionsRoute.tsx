import { LeftOutlined, RightOutlined } from "@ant-design/icons";
import { Button, Card, Space, Tabs, Typography } from "antd";
import type { Dayjs } from "dayjs";
import dayjs from "dayjs";
import { useTranslation } from "react-i18next";
import { TransactionsTable } from "./TransactionsTable";
import type { JournalTransaction } from "./types";
import { formatCount } from "../utils/format";

export function TransactionsRoute({
  monthlyTransactions,
  scheduledTransactions,
  accountsCount,
  activeMonth,
  activeMonthLabel,
  loading,
  powerUser,
  onMonthChange,
  onEditTransaction,
  onDeleteTransaction,
}: {
  monthlyTransactions: JournalTransaction[];
  scheduledTransactions: JournalTransaction[];
  accountsCount: number;
  activeMonth: Dayjs;
  activeMonthLabel: string;
  loading: boolean;
  powerUser: boolean;
  onMonthChange: (month: Dayjs) => void;
  onEditTransaction: (transaction: JournalTransaction) => void;
  onDeleteTransaction: (id: string) => void;
}) {
  const { t } = useTranslation();

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

      <Card className="settings-card month-toolbar-card">
        <Space className="month-toolbar" wrap>
          <Button icon={<LeftOutlined />} onClick={() => onMonthChange(activeMonth.subtract(1, "month"))}>
            {t("common.previous")}
          </Button>
          <Typography.Title level={4}>{activeMonthLabel}</Typography.Title>
          <Button icon={<RightOutlined />} onClick={() => onMonthChange(activeMonth.add(1, "month"))}>
            {t("common.next")}
          </Button>
          <Button onClick={() => onMonthChange(dayjs().startOf("month"))}>
            {t("common.currentMonth")}
          </Button>
        </Space>
      </Card>

      <Tabs
        className="document-tabs"
        items={[
          {
            key: "executed",
            label: t("dashboard.monthlyTransactionsTab"),
            children: (
              <TransactionsTable
                transactions={monthlyTransactions}
                loading={loading}
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
                loading={loading}
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
