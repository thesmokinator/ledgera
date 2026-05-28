import { DeleteOutlined, EditOutlined, ArrowRightOutlined, SwapOutlined } from "@ant-design/icons";
import { Button, Modal, Popover, Space, Table, Typography } from "antd";
import { useTranslation } from "react-i18next";
import type { JournalTransaction } from "./types";
import { Amount, classSuffix } from "../components/Amount";
import type { TransactionDisplay } from "./types";
import styles from "./TransactionsTable.module.css";

const flowIconClass: Record<string, string | undefined> = {
  expense: styles.flowIconExpense,
  income: styles.flowIconIncome,
  transfer: styles.flow_icon_transfer,
  investment: styles.flowIconInvestment,
};

function flowIcon(kind: TransactionDisplay["kind"]) {
  if (kind === "transfer") return <SwapOutlined className={`${styles.flow_icon} ${styles.flow_icon_transfer}`} />;
  return <ArrowRightOutlined className={`${styles.flow_icon} ${flowIconClass[kind] ?? styles[`flowIcon${classSuffix(kind)}`]}`} />;
}

function AccountList({ accounts }: { accounts: string[] }) {
  return (
    <span className={styles.account_flow_side}>
      {accounts.map((account) => (
        <span key={account}>{account}</span>
      ))}
    </span>
  );
}

export function accountFlow(transaction: JournalTransaction) {
  const { from, to } = transaction.display.flow;
  if (from.length === 0 && to.length === 0) return transaction.display.account || "-";
  if (from.length === 0) return <AccountList accounts={to} />;
  if (to.length === 0) return <AccountList accounts={from} />;

  return (
    <span className={styles.account_flow}>
      <AccountList accounts={from} />
      {flowIcon(transaction.display.kind)}
      <AccountList accounts={to} />
    </span>
  );
}

export function TransactionsTable({
  transactions,
  loading,
  powerUser,
  onEdit,
  onDelete,
}: {
  transactions: JournalTransaction[];
  loading: boolean;
  powerUser: boolean;
  onEdit: (transaction: JournalTransaction) => void;
  onDelete: (id: string) => void;
}) {
  const { t } = useTranslation();
  const [modal, modalContextHolder] = Modal.useModal();

  function confirmDelete(transaction: JournalTransaction) {
    modal.confirm({
      title: t("transactions.delete_transaction_action"),
      content: t("transactions.delete_transaction_description"),
      okText: t("transactions.delete"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      centered: true,
      onOk: () => onDelete(transaction.id),
    });
  }

  return (
    <>
      {modalContextHolder}
      <Table<JournalTransaction>
        rowKey="id"
        loading={loading}
        dataSource={transactions}
        pagination={{ pageSize: 12 }}
        scroll={{ x: 900 }}
        expandable={
          powerUser
            ? {
              expandedRowRender: (transaction) => (
                <pre className={styles.raw}>{transaction.raw}</pre>
              ),
            }
            : undefined
        }
        columns={[
          { title: t("transactions.date"), dataIndex: "date", width: 120 },
          { title: t("transactions.status"), dataIndex: "status", width: 76, render: (status: string) => status || "-" },
          {
            title: t("transactions.description"),
            dataIndex: "description",
            width: 200,
            ellipsis: true,
            render: (desc: string) => (
              <Popover content={desc} trigger="hover" placement="topLeft">
                <Typography.Text ellipsis style={{ maxWidth: 180 }}>
                  {desc}
                </Typography.Text>
              </Popover>
            ),
          },
          {
            title: t("transactions.account"),
            width: 320,
            render: (_, transaction) => accountFlow(transaction),
          },
          {
            title: t("transactions.amount"),
            width: 160,
            align: "right",
            render: (_, transaction) => (
              <Amount
                formatted={transaction.display.formatted || transaction.display.amount}
                tint={transaction.display.tint}
                kind={transaction.display.kind}
                className={styles.amount}
              />
            ),
          },
          ...(powerUser
            ? [
              {
                title: t("transactions.lines"),
                width: 120,
                render: (_: unknown, transaction: JournalTransaction) =>
                  `${transaction.startLine}-${transaction.endLine}`,
              },
            ]
            : []),
          {
            title: t("transactions.actions"),
            width: 136,
            render: (_, transaction) => (
              <Space>
                <Button aria-label={t("transactions.edit_transaction_action")} icon={<EditOutlined />} onClick={() => onEdit(transaction)} />
                <Button
                  danger
                  aria-label={t("transactions.delete_transaction_action")}
                  icon={<DeleteOutlined />}
                  onClick={() => confirmDelete(transaction)}
                />
              </Space>
            ),
          },
        ]}
      />
    </>
  );
}
