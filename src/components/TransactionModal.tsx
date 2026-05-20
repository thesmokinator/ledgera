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
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { JournalTransaction, TransactionInput, TransactionType } from "../types";
import { isValidJournalDate, journalDateFormat } from "../utils/date";
import styles from "./TransactionModal.module.css";



function PostingRow({
  field,
  accountOptions,
  commodityOptions,
  isInvestmentMode,
  onRemove,
}: {
  field: { key: number; name: number };
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  isInvestmentMode: boolean;
  onRemove: () => void;
}) {
  const { t } = useTranslation();

  return (
    <div className={`${styles.posting_row}${isInvestmentMode ? ` ${styles.posting_row_investment}` : ""}`}>
      <Form.Item label={t("transactions.account")} name={[field.name, "account"]}>
        <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
      </Form.Item>
      <Form.Item
        label={t("transactions.commodity")}
        name={[field.name, "commodity"]}
        rules={[{ required: true, message: t("transactions.commodity_required") }]}
      >
        <AutoComplete options={commodityOptions} placeholder="EUR" filterOption />
      </Form.Item>
      {isInvestmentMode ? (
        <>
          <Form.Item label={t("transactions.quantity")} name={[field.name, "amount"]}>
            <Input placeholder="10" />
          </Form.Item>
          <Form.Item label={t("transactions.unit_price")} name={[field.name, "unitPrice"]}>
            <Input placeholder="150 EUR" />
          </Form.Item>
        </>
      ) : (
        <Form.Item label={t("transactions.amount")} name={[field.name, "amount"]}>
          <Input placeholder="25.00" />
        </Form.Item>
      )}
      <Button
        danger
        className={styles.posting_delete_button}
        aria-label={t("transactions.remove_posting")}
        icon={<DeleteOutlined />}
        onClick={onRemove}
      />
      <Form.Item className={styles.posting_comment_field} name={[field.name, "comment"]}>
        <Input placeholder={t("transactions.comment_placeholder")} />
      </Form.Item>
    </div>
  );
}

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
  editingTransaction: JournalTransaction | null;
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
  const [isFormValid, setFormValid] = useState(false);

  function validateFormSilently() {
    transactionForm
      .validateFields()
      .then(() => setFormValid(true))
      .catch(() => setFormValid(false));
  }

  useEffect(() => {
    if (!open) {
      setFormValid(false);
      return;
    }

    const timer = window.setTimeout(validateFormSilently, 0);
    return () => window.clearTimeout(timer);
  }, [open, transactionForm, transactionType, defaultCommodity]);

  const isInvestmentMode =
    transactionType === "investment" ||
    (editingTransaction?.postings ?? []).some(
      (p) => p.commodity.trim() && p.commodity !== defaultCommodity
    );

  return (
    <Modal
      title={editingTransaction ? t("transactions.edit_transaction") : t("transactions.new_transaction")}
      open={open}
      width={isInvestmentMode ? 780 : 620}
      okText={editingTransaction ? t("common.save") : t("transactions.create_transaction")}
      confirmLoading={isSaving}
      okButtonProps={{ disabled: !isFormValid || isSaving }}
      onCancel={onClose}
      onOk={() => transactionForm.submit()}
    >
      <Form<TransactionInput>
        className={styles.transaction_form}
        form={transactionForm}
        layout="vertical"
        onValuesChange={validateFormSilently}
        onFinish={onSubmit}
      >
        {!editingTransaction ? (
          <Segmented<TransactionType>
            className={styles.transaction_type_selector}
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
        <Space className={styles.form_row} size="middle">
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
              { required: true, message: t("transactions.enter_transaction_date") },
              {
                validator: (_, value: string) =>
                  !value || isValidJournalDate(value)
                    ? Promise.resolve()
                    : Promise.reject(new Error(t("transactions.invalid_date"))),
              },
            ]}
          >
            <DatePicker format={journalDateFormat} className={styles.full_width_control} />
          </Form.Item>
          <Form.Item label={t("transactions.status")} name="status">
            <Select
              allowClear
              className={styles.full_width_control}
              placeholder={t("transactions.status_placeholder")}
              options={[
                { value: "*", label: t("transactions.status_cleared") },
                { value: "!", label: t("transactions.status_pending") },
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
                <PostingRow
                  key={field.key}
                  field={field}
                  accountOptions={accountOptions}
                  commodityOptions={commodityOptions}
                  isInvestmentMode={isInvestmentMode}
                  onRemove={() => remove(field.name)}
                />
              ))}
              <Button
                icon={<PlusOutlined />}
                onClick={() => {
                  add({ account: "", amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" });
                  window.setTimeout(validateFormSilently, 0);
                }}
              >
                {t("transactions.add_posting")}
              </Button>
            </Space>
          )}
        </Form.List>
      </Form>
    </Modal>
  );
}
