import { DeleteOutlined, EditOutlined, ArrowRightOutlined, SwapOutlined } from "@ant-design/icons";
import { Button, Modal, Popover, Space, Table, Typography } from "antd";
import { useTranslation } from "react-i18next";
import type { JournalTransaction } from "./types";
import styles from "./TransactionsTable.module.css";

function flowIcon(kind: string) {
  if (kind === "transfer") return <SwapOutlined className={`${styles.flowIcon} ${styles.flowIconTransfer}`} />;
  return <ArrowRightOutlined className={`${styles.flowIcon} ${styles[`flowIcon${kind.charAt(0).toUpperCase() + kind.slice(1)}`]}`} />;
}

function accountFlow(transaction: JournalTransaction) {
  const accounts = transaction.postings
    .map((p) => p.account.trim())
    .filter(Boolean);
  const unique = [...new Set(accounts)];
  if (unique.length === 0) return "—";
  if (unique.length === 1) return unique[0];

  const kind = transaction.display.kind;
  // For income, money flows from income account to asset
  // For expense, money flows from asset to expense
  if (kind === "income") {
    return (
      <span className={styles.accountFlow}>
        {unique[0]}
        {flowIcon("income")}
        {unique[unique.length - 1]}
      </span>
    );
  }
  return (
    <span className={styles.accountFlow}>
      {unique[unique.length - 1]}
      {flowIcon(kind)}
      {unique[0]}
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
      title: t("transactions.deleteTransactionAction"),
      content: t("transactions.deleteTransactionDescription"),
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
        pagination={{ pageSize: 8 }}
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
              <span className={`${styles.amount} ${styles[`amount${transaction.display.kind.charAt(0).toUpperCase() + transaction.display.kind.slice(1)}`]}`}>
                {transaction.display.formatted || transaction.display.amount}
              </span>
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
                <Button aria-label={t("transactions.editTransactionAction")} icon={<EditOutlined />} onClick={() => onEdit(transaction)} />
                <Button
                  danger
                  aria-label={t("transactions.deleteTransactionAction")}
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
