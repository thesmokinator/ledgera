import {
  AutoComplete,
  Button,
  DatePicker,
  Form,
  Input,
  Modal,
  Segmented,
  Select,
  Space,
} from "antd";
import {
  DeleteOutlined,
  PlusOutlined,
} from "@ant-design/icons";
import type { FormInstance } from "antd";
import dayjs from "dayjs";
import { useTranslation } from "react-i18next";
import type { TransactionInput, TransactionType } from "../types";
import { isValidJournalDate, journalDateFormat } from "../utils/date";

export function TransactionModal({
  open,
  editingTransaction,
  transactionForm,
  isSaving,
  transactionType,
  codeOptions,
  descriptionOptions,
  accountOptions,
  commodityOptions,
  defaultCommodity,
  onClose,
  onSubmit,
  onTransactionTypeChange,
}: {
  open: boolean;
  editingTransaction: unknown;
  transactionForm: FormInstance<TransactionInput>;
  isSaving: boolean;
  transactionType: TransactionType;
  codeOptions: { value: string }[];
  descriptionOptions: { value: string }[];
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  defaultCommodity: string;
  onClose: () => void;
  onSubmit: (values: TransactionInput) => void;
  onTransactionTypeChange: (type: TransactionType) => void;
}) {
  const { t } = useTranslation();

  return (
    <Modal
      title={editingTransaction ? t("transactions.editTransaction") : t("transactions.newTransaction")}
      open={open}
      okText={editingTransaction ? t("common.save") : t("transactions.createTransaction")}
      confirmLoading={isSaving}
      onCancel={onClose}
      onOk={() => transactionForm.submit()}
    >
      <Form<TransactionInput>
        form={transactionForm}
        layout="vertical"
        onFinish={onSubmit}
      >
        {!editingTransaction ? (
          <Segmented<TransactionType>
            className="transaction-type-selector"
            block
            value={transactionType}
            onChange={onTransactionTypeChange}
            options={[
              { value: "expense", label: t("transactions.types.expense") },
              { value: "income", label: t("transactions.types.income") },
              { value: "transfer", label: t("transactions.types.transfer") },
              { value: "investment", label: t("transactions.types.investment") },
              { value: "custom", label: t("transactions.types.custom") },
            ]}
          />
        ) : null}
        <Space className="form-row" size="middle">
          <Form.Item
            label={t("transactions.date")}
            name="date"
            getValueProps={(value?: string) => ({
              value: value ? dayjs(value, journalDateFormat) : null,
            })}
            normalize={(value: dayjs.Dayjs | null) =>
              value ? value.format(journalDateFormat) : ""
            }
            rules={[
              { required: true, message: t("transactions.enterTransactionDate") },
              {
                validator: (_, value: string) =>
                  !value || isValidJournalDate(value)
                    ? Promise.resolve()
                    : Promise.reject(new Error(t("transactions.invalidDate"))),
              },
            ]}
          >
            <DatePicker format={journalDateFormat} className="full-width-control" />
          </Form.Item>
          <Form.Item label={t("transactions.status")} name="status">
            <Select
              allowClear
              className="full-width-control"
              placeholder={t("transactions.statusPlaceholder")}
              options={[
                { value: "*", label: t("transactions.statusCleared") },
                { value: "!", label: t("transactions.statusPending") },
              ]}
            />
          </Form.Item>
          <Form.Item label={t("transactions.code")} name="code">
            <AutoComplete options={codeOptions} placeholder="(INV-001)" filterOption />
          </Form.Item>
        </Space>
        <Form.Item label={t("transactions.description")} name="description">
          <AutoComplete options={descriptionOptions} filterOption />
        </Form.Item>
        <Form.List name="postings">
          {(fields, { add, remove }) => (
            <Space direction="vertical" className="content-stack">
              {fields.map((field) => (
                <div key={field.key} className="posting-row">
                  <Form.Item label={t("transactions.account")} name={[field.name, "account"]}>
                    <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
                  </Form.Item>
                  <Form.Item label={t("transactions.commodity")} name={[field.name, "commodity"]}>
                    <AutoComplete options={commodityOptions} placeholder="EUR" filterOption />
                  </Form.Item>
                  <Form.Item label={t("transactions.amount")} name={[field.name, "amount"]}>
                    <Input placeholder="25.00" />
                  </Form.Item>
                  <Button
                    danger
                    className="posting-delete-button"
                    aria-label={t("transactions.removePosting")}
                    icon={<DeleteOutlined />}
                    onClick={() => remove(field.name)}
                  />
                  <Form.Item className="posting-comment-field" name={[field.name, "comment"]}>
                    <Input placeholder={t("transactions.commentPlaceholder")} />
                  </Form.Item>
                </div>
              ))}
              <Button icon={<PlusOutlined />} onClick={() => add({ account: "", amount: "", commodity: defaultCommodity, comment: "" })}>
                {t("transactions.addPosting")}
              </Button>
            </Space>
          )}
        </Form.List>
      </Form>
    </Modal>
  );
}
