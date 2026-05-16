import { DeleteOutlined, EditOutlined } from "@ant-design/icons";
import { Button, Modal, Space, Table } from "antd";
import { useTranslation } from "react-i18next";
import type { JournalTransaction } from "./types";

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
                <pre className="transaction-raw">{transaction.raw}</pre>
              ),
            }
            : undefined
        }
        columns={[
          { title: t("transactions.date"), dataIndex: "date", width: 132 },
          { title: t("transactions.status"), dataIndex: "status", width: 88, render: (status: string) => status || "-" },
          { title: t("transactions.description"), dataIndex: "description" },
          {
            title: t("transactions.account"),
            width: 260,
            render: (_, transaction) => transaction.display.account,
          },
          {
            title: t("transactions.amount"),
            width: 160,
            align: "right",
            render: (_, transaction) => (
              <span className={`transaction-amount transaction-amount-${transaction.display.kind}`}>
                {transaction.display.amount}
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
